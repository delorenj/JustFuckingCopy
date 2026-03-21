# Roadmap: JustFuckingCopy — Ollama OCR Migration

## Overview

Replace three platform-specific OCR backends (Apple Vision Swift, Tesseract CLI, Windows stub) with a single async HTTP call to a local Ollama instance running `glm-ocr`. The change is tightly scoped to `platform.rs`, a new `ollama.rs` module, and the `commit_selection` command handler in `lib.rs`. The merge algorithm and all screenshot capture code are untouched.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Ollama HTTP Module** - Build and test the new `ollama.rs` OCR module in isolation
- [ ] **Phase 2: Platform Cleanup** - Remove all legacy OCR backends and dead code
- [ ] **Phase 3: Command Wiring** - Connect Ollama OCR to `commit_selection` and validate end-to-end

## Phase Details

### Phase 1: Ollama HTTP Module
**Goal**: A tested `ollama.rs` module exists that can reliably OCR a PNG via Ollama GLM-OCR
**Depends on**: Nothing (first phase)
**Requirements**: OCR-01, OCR-02, OCR-03, OCR-04, OCR-05, OCR-06, OCR-07, CLN-05
**Success Criteria** (what must be TRUE):
  1. `cargo build` passes after adding `reqwest 0.12` and `serde_json 1` to `Cargo.toml`
  2. `ollama::recognize_text(png_bytes)` sends a POST to `http://192.168.1.12:11434/api/generate` with `model: "glm-ocr"`, raw base64 PNG (no `data:` prefix), `stream: false`, and `options.num_ctx: 16384`
  3. Images larger than 2048px are resized before the request is sent
  4. When Ollama is unreachable, times out, or returns a model-not-found error, the function returns a descriptive error string distinguishing the failure type
  5. `cargo test` passes for unit tests covering base64 encoding, request construction, and error classification
**Plans**: TBD

Plans:
- [ ] 01-01: Add `reqwest 0.12` and `serde_json 1` dependencies; create `ollama.rs` with `recognize_text` implementing all request guards, timeout, and error classification

### Phase 2: Platform Cleanup
**Goal**: All legacy OCR code is deleted; `platform.rs` contains only screenshot capture and `crop_png`
**Depends on**: Phase 1
**Requirements**: CLN-01, CLN-02, CLN-03, CLN-04
**Success Criteria** (what must be TRUE):
  1. `src-tauri/scripts/vision_ocr.swift` no longer exists in the repository
  2. `platform.rs` contains no Tesseract, Apple Vision, or Windows OCR code
  3. No `#[cfg(target_os)]` conditional compilation blocks for OCR remain in any file
  4. `cargo build` passes with no warnings about unused imports or dead code from removed backends
**Plans**: TBD

Plans:
- [ ] 02-01: Delete `vision_ocr.swift`; remove Tesseract, Apple Vision, and Windows stub OCR code from `platform.rs`; remove all OCR-related `#[cfg(target_os)]` blocks

### Phase 3: Command Wiring
**Goal**: The full pipeline works end-to-end — marquee selection triggers Ollama OCR and produces correct deduplicated clipboard text
**Depends on**: Phase 2
**Requirements**: ASY-01, ASY-02
**Success Criteria** (what must be TRUE):
  1. `commit_selection` in `lib.rs` is `async fn` and compiles without errors (no `MutexGuard` held across `.await`)
  2. Drawing a marquee and committing a selection triggers an Ollama OCR call and the result appears in the merged text area
  3. When Ollama is stopped, committing a selection shows a clear error message in the UI rather than hanging or crashing
  4. Multi-capture sessions with overlapping text produce correctly deduplicated output (merge algorithm unaffected by OCR source change)
**Plans**: TBD

Plans:
- [ ] 03-01: Convert `commit_selection` to `async fn` with clone-before-await state pattern; wire `ollama::recognize_text` call; smoke-test full capture-OCR-merge-copy flow

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Ollama HTTP Module | 0/1 | Not started | - |
| 2. Platform Cleanup | 0/1 | Not started | - |
| 3. Command Wiring | 0/1 | Not started | - |
