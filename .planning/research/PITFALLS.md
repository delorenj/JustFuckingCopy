# Domain Pitfalls: Ollama GLM-OCR Integration in Tauri 2

**Domain:** Network OCR via Ollama vision model in a Rust/Tauri desktop app
**Researched:** 2026-03-20
**Confidence:** HIGH for Tauri/reqwest patterns (official docs + multiple issues), HIGH for GLM-OCR specifics (active GitHub issues with reproduction steps)

---

## Critical Pitfalls

Mistakes that cause incorrect output, panics, or rewrites.

---

### Pitfall 1: Including the Data URL Prefix in the Base64 Payload

**What goes wrong:** The current codebase stores screenshots as base64-encoded data URLs (`data:image/png;base64,...`) in `AppState`. The Ollama `/api/generate` and `/api/chat` endpoints expect the `images` array to contain raw base64 strings — no scheme, no mime-type prefix, no comma. Passing the full data URL silently produces garbled or empty OCR output; Ollama does not return a format error.

**Why it happens:** The frontend needs a data URL for the `<img>` src attribute, so the backend encodes as a data URL. When the same string gets forwarded to Ollama, the prefix rides along undetected because the HTTP request still succeeds with a 200.

**Consequences:** OCR returns empty string or garbage. Because there is no error code to catch, the merge algorithm receives empty input and either appends nothing or corrupts the merged text silently.

**Prevention:** Strip the prefix before constructing the Ollama request body. In Rust:
```rust
let raw_b64 = data_url
    .strip_prefix("data:image/png;base64,")
    .unwrap_or(data_url);
```
Alternatively, keep two separate representations in `AppState`: one data URL for the frontend, one raw base64 for OCR. The `state.rs` `Snapshot` struct currently stores only `data_url`; add a `png_bytes` field or derive raw base64 on demand.

**Detection:** Unit-test the OCR HTTP body construction with a known small PNG; assert that `images[0]` does not start with `data:`.

**Phase:** Implement in the first (and only) implementation phase, before any integration testing.

---

### Pitfall 2: GLM-OCR Default Context Window Truncates Output

**What goes wrong:** Ollama defaults to `num_ctx = 4096` tokens for all models. For GLM-OCR, this is insufficient for images larger than roughly 800x600 at standard DPI. The model silently stops mid-extraction with `"done": true` before finishing. The Ollama issue tracker documents this as "prediction aborted, token repeat limit reached" and a separate issue "failed to fully read image, stopped with `done: false`".

**Why it happens:** GLM-OCR uses a visual token encoder that maps image pixels to tokens at high density. A 1920x1080 cropped region can produce several thousand image tokens before the language decoder even begins emitting text. The 4096-token budget is exhausted before text generation completes.

**Consequences:** OCR output is truncated mid-line. The merge algorithm receives partial text, which may or may not deduplicate correctly depending on where the cut falls. No error is raised — the response body is valid JSON with `"done": true`.

**Prevention:** Deploy the model with `num_ctx` set to at least 10240. The canonical fix is a custom Modelfile on the Ollama server:
```
FROM glm-ocr
PARAMETER num_ctx 10240
```
The Rust client can also request a higher context per-call by including `"options": {"num_ctx": 10240}` in the generate request body, which overrides the server default without requiring a server-side Modelfile change.

**Detection:** Compare OCR output length against input image dimensions. Log a warning if a large crop (say, >800px in either dimension) returns fewer than 50 characters. Test with a dense-text screenshot at 1920x1080 full-width crop.

**Phase:** Implement the `options.num_ctx` override in the HTTP call during the implementation phase. Document the Modelfile requirement in any setup notes.

---

### Pitfall 3: Using `std::sync::Mutex` Guard Across `.await` in an Async Tauri Command

**What goes wrong:** `platform.rs` currently calls `recognize_text_from_png` as a synchronous function. The new implementation will make an async HTTP call. If the `AppState` mutex (currently `std::sync::Mutex` in `state.rs`) is locked before the `.await` on the HTTP call, the compiler rejects it with "future cannot be sent between threads safely" because `std::sync::MutexGuard` is not `Send`.

**Why it happens:** Tauri 2's built-in tokio runtime is multi-threaded. Every `.await` point is a potential thread switch. `std::sync::MutexGuard` explicitly opts out of `Send` to prevent this. Developers familiar with sync Rust reach for `std::sync::Mutex` instinctively.

**Consequences:** Compile error at minimum. If worked around incorrectly (e.g., `block_on` inside an async context), it causes a runtime panic: "Cannot block the current thread from within an asynchronous context."

**Prevention:** The OCR HTTP call must happen outside the state lock. The correct pattern for `commit_selection` in `lib.rs`:
1. Lock state, extract the PNG bytes, release lock.
2. Call the async OCR function (no lock held).
3. Lock state again, push the result.

If state access must be held across await points for some reason, switch `state.rs` to `tokio::sync::Mutex`, whose guard is `Send`. Do not mix `std::sync::Mutex` and `.await`.

**Detection:** The compiler will catch this. The warning sign at design time is any async function that holds a `std::sync::MutexGuard` and calls `.await` inside the same scope.

**Phase:** Architecture decision in the implementation phase. The lock-extract-release-relock pattern is the lower-risk choice because it avoids migrating `AppState` to a tokio mutex.

---

### Pitfall 4: No Timeout on the Ollama HTTP Request

**What goes wrong:** `reqwest`'s default client has no request timeout. If the Ollama server is reachable but the model is slow (cold start, swapping from disk, CPU-only inference), the Tauri command hangs indefinitely. The UI freezes on "Committing..." with no feedback and no way to cancel.

**Why it happens:** The project requirement is "hard fail on unreachable" — developers implement connection-error handling but omit inference-time timeout, treating them as the same problem. They are not. A refused connection fails in milliseconds. A slow model hangs for 30–120 seconds silently.

**Consequences:** The UI is unresponsive. The `commit_selection` command never returns. On some platforms, the Tauri window becomes visually frozen. The user has no recovery path except force-quit.

**Prevention:** Configure the reqwest `Client` with both a connection timeout and a request timeout:
```rust
let client = reqwest::Client::builder()
    .connect_timeout(std::time::Duration::from_secs(3))
    .timeout(std::time::Duration::from_secs(60))
    .build()
    .map_err(|e| format!("Failed to build HTTP client: {e}"))?;
```
The connection timeout catches "host unreachable" fast. The request timeout catches "model is running but taking too long." 60 seconds is generous for a local network OCR call; tune down if the Ollama instance is consistently fast.

**Detection:** Test with Ollama running but glm-ocr model unloaded (requires a pull). Measure cold-start time. Set timeout to observed cold-start + 15 seconds headroom.

**Phase:** Implement in the HTTP client construction in the implementation phase. Non-negotiable given the hard-fail requirement.

---

### Pitfall 5: Treating All HTTP Errors as "Ollama Unreachable"

**What goes wrong:** `reqwest` surfaces multiple distinct failure modes. Conflating them into one generic error message ("Ollama is unreachable") makes triage impossible and gives users wrong recovery instructions.

**Why it happens:** The simplest implementation maps `Err(e) => Err(e.to_string())`. This is what the current `platform.rs` does for subprocess errors and what the frontend `CONCERNS.md` already flags as a UX problem.

**Consequences:**
- `is_connect()` = true → Ollama process is not running or port 11434 is closed. Correct message: "Ollama is not reachable at 192.168.1.12. Is it running?"
- `is_timeout()` = true → Ollama is up but inference timed out. Correct message: "OCR timed out. The model may be loading — try again."
- HTTP 404 → glm-ocr model is not pulled. Correct message: "glm-ocr model not found on Ollama. Run: ollama pull glm-ocr"
- HTTP 500 → Inference error on server side. Correct message: "Ollama returned a server error. Check Ollama logs."
- HTTP 200 but empty `response` field → Context window truncation (see Pitfall 2). Requires separate detection.

**Prevention:** Match on `reqwest::Error` methods and HTTP status codes explicitly:
```rust
fn classify_ollama_error(e: reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() && e.is_connect() {
        "Ollama is not reachable at 192.168.1.12:11434".into()
    } else if e.is_timeout() {
        "OCR timed out — Ollama may be loading the model".into()
    } else {
        format!("OCR request failed: {e}")
    }
}
```
For non-`reqwest` errors (HTTP 4xx/5xx), check `response.status()` before deserializing the body.

**Detection:** Warning sign is a single `map_err(|e| e.to_string())` on the entire request chain.

**Phase:** Error classification belongs in the implementation phase alongside the HTTP client. The frontend already flash-displays the raw error string, so improving the string quality immediately improves UX without frontend changes.

---

### Pitfall 6: Using Ollama's OpenAI-Compatible Endpoint Instead of Native `/api/generate`

**What goes wrong:** Ollama exposes an OpenAI-compatible `/v1/chat/completions` endpoint. For vision/image input, this endpoint has known compatibility gaps with GLM-OCR. Community reports and the GLM-OCR Hugging Face discussion explicitly recommend using `/api/generate` (or `/api/chat`) with the native format. The OpenAI-compat endpoint may silently drop the `images` field or mangle the request.

**Why it happens:** Developers familiar with OpenAI client libraries default to the `/v1/` path because it allows reusing existing SDKs. The native Ollama API requires a slightly different request shape.

**Consequences:** Images are ignored; the model produces hallucinated text or returns an empty response. The failure is silent — HTTP 200 is returned.

**Prevention:** Use the native Ollama endpoint. For a generate-style request:
```
POST http://192.168.1.12:11434/api/generate
{
  "model": "glm-ocr",
  "prompt": "Extract all text from this image. Output only the extracted text.",
  "images": ["<raw-base64-no-prefix>"],
  "stream": false,
  "options": { "num_ctx": 10240 }
}
```
The response field containing the text is `response` (not `choices[0].message.content`).

**Detection:** Integration test: send a known image and assert the response contains expected text. A wrong endpoint returns 200 but `response` is empty or contains model preamble with no OCR content.

**Phase:** Implementation phase. Lock the endpoint path as a named constant to prevent accidental drift.

---

## Moderate Pitfalls

---

### Pitfall 7: GLM-OCR Wraps Output in Markdown When Input Contains Code or Tables

**What goes wrong:** GLM-OCR uses Markdown formatting automatically when the image contains code blocks, tables, or multi-section documents. Output like ` ```python\nfor i in range...``` ` will pass through `sanitize_ocr_output` unchanged and then enter the Levenshtein merge algorithm, where backtick fences and newlines can confuse overlap detection.

**Prevention:** The existing `sanitize_ocr_output` function in `platform.rs` only strips trailing whitespace and empty lines — it does not strip Markdown syntax. For the OCR use case (copying visible text), optionally strip Markdown fences. At minimum, be aware that the merge threshold logic was calibrated on plain-text OCR output and may behave differently on Markdown-formatted input.

**Phase:** Post-implementation validation. Test with a code-heavy screenshot. If merge quality degrades, extend `sanitize_ocr_output` to strip Markdown fences.

---

### Pitfall 8: `recognize_text_from_png` Is Synchronous; the New Implementation Must Be Async

**What goes wrong:** The current `recognize_text_from_png` is a synchronous `fn`. The Ollama HTTP call must be `async`. The call site in `lib.rs`'s `commit_selection` is already an `async fn`, so the fix is straightforward — but `platform.rs`'s function signature must change, and any blocking `reqwest` client must not be used inside an async context (it will panic with "Cannot drop a runtime in a context where blocking is not allowed").

**Prevention:** Use `reqwest::Client` (async), not `reqwest::blocking::Client`. Mark `recognize_text_from_png` as `async fn`. Update `commit_selection` to `.await` it.

**Detection:** The compiler will catch `reqwest::blocking` misuse in an async context as a panic at runtime, not a compile error. Write a test that calls the function from within a tokio runtime.

**Phase:** Signature change is the first thing to do in the implementation phase — it propagates up the call chain.

---

### Pitfall 9: GLM-OCR Cannot Process Images Larger Than 2048x2048

**What goes wrong:** An open Ollama issue (Issue #14114) documents that GLM-OCR fails or produces garbage on images exceeding 2048x2048 pixels. A 4K full-screen screenshot (3840x2160) fed directly to the model will fail. Cropped selections are usually smaller, but a full-width crop on a 4K display can still exceed this limit.

**Prevention:** After `crop_png` returns bytes, decode the dimensions and resize down to a maximum of 2048 on the longest side before base64-encoding for the Ollama request. The `image` crate (already a dependency) handles this with `resize` or `thumbnail`. Preserve aspect ratio. Do not resize the stored snapshot — only resize the OCR-bound crop.

**Detection:** Test with a full-width marquee selection on a high-DPI display. Warning sign: empty or partial OCR output on large selections that work correctly on small ones.

**Phase:** Implementation phase. Add a `clamp_for_ocr(bytes: &[u8], max_dim: u32) -> Vec<u8>` helper alongside `crop_png` in `platform.rs`.

---

### Pitfall 10: reqwest Client Constructed Per-Request

**What goes wrong:** Constructing a `reqwest::Client` inside `recognize_text_from_png` on every OCR call is wasteful: each construction allocates a connection pool, sets up TLS state, and does DNS work. For a local network call to a hardcoded IP this is minor but non-zero overhead, and it prevents connection reuse.

**Prevention:** Construct the client once. Options: store it in `AppState` alongside `snapshots` and `segments`, or use `once_cell::sync::Lazy` (available in Rust stable via `std::sync::OnceLock` in Rust 1.70+) to create a module-level singleton. Given the existing `SharedState` pattern in this codebase, adding the client to `AppState` is the most consistent approach.

**Detection:** Profile with `cargo tauri dev` and a rapid succession of commits. Symptom is 50–100ms added latency on each call even when Ollama responds quickly.

**Phase:** Can be deferred to a follow-up polish phase. Not a correctness issue, only a performance one.

---

## Minor Pitfalls

---

### Pitfall 11: `serde_json` Deserialization Panics on Unexpected Ollama Response Shape

**What goes wrong:** If Ollama returns an error body (e.g., `{"error": "model not found"}`), deserializing it as the expected success struct (with a `response` field) will return a `None` or cause a deserialization error that surfaces as a confusing internal error rather than the actual cause.

**Prevention:** Deserialize into an enum or check for the `error` key before attempting to extract `response`. A simple approach: deserialize into `serde_json::Value`, check `value["error"]` first, then extract `value["response"].as_str()`.

**Phase:** Implementation phase, handled within the error classification logic from Pitfall 5.

---

### Pitfall 12: Ollama Requires `OLLAMA_HOST=0.0.0.0` to Accept Non-Localhost Connections

**What goes wrong:** By default, Ollama binds to `127.0.0.1:11434`. A client connecting from another machine (or from a different network namespace) will receive a connection refused even though Ollama is running. This is the most common setup mistake for network-accessible Ollama instances.

**Prevention:** This is a server-side configuration concern, not a code concern. Document it in the project's setup notes. The hardcoded endpoint `192.168.1.12` implies the Ollama host must be configured with `OLLAMA_HOST=0.0.0.0` or `OLLAMA_HOST=192.168.1.12`.

**Detection:** From the development machine, run `curl http://192.168.1.12:11434/api/tags` before any code changes. Connection refused at this step = server-side config, not app bug.

**Phase:** Pre-implementation setup verification. Add a health-check call to `/api/tags` in the app's startup or first-OCR path to surface this immediately with a clear message.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|---|---|---|
| HTTP client construction | reqwest blocking vs async mismatch (Pitfall 8) | Use async `reqwest::Client`; mark OCR fn async |
| Base64 encoding for request | Data URL prefix in images array (Pitfall 1) | Strip prefix; unit-test the serialized body |
| Context window | GLM-OCR silent truncation (Pitfall 2) | Pass `options.num_ctx: 10240` in every request |
| Image size pre-processing | 2048x2048 hard limit (Pitfall 9) | Clamp crop dimensions before encoding |
| Endpoint selection | OpenAI-compat endpoint silently drops images (Pitfall 6) | Use `/api/generate` exclusively |
| Error handling | Single `map_err(|e| e.to_string())` (Pitfall 5) | Classify by `is_connect`, `is_timeout`, status code |
| Timeout | No timeout on slow inference (Pitfall 4) | Set both `connect_timeout` (3s) and `timeout` (60s) |
| State lock + await | `std::sync::MutexGuard` not Send (Pitfall 3) | Release lock before `.await`; reacquire after |
| Markdown output | Merge algorithm calibrated on plain text (Pitfall 7) | Test with code/table screenshots post-integration |

---

## Sources

- Ollama GLM-OCR "failed to fully read image" issue: https://github.com/ollama/ollama/issues/14117
- GLM-OCR image size limit (>2048x2048): https://github.com/ollama/ollama/issues/14114
- GLM-OCR blank output on macOS: https://github.com/ollama/ollama/issues/14053
- GLM-OCR no text output / context window: https://huggingface.co/zai-org/GLM-OCR/discussions/8
- Ollama base64 format incorrect (ragflow bug report): https://github.com/infiniflow/ragflow/issues/9452
- Tauri async command Send requirement: https://github.com/tauri-apps/tauri/discussions/7963
- Tauri tokio::main conflict: https://github.com/tauri-apps/tauri/issues/13330
- reqwest error classification: https://webscraping.ai/faq/reqwest/what-is-the-proper-way-to-handle-reqwest-errors
- Ollama connection refused / OLLAMA_HOST: https://github.com/ollama/ollama/issues/2132
- Ollama cold-start timeout: https://localllm.in/blog/cline-ollama-timeout-fix
- Ollama model not found (404): https://github.com/ollama/ollama/issues/2203
- Ollama native vs OpenAI-compat endpoint for vision: https://huggingface.co/zai-org/GLM-OCR/discussions/8
