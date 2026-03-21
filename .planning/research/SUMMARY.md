# Project Research Summary

**Project:** JustFuckingCopy — Ollama GLM-OCR Integration
**Domain:** Local LLM-based OCR via Ollama HTTP API in a Rust/Tauri 2 desktop app
**Researched:** 2026-03-20
**Confidence:** MEDIUM-HIGH (stack and architecture HIGH, API contract MEDIUM)

## Executive Summary

This milestone replaces three platform-specific OCR backends (Apple Vision Swift subprocess, Tesseract CLI, Windows stub) with a single async HTTP call to an Ollama instance running `glm-ocr` at `192.168.1.12:11434`. The change is tightly scoped: only `platform.rs` is modified in any substantive way, a new `ollama.rs` module is added, and `lib.rs` gains an `async` qualifier on `commit_selection`. The core algorithm (`merge.rs`, `state.rs`) is untouched — it consumes a `String` regardless of OCR source.

The recommended approach is to use `reqwest 0.12` with `rustls-tls` for the HTTP call, posting to `/api/generate` (not the OpenAI-compat endpoint), with `stream: false`, the task prefix `"Text Recognition:"`, raw base64 PNG in the `images` field, and `options.num_ctx: 10240` in every request. Two image pre-processing guards are required before sending to Ollama: strip the `data:image/png;base64,` URI prefix from any stored data URL, and clamp image dimensions to a maximum of 2048px on the longest side. Both guards prevent silent failures that return HTTP 200 with garbled or empty output.

The key risks are well-documented and preventable. GLM-OCR has a confirmed context-window truncation bug at the default 4096-token limit and a confirmed image-size failure above 2048px — both require specific mitigations in the request construction code, not in Ollama server configuration. The async/sync boundary in Tauri's `commit_selection` command requires a clone-before-await pattern to avoid holding a `std::sync::MutexGuard` across an `.await` point, which is a compile error. Every other pitfall (timeout, error classification, endpoint choice) reduces to a handful of lines of code.

## Key Findings

### Recommended Stack

The entire change adds two Cargo dependencies: `reqwest = "0.12"` (async HTTP client, Tokio-native, with `json` and `rustls-tls` features) and `serde_json = "1"` (for `json!()` macro-based request construction). The `base64`, `image`, `serde`, `tauri`, and `tauri-plugin-clipboard-manager` crates remain unchanged. `reqwest 0.13` exists but introduces breaking changes (opt-in `query`/`form` features, `aws-lc` TLS default) that are not worth adopting now; 0.12.24 is the current stable line. `tokio` should be added as a direct dev-dependency with `features = ["rt"]` to enable `#[tokio::test]` in unit tests without conflicting with Tauri's owned runtime entry point.

**Core technologies:**
- `reqwest 0.12`: Async HTTP client for Ollama POST — Tokio-native, avoids runtime conflict, `rustls-tls` eliminates OpenSSL build complexity on Linux
- `serde_json 1`: Request body construction via `json!()` macro — avoids manual struct definitions for a one-off API shape
- `base64 0.22` (existing): Raw base64 encoding of PNG bytes — already present, no new dependency
- `image 0.25` (existing): PNG dimension clamping pre-send — already present, handles the 2048px guard

### Expected Features

The feature scope is a direct backend swap with three mandatory correctness guards. Nothing in the UI changes. The merge algorithm contract (`String` in, `String` out) is unaffected.

**Must have (table stakes):**
- Async POST to `http://192.168.1.12:11434/api/generate` with `model: "glm-ocr"`, `stream: false`, `prompt: "Text Recognition:"`, raw base64 PNG in `images` array — core mechanic
- `options.num_ctx: 10240` in every request — prevents silent context truncation (confirmed production bug)
- Raw base64 (no `data:image/png;base64,` prefix) in `images` array — Ollama silently produces garbage with the URI prefix
- Image dimension clamp to max 2048px before encoding — confirmed GLM-OCR failure above this limit
- 45–60 second request timeout with 3-second connect timeout — prevents indefinite UI freeze on slow/cold-start inference
- Classified error messages distinguishing: unreachable (connection refused), timeout, model not found (404), server error (500) — required by project spec
- Remove all three old OCR backends (Swift, Tesseract, Windows stub) with no `#[cfg]` gates remaining
- Preserve empty-text validation (`"OCR returned no text"` error path)

**Should have (quality improvements):**
- Log OCR round-trip timing to stderr — zero-cost debug aid, single `eprintln!` line
- Response preamble stripping — GLM-OCR may emit conversational wrapper text; strip before passing to `push_segment()`

**Defer (v2+):**
- Configurable Ollama endpoint in UI — one known instance; hardcode the address
- Structured error type enum surfaced to frontend — adds complexity for marginal UX gain at this scope
- `reqwest::Client` singleton (via `AppState` or `OnceLock`) — per-request construction is acceptable for low-frequency single-user use; minor latency issue only
- Markdown stripping from GLM-OCR output — post-integration validation item; address only if merge quality degrades on code/table screenshots

### Architecture Approach

The migration is cleanly separated into one new module and modifications to two existing files. `ollama.rs` (new) owns the entire HTTP boundary: base64 encoding, request construction, response parsing, text sanitization, and error classification. `platform.rs` retains only screenshot capture and `crop_png`; all OCR-related code including `recognize_text_from_png`, `recognize_text_from_file`, `sanitize_ocr_output`, and `VISION_OCR_SCRIPT` move out or are deleted. `lib.rs` gains `async` on `commit_selection` and applies the clone-before-await state lock pattern. `state.rs` and `merge.rs` are not touched.

**Major components:**
1. `ollama.rs` (new) — async HTTP client layer; `pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String>`; no knowledge of AppState or merge logic
2. `platform.rs` (trimmed) — screenshot capture and `crop_png` only; all `#[cfg(target_os)]` OCR dispatch deleted
3. `lib.rs` (updated) — `commit_selection` becomes `async fn`; uses clone-before-await pattern for `SharedState`; calls `ollama::recognize_text(&crop).await?`
4. `state.rs` / `merge.rs` (unchanged) — pure text logic consuming `String`; OCR source is irrelevant

### Critical Pitfalls

1. **Data URL prefix in base64 payload** — Ollama returns HTTP 200 but produces garbled/empty OCR; strip `data:image/png;base64,` prefix before encoding or store raw `png_bytes` separately; unit-test that `images[0]` in the serialized request body does not start with `data:`
2. **GLM-OCR default context window (4096 tokens) silently truncates output** — pass `"options": {"num_ctx": 10240}` in every request body; this overrides the server default without requiring a Modelfile change on the Ollama host
3. **`std::sync::MutexGuard` held across `.await` is a compile error** — lock state, clone needed data, drop the guard, call `ollama::recognize_text().await`, re-acquire lock to write result; do not switch `SharedState` to `tokio::sync::Mutex` (unnecessary cascade of changes)
4. **No timeout causes indefinite UI freeze** — set `connect_timeout(3s)` + `timeout(60s)` on `reqwest::ClientBuilder`; GLM-OCR cold-start can take 30+ seconds; without timeout the frontend freezes with no recovery path
5. **Wrong endpoint (`/v1/chat/completions`) silently drops image data** — use `/api/generate` exclusively; the OpenAI-compat layer has known gaps for vision/image requests with GLM-OCR; returns HTTP 200 with no OCR content
6. **Image dimensions above 2048px cause failure** — add `clamp_for_ocr(bytes, max_dim: 2048)` helper in `platform.rs` alongside `crop_png`; apply before base64-encoding the crop; use the existing `image` crate

## Implications for Roadmap

This milestone is a single cohesive implementation unit. The dependency chain is linear with no parallelizable phases. The suggested build order from architecture research maps directly to phases.

### Phase 1: Infrastructure Setup
**Rationale:** Dependencies must compile before any logic can be written; verifying `cargo build` passes before touching OCR code creates a clean baseline.
**Delivers:** Updated `Cargo.toml` with `reqwest 0.12` and `serde_json 1`; confirmed clean build
**Addresses:** Prerequisite for all subsequent implementation
**Avoids:** Discovering dependency conflicts mid-implementation

### Phase 2: New Ollama HTTP Module
**Rationale:** `ollama.rs` has no dependencies on the rest of the codebase changes; it can be written and unit-tested in isolation against a real Ollama instance before any callers exist.
**Delivers:** `src-tauri/src/ollama.rs` with `pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String>`; includes base64 encoding, request construction, response parsing, error classification, connect/request timeouts
**Addresses:** Table stakes features (correct endpoint, num_ctx override, raw base64, classified errors, timeout)
**Avoids:** Pitfalls 1, 2, 4, 5, 6 (all live in this module's implementation)

### Phase 3: Platform Layer Cleanup
**Rationale:** Remove the old OCR backends after the replacement exists; this keeps the diff atomic and ensures no dangling imports in `lib.rs`.
**Delivers:** `platform.rs` stripped of all OCR code and `#[cfg(target_os)]` dispatch; `vision_ocr.swift` deleted; `sanitize_ocr_output` moved to `ollama.rs`
**Addresses:** Table stakes — remove Apple Vision, Tesseract, Windows stub
**Avoids:** Dead code confusion and false impression of fallback behavior

### Phase 4: Command Layer Wiring
**Rationale:** Connect the new module to the Tauri command handler; this is last because it requires both `ollama.rs` (Phase 2) and the cleaned `platform.rs` (Phase 3) to be in place.
**Delivers:** `commit_selection` in `lib.rs` converted to `async fn`; clone-before-await pattern applied for `SharedState`; `ollama::recognize_text(&crop).await?` call in place
**Addresses:** Async boundary correctness
**Avoids:** Pitfall 3 (MutexGuard across await)

### Phase 5: Integration Validation
**Rationale:** End-to-end smoke test to confirm the full pipeline works before closing the milestone.
**Delivers:** Confirmed working OCR via Ollama; confirmed hard-fail error messages when Ollama is stopped; confirmed correct merge output on multi-capture sessions
**Addresses:** Empty-text validation, error message UX, merge algorithm compatibility
**Avoids:** Pitfall 7 (Markdown output from code screenshots) — test with a code-heavy screenshot and extend `sanitize_ocr_output` if merge quality degrades

### Phase Ordering Rationale

- Phase 1 before 2 because `ollama.rs` will not compile without `reqwest`
- Phase 2 before 3/4 because the new function must exist before it can be called or the old one removed
- Phase 3 before 4 because `lib.rs` must reference the new import path, not the old one; removing the old import before adding the new call keeps the diff atomic
- Phase 5 is terminal; it validates the complete chain and surfaces the Markdown-output pitfall only detectable with real images

### Research Flags

Phases with well-documented patterns (skip additional research):
- **Phase 1:** Standard Cargo dependency management — no research needed
- **Phase 3:** Mechanical deletion — no research needed
- **Phase 4:** Tauri 2 async command pattern is HIGH confidence from official docs; clone-before-await is thoroughly documented

Phases that may benefit from validation during implementation:
- **Phase 2:** GLM-OCR API contract is MEDIUM confidence — the `"Text Recognition:"` prompt and `num_ctx` requirement are well-sourced but behavior with edge-case images (very small crops, non-Latin text) is unverified
- **Phase 5:** Markdown output behavior (Pitfall 7) cannot be fully characterized without real image data; test coverage here will determine whether `sanitize_ocr_output` needs extension

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | `reqwest 0.12` and `serde_json 1` are industry-standard choices with official docs; version guidance confirmed from crates.io |
| Features | HIGH | Table-stakes features derived from confirmed GitHub issues and official GLM-OCR docs; anti-features are deliberate scope constraints |
| Architecture | HIGH | Tauri 2 async command pattern and `std::sync::Mutex` clone-before-await are from official Tauri docs; module split is straightforward |
| Pitfalls | HIGH | Critical pitfalls backed by active GitHub issues with reproduction steps (ollama/ollama #14114, #14117, HuggingFace discussion #8) |

**Overall confidence:** HIGH for implementation approach; MEDIUM for GLM-OCR model behavior edge cases

### Gaps to Address

- **GLM-OCR prompt sensitivity:** The `"Text Recognition:"` prefix is confirmed required, but alternative prompt variants and their effect on output formatting are not characterized. If response preamble stripping becomes complex, revisit the prompt.
- **Ollama version compatibility:** Issues #14117, #14296, #14494, #14498 indicate GLM-OCR had loading failures in Ollama 0.15.6–0.17.4. The running Ollama version at `192.168.1.12` is unknown. Verify version compatibility before integration testing; document the confirmed-working version in setup notes.
- **Non-Latin text fidelity:** GLM-OCR benchmarks focus on English and Chinese. Mixed-language or symbol-heavy captures are untested in this context.
- **`num_ctx` optimal value:** Research cites 10240 as the minimum safe value; 16384 appears in some sources. The FEATURES.md recommendation is 16384 while PITFALLS.md cites 10240. Use 16384 to be conservative — it has no downside on a local inference host.

## Sources

### Primary (HIGH confidence)
- [Tauri 2 async commands](https://v2.tauri.app/develop/calling-rust/) — async fn command handlers, Tokio runtime ownership
- [Tauri 2 state management](https://v2.tauri.app/develop/state-management/) — SharedState patterns
- [Ollama API reference](https://github.com/ollama/ollama/blob/main/docs/api.md) — `/api/generate` response schema
- [Ollama Vision docs](https://docs.ollama.com/capabilities/vision) — base64 image format
- [reqwest crates.io](https://crates.io/crates/reqwest) — 0.12.24 current stable, breaking changes in 0.13

### Secondary (MEDIUM confidence)
- [GLM-OCR Ollama deploy README](https://github.com/zai-org/GLM-OCR/blob/main/examples/ollama-deploy/README.md) — canonical prompt `"Text Recognition:"`, endpoint recommendation
- [ollama.com/library/glm-ocr](https://ollama.com/library/glm-ocr) — model name, available tags
- [GLM-OCR HuggingFace discussion #8](https://huggingface.co/zai-org/GLM-OCR/discussions/8) — `num_ctx` fix, OpenAI-compat endpoint issue
- [ollama-js issue #68](https://github.com/ollama/ollama-js/issues/68) — raw base64 vs data URL format

### Tertiary (issue-tracker evidence)
- [ollama/ollama #14114](https://github.com/ollama/ollama/issues/14114) — GLM-OCR image size limit >2048px
- [ollama/ollama #14117](https://github.com/ollama/ollama/issues/14117) — "failed to fully read image"
- [ollama/ollama #14296, #14494, #14498](https://github.com/ollama/ollama/issues/) — GLM-OCR loading failures in specific Ollama versions (open issues, may be resolved)
- [Tauri discussion #7963](https://github.com/tauri-apps/tauri/discussions/7963) — async command Send requirement

---
*Research completed: 2026-03-20*
*Ready for roadmap: yes*
