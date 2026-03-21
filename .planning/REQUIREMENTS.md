# Requirements: JustFuckingCopy

**Defined:** 2026-03-20
**Core Value:** Capture visible text from any screen region and get clean, deduplicated clipboard content in as few clicks as possible.

## v1 Requirements

Requirements for OCR backend migration to Ollama GLM-OCR.

### OCR Backend

- [ ] **OCR-01**: OCR requests are sent via HTTP POST to Ollama `/api/generate` at `192.168.1.12:11434` with model `glm-ocr`
- [ ] **OCR-02**: PNG image data is base64-encoded without data URL prefix (`data:image/png;base64,` stripped before sending)
- [ ] **OCR-03**: Images exceeding 2048px in either dimension are resized proportionally before sending to Ollama
- [ ] **OCR-04**: Every Ollama request includes `options.num_ctx: 16384` to prevent context truncation
- [ ] **OCR-05**: Ollama requests use `stream: false` with a 60-second timeout
- [ ] **OCR-06**: User sees a clear error message when Ollama is unreachable, returns an error, or the model is not loaded
- [ ] **OCR-07**: OCR text returned from Ollama feeds into the existing merge/dedup pipeline identically to the old OCR backends

### Code Cleanup

- [ ] **CLN-01**: Apple Vision OCR Swift script (`src-tauri/scripts/vision_ocr.swift`) is deleted
- [ ] **CLN-02**: Tesseract CLI integration code is removed from `platform.rs`
- [ ] **CLN-03**: Windows OCR stub is removed from `platform.rs`
- [ ] **CLN-04**: All `#[cfg(target_os)]` conditional compilation blocks for OCR are removed
- [ ] **CLN-05**: New `ollama.rs` module contains all Ollama HTTP logic, independently testable

### Async Integration

- [ ] **ASY-01**: `commit_selection` Tauri command handler is async
- [ ] **ASY-02**: State mutex is not held across `.await` boundaries (clone-before-await pattern)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Configuration

- **CFG-01**: Ollama endpoint is configurable via environment variable or config file
- **CFG-02**: OCR model name is configurable (swap GLM-OCR for other vision models)

### Quality

- **QUA-01**: Prompt tuning for different content types (code vs prose vs mixed)
- **QUA-02**: Connection health check on app startup with status indicator

## Out of Scope

| Feature | Reason |
|---------|--------|
| Fallback to local OCR | User explicitly chose hard fail over degraded experience |
| Configurable endpoint | Hardcoded for simplicity; only one Ollama instance exists |
| Streaming OCR responses | No UX benefit to partial text; adds complexity |
| Screenshot capture changes | Only OCR changes; capture backends stay platform-specific |
| App bundling/distribution | Dev mode only (`bundle.active = false`) |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| OCR-01 | Phase 1 | Pending |
| OCR-02 | Phase 1 | Pending |
| OCR-03 | Phase 1 | Pending |
| OCR-04 | Phase 1 | Pending |
| OCR-05 | Phase 1 | Pending |
| OCR-06 | Phase 1 | Pending |
| OCR-07 | Phase 1 | Pending |
| CLN-01 | Phase 2 | Pending |
| CLN-02 | Phase 2 | Pending |
| CLN-03 | Phase 2 | Pending |
| CLN-04 | Phase 2 | Pending |
| CLN-05 | Phase 1 | Pending |
| ASY-01 | Phase 3 | Pending |
| ASY-02 | Phase 3 | Pending |

**Coverage:**
- v1 requirements: 14 total
- Mapped to phases: 14
- Unmapped: 0

---
*Requirements defined: 2026-03-20*
*Last updated: 2026-03-21 after roadmap creation*
