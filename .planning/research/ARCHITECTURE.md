# Architecture Patterns

**Domain:** Ollama OCR integration in Tauri 2 desktop app
**Researched:** 2026-03-20

---

## Recommended Architecture

Replace the three `#[cfg(target_os)]` OCR branches in `platform.rs` with a single
async HTTP call to Ollama. The call site in `commit_selection` (in `lib.rs`) becomes
`async`. State management stays on `std::sync::Mutex`; only the OCR step touches
async.

```
Frontend (JS)
    │  invoke("commit_selection", {snapshotId, selection})
    ▼
lib.rs  commit_selection  [async #[tauri::command]]
    │  1. lock SharedState (std::sync::Mutex — sync lock, fast)
    │  2. clone snapshot bytes + validate
    │  3. release lock
    │  4. crop_png()  [sync, stays in platform.rs]
    │  5. ollama::recognize_text()  [async HTTP — NEW]
    │  6. re-acquire lock
    │  7. push_segment() → rebuild_merge()
    │  8. release lock, return AppStatePayload
    ▼
platform.rs  crop_png()  [unchanged]

ollama.rs  (NEW MODULE)
    │  base64-encode PNG bytes
    │  POST http://192.168.1.12:11434/api/generate
    │     { model, prompt, images: [<base64>], stream: false }
    │  parse JSON → extract .response field
    │  sanitize_ocr_output()  [moved from platform.rs]
    ▼
Ollama at 192.168.1.12
```

---

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `lib.rs` (command layer) | IPC handler; owns the async boundary; orchestrates crop + OCR + merge | `platform.rs`, `ollama.rs`, `state.rs` |
| `platform.rs` (platform layer) | Screenshot capture, PNG crop/decode only — no OCR after migration | `lib.rs` |
| `ollama.rs` (new HTTP layer) | Base64-encode PNG, POST to Ollama, parse response, sanitize text | `lib.rs` |
| `state.rs` (state layer) | Session state machine, merge orchestration | `lib.rs`, `merge.rs` |
| `merge.rs` (algorithm layer) | Pure fuzzy deduplication — untouched | `state.rs` |

### What changes hands

- `recognize_text_from_png()` moves from `platform.rs` to `ollama.rs`
- `sanitize_ocr_output()` moves from `platform.rs` to `ollama.rs` (or stays as a shared utility; either is fine)
- `platform.rs` loses all `recognize_text_from_file()` implementations and the
  `#[cfg(target_os)]` dispatch
- `src-tauri/scripts/vision_ocr.swift` is deleted
- `VISION_OCR_SCRIPT` constant and `include_str!()` in `platform.rs` are removed

---

## Data Flow

### Image encoding

Ollama's `/api/generate` REST endpoint expects images as **raw base64 strings** (no
`data:image/png;base64,` URI prefix) in a JSON array field called `images`.
(MEDIUM confidence — confirmed via Ollama GitHub issue #68 in ollama-js and API docs.)

The PNG bytes already live in memory as `Vec<u8>` after `crop_png()` returns. Encode
them with the `base64` crate (already in `Cargo.toml`):

```rust
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

let b64 = STANDARD.encode(&crop_bytes);
```

No temp file write is needed. This avoids the current pattern of writing a file, passing
its path to the OCR subprocess, then deleting it.

### HTTP request shape

```json
{
  "model": "glm-ocr",
  "prompt": "Extract all text from the image exactly as it appears.",
  "images": ["<raw-base64-encoded-PNG>"],
  "stream": false
}
```

Endpoint: `POST http://192.168.1.12:11434/api/generate`
Headers: `Content-Type: application/json`

Response JSON contains a `.response` string field with the extracted text.

**Note on model name:** `glm-ocr` is the Ollama library name confirmed at
`ollama.com/library/glm-ocr`. The running instance may use `glm-ocr:latest` or a
quantized tag like `glm-ocr:q8_0`. Hardcode `"glm-ocr"` initially; Ollama resolves
`:latest` automatically. (MEDIUM confidence — verified from Ollama library page.)

### Async boundary

The current `commit_selection` command is sync. It becomes `async fn` because the HTTP
call must be awaited. Tauri 2 supports `async` command handlers natively — they are
dispatched on Tauri's `tokio`-backed async runtime.

**Critical constraint:** `std::sync::Mutex` guards cannot be held across `.await`
points. The solution is to lock, clone what is needed, drop the lock, do async work,
then re-acquire the lock:

```rust
#[tauri::command]
async fn commit_selection(
    request: CommitSelectionRequest,
    state: State<'_, SharedState>,
) -> Result<AppStatePayload, String> {
    // 1. Sync lock — clone data, then drop guard before any .await
    let snapshot = {
        let guard = state.inner.lock()
            .map_err(|_| "State lock was poisoned.".to_string())?;
        guard.current_snapshot.clone()
            .ok_or_else(|| "Capture a snapshot before committing.".to_string())?
    };  // guard dropped here

    if snapshot.id != request.snapshot_id {
        return Err("Snapshot changed before selection was committed.".into());
    }

    // 2. Sync crop (no lock needed)
    let crop = crop_png(&snapshot.png_bytes, ...)?;

    // 3. Async HTTP call — no lock held
    let recognized_text = ollama::recognize_text(&crop).await?;

    if recognized_text.trim().is_empty() {
        return Err("OCR returned no text.".into());
    }

    // 4. Re-acquire lock to mutate state
    let mut guard = state.inner.lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;
    guard.push_segment(snapshot.id, request.selection, recognized_text);
    Ok(guard.to_payload())
}
```

This pattern is confirmed correct for Tauri 2 async commands with
`std::sync::Mutex`-wrapped state. (HIGH confidence — official Tauri docs and community
discussions.)

**Do not switch to `tokio::sync::Mutex` for `SharedState`.** The existing `std::sync::Mutex` is
correct as long as no lock guard crosses an await point. The clone-before-await
pattern above satisfies this without changing the state module.

---

## New Module: `ollama.rs`

Create `src-tauri/src/ollama.rs`. Export one public function:

```rust
pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String>
```

Internally:
1. Base64-encode `png_bytes`
2. Build JSON body with `serde_json::json!`
3. POST with a `reqwest::Client` (configured with a 60-second timeout)
4. Deserialize response body
5. Extract `.response` field
6. Run `sanitize_ocr_output()` on the text
7. Return

Error mapping:
- Connection refused / network error → `"Ollama is unreachable at 192.168.1.12. Start the Ollama service and ensure glm-ocr is loaded."`
- Timeout → `"Ollama OCR timed out after 60 seconds. The model may still be loading."`
- Non-2xx HTTP → `"Ollama returned error {status}: {body}"`
- Missing `.response` in JSON → `"Ollama response did not contain text output."`

---

## Cargo.toml Changes

Add two dependencies:

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde_json = "1"
```

Notes:
- `default-features = false` drops OpenSSL linkage; `rustls-tls` gives pure-Rust TLS
  (unnecessary for plain HTTP to a LAN address, but consistent with Tauri's own
  reqwest usage pattern and avoids OpenSSL build issues on Linux)
- `features = ["json"]` enables `.json()` serialization/deserialization on responses
- `serde_json` is needed for `serde_json::json!` macro to build the request body;
  `serde` is already present
- `reqwest` `0.12` is the current stable line as of early 2026 (MEDIUM confidence —
  based on known 0.12 release in 2024 and no evidence of 0.13)
- The `base64` crate is already present in `Cargo.toml` — no addition needed

---

## Files to Delete

| File | Reason |
|------|--------|
| `src-tauri/scripts/vision_ocr.swift` | Apple Vision OCR replaced by Ollama |
| `VISION_OCR_SCRIPT` constant in `platform.rs` | `include_str!` of deleted script |

---

## Patterns to Follow

### Pattern 1: Clone Before Await
**What:** Extract needed data from `std::sync::Mutex` guard into owned values, drop the
guard, then call async functions.
**When:** Any async Tauri command that also reads/writes `SharedState`.
**Why:** `std::sync::MutexGuard` is not `Send`; it cannot be held across an `.await`
point without compiler error.

### Pattern 2: Single Responsibility in `ollama.rs`
**What:** `ollama.rs` only handles HTTP transport and response parsing. It does not
know about `AppState`, segments, or merge logic.
**When:** Adding any Ollama capability beyond OCR in future.
**Why:** Keeps the HTTP boundary narrow and independently testable.

### Pattern 3: Timeout on Every HTTP Call
**What:** Configure `reqwest::Client` with `.timeout(Duration::from_secs(60))`.
**When:** Any network call to Ollama.
**Why:** GLM-OCR can be slow on first use while the model loads into VRAM. Without a
timeout, `commit_selection` hangs indefinitely on an unreachable or sluggish server.

### Pattern 4: Hard Fail with Actionable Message
**What:** Map every network error to a specific user-facing string that tells the user
what to check (service running? model loaded?).
**When:** All error variants from reqwest.
**Why:** The project spec requires hard fail with clear error message; the frontend
already displays `flash(error, true)` for all backend errors.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Blocking HTTP in Sync Command
**What:** Using `reqwest::blocking::Client` inside a sync `#[tauri::command]`.
**Why bad:** Blocks Tauri's async runtime thread. `reqwest::blocking` panics if called
inside a Tokio context. Tauri's command handlers run inside the Tokio runtime even
when declared sync.
**Instead:** Declare `commit_selection` as `async fn` and use the async
`reqwest::Client`.

### Anti-Pattern 2: Holding Mutex Across Await
**What:** Acquiring `state.inner.lock()`, keeping the `MutexGuard` in scope, then
calling `.await`.
**Why bad:** `MutexGuard<AppState>` is not `Send`. The compiler will refuse to compile
the async function, or if `unsafe` is used to force it, deadlocks become possible.
**Instead:** Clone the data out of the guard before any `.await` (Pattern 1 above).

### Anti-Pattern 3: Switching to `tokio::sync::Mutex` for SharedState
**What:** Replacing `std::sync::Mutex` in `state.rs` with `tokio::sync::Mutex` to
avoid the clone-before-await constraint.
**Why bad:** Over-engineering. The current `std::sync::Mutex` is correct and idiomatic
for in-process state. The OCR call does not need to hold state during the await.
Changing the mutex type cascades changes through `state.rs` and all six command
handlers.
**Instead:** Clone needed data before awaiting (Pattern 1).

### Anti-Pattern 4: Passing Data URL to Ollama
**What:** Sending the base64 PNG as `data:image/png;base64,<data>` (with the URI
prefix).
**Why bad:** Ollama's `/api/generate` endpoint expects raw base64 strings in the
`images` array. The URI prefix causes the image to be rejected or misinterpreted.
(MEDIUM confidence — confirmed by ollama-js issue #68 and Ollama API docs.)
**Instead:** Encode with `STANDARD.encode()` and pass the raw string.

---

## Suggested Build Order

Dependencies between components determine this order. Each step produces a
compilable, testable increment.

**Step 1: Add dependencies to `Cargo.toml`**
- Add `reqwest` and `serde_json`
- Verify `cargo build` compiles with no errors before touching logic

**Step 2: Create `ollama.rs` with `recognize_text()`**
- Pure async function with no Tauri or state dependencies
- Unit-testable in isolation (can be tested with a real Ollama instance via `cargo test`)
- Depends on: `base64` (existing), `reqwest` (step 1), `serde_json` (step 1)

**Step 3: Replace `recognize_text_from_png()` in `platform.rs`**
- Remove `recognize_text_from_file()` implementations (all three `#[cfg]` variants)
- Remove `recognize_text_from_png()` wrapper
- Remove `VISION_OCR_SCRIPT` constant and `include_str!`
- Move `sanitize_ocr_output()` to `ollama.rs` or keep it in `platform.rs` as a utility
- Delete `src-tauri/scripts/vision_ocr.swift`

**Step 4: Update `lib.rs` — make `commit_selection` async**
- Change `fn commit_selection` to `async fn commit_selection`
- Apply clone-before-await pattern for `SharedState`
- Replace `recognize_text_from_png(&crop)?` with `ollama::recognize_text(&crop).await?`
- Add `mod ollama;` at top of `lib.rs`

**Step 5: Smoke test end-to-end**
- Run `cargo tauri dev`
- Capture a snapshot, draw a marquee, commit selection
- Verify text appears from Ollama
- Verify hard-fail error message when Ollama is stopped

Build order rationale:
- Step 1 before 2 because `ollama.rs` won't compile without `reqwest`
- Step 2 before 3/4 because the new function must exist before it can be called
- Step 3 before 4 because `lib.rs` imports from `platform.rs`; removing the old
  import before adding the new one keeps the diff atomic

---

## Scalability Considerations

This is a single-user desktop app; scalability is not a concern. The relevant
operational considerations are:

| Concern | Now | If Ollama endpoint changes |
|---------|-----|--------------------------|
| Endpoint address | Hardcoded `192.168.1.12:11434` in `ollama.rs` | Change one constant |
| Model name | Hardcoded `"glm-ocr"` in `ollama.rs` | Change one constant |
| Timeout | 60 seconds in `reqwest::Client` config | Change one constant |
| Multiple OCR requests | Sequential (one per commit_selection) | No contention; `reqwest::Client` is cheaply cloneable if needed |

---

## Sources

- [Tauri 2 — Calling Rust from the Frontend (async commands)](https://v2.tauri.app/develop/calling-rust/) — HIGH confidence
- [Tauri 2 — State Management](https://v2.tauri.app/develop/state-management/) — HIGH confidence
- [Ollama GitHub — docs/api.md](https://github.com/ollama/ollama/blob/main/docs/api.md) — HIGH confidence
- [Ollama — Vision capabilities](https://docs.ollama.com/capabilities/vision) — HIGH confidence
- [Ollama library — glm-ocr](https://ollama.com/library/glm-ocr) — MEDIUM confidence
- [ollama-js issue #68 — base64 prefix handling](https://github.com/ollama/ollama-js/issues/68) — MEDIUM confidence
- [reqwest::blocking docs — cannot run inside async runtime](https://docs.rs/reqwest/latest/reqwest/blocking/index.html) — HIGH confidence
- [tauri::async_runtime](https://docs.rs/tauri/latest/tauri/async_runtime/index.html) — HIGH confidence
- [Tauri discussion #6820 — awaiting async in async command](https://github.com/tauri-apps/tauri/discussions/6820) — MEDIUM confidence
- [Tauri discussion #6531 — async State management pattern](https://github.com/tauri-apps/tauri/discussions/6531) — MEDIUM confidence
