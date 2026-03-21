---
phase: 01-ollama-http-module
plan: "01"
subsystem: api
tags: [rust, reqwest, serde_json, ollama, ocr, base64, image-processing, tokio]

requires: []
provides:
  - "src-tauri/src/ollama.rs: self-contained async OCR module using Ollama GLM-OCR HTTP API"
  - "reqwest 0.12 and serde_json 1 added to Cargo.toml"
  - "pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String>"
  - "Image resize guard clamping images to 2048px max dimension before OCR"
  - "Three distinct error classes: connection failure, timeout, HTTP status error"
affects:
  - 02-platform-cleanup
  - 03-wiring

tech-stack:
  added:
    - "reqwest 0.12 (default-features=false, features=[json, rustls-tls])"
    - "serde_json 1"
    - "tokio 1 (dev-dep, features=[rt, macros])"
  patterns:
    - "Async HTTP via reqwest::Client with per-client connect and read timeouts"
    - "serde_json::json! macro for building Ollama API request bodies"
    - "Error classification via reqwest::Error::is_connect() and is_timeout()"
    - "Image resize with proportional scaling using image::imageops::FilterType::Lanczos3"

key-files:
  created:
    - src-tauri/src/ollama.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs

key-decisions:
  - "reqwest with rustls-tls (not native-tls): avoids OpenSSL link complexity on Linux, consistent with Tauri's own TLS approach"
  - "tokio as dev-dep only: Tauri owns the runtime; tokio dev-dep enables #[tokio::test] without runtime conflicts"
  - "module registered in lib.rs without wiring: ollama.rs compiles and is tested independently; command wiring deferred to Phase 3"
  - "sanitize_ocr_output copied verbatim from platform.rs: ensures identical text normalization behavior across OCR backends"

patterns-established:
  - "Ollama API pattern: POST /api/generate with model, prompt, images (raw base64), stream:false, options.num_ctx"
  - "Error message pattern: human-readable strings with actionable guidance (Is it running? / model may still be loading)"
  - "Image guard pattern: check dimensions first, return original bytes unchanged if within bounds, resize only when needed"

requirements-completed: [OCR-01, OCR-02, OCR-03, OCR-04, OCR-05, OCR-06, OCR-07, CLN-05]

duration: 2min
completed: "2026-03-21"
---

# Phase 01 Plan 01: Ollama HTTP Module Summary

**Self-contained Rust module `ollama.rs` that OCRs PNG bytes via Ollama GLM-OCR HTTP API with image resize guard, error classification, and 7 passing unit tests**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-21T04:47:04Z
- **Completed:** 2026-03-21T04:49:15Z
- **Tasks:** 2 of 2
- **Files modified:** 3

## Accomplishments

- Added `reqwest 0.12`, `serde_json 1`, and `tokio` dev-dep to Cargo.toml; `cargo build` passes cleanly
- Created `src-tauri/src/ollama.rs` with `pub async fn recognize_text` posting to Ollama at `192.168.1.12:11434/api/generate` with model `glm-ocr`, `stream: false`, `num_ctx: 16384`, and raw base64 PNG (no `data:` prefix)
- Implemented image resize guard (`clamp_image_for_ocr`) that proportionally scales images exceeding 2048px; images within bounds returned unchanged
- All 7 unit tests pass covering: base64 no-prefix guard, image resize clamp with aspect ratio check, connect error classification, timeout message format, response parse success, empty response error, error key in response

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Cargo dependencies and verify clean build** - `991dad4` (chore)
2. **Task 2: Create ollama.rs with recognize_text, image guard, and unit tests** - `03a6d03` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `src-tauri/Cargo.toml` - Added reqwest 0.12, serde_json 1, tokio dev-dep
- `src-tauri/src/ollama.rs` - New Ollama OCR module with all helpers and unit tests
- `src-tauri/src/lib.rs` - Added `mod ollama;` declaration (module not yet wired to commands)

## Decisions Made

- Used `rustls-tls` instead of `native-tls` for reqwest to avoid OpenSSL link complexity on Linux and stay consistent with Tauri's TLS approach
- `tokio` added as dev-dep only (not in `[dependencies]`) because Tauri owns the async runtime; dev-dep provides `#[tokio::test]` without runtime conflicts
- Module declared in `lib.rs` but not wired to any commands — that is explicitly Phase 3's responsibility
- `sanitize_ocr_output` copied verbatim from `platform.rs` so text normalization behavior is identical to existing OCR backends

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - build passed on first attempt with `rustls-tls`. TDD RED/GREEN/REFACTOR was a single pass since the implementation and tests were written together (tests passed immediately — no iteration needed beyond the initial write).

## User Setup Required

None - no external service configuration required for the module itself. Ollama at `192.168.1.12:11434` must be running with `glm-ocr` loaded for `recognize_text` to succeed at runtime (this is a network/runtime dependency, not a build dependency).

## Next Phase Readiness

- Phase 2 (platform cleanup): `ollama.rs` is ready for wiring; `platform.rs` OCR backends can be removed
- Phase 3 (wiring): `pub async fn recognize_text` exports the correct signature for replacing `recognize_text_from_png` calls in `lib.rs`
- Blocker from STATE.md still applies: verify Ollama version at `192.168.1.12` is not in the 0.15.6–0.17.4 range that has known GLM-OCR loading failures before integration testing

## Known Stubs

None - the module is complete. `recognize_text` is fully implemented and tested. It is not yet called from any command handler (by design — that is Phase 3), but the function itself is not a stub.

---
*Phase: 01-ollama-http-module*
*Completed: 2026-03-21*

## Self-Check: PASSED

- FOUND: src-tauri/src/ollama.rs
- FOUND: src-tauri/Cargo.toml (modified)
- FOUND: src-tauri/src/lib.rs (modified)
- FOUND: .planning/phases/01-ollama-http-module/01-01-SUMMARY.md
- FOUND commit: 991dad4 (chore: Cargo deps)
- FOUND commit: 03a6d03 (feat: ollama.rs module)
- FOUND commit: f11ee43 (docs: plan metadata)
