# Requirements: JustFuckingCopy

**Defined:** 2026-03-20
**Core Value:** Capture visible text from any screen region and get clean, deduplicated clipboard content with zero workflow interruption.

## v1 Requirements

Requirements for the shipped Ollama GLM-OCR migration milestone.

### OCR Backend

- [x] **OCR-01**: OCR requests are sent via HTTP POST to Ollama `/api/generate` at `192.168.1.12:11434` with model `glm-ocr`
- [x] **OCR-02**: PNG image data is base64-encoded without data URL prefix (`data:image/png;base64,` stripped before sending)
- [x] **OCR-03**: Images exceeding 2048px in either dimension are resized proportionally before sending to Ollama
- [x] **OCR-04**: Every Ollama request includes `options.num_ctx: 16384` to prevent context truncation
- [x] **OCR-05**: Ollama requests use `stream: false` with a 60-second timeout
- [x] **OCR-06**: User sees a clear error message when Ollama is unreachable, returns an error, or the model is not loaded
- [x] **OCR-07**: OCR text returned from Ollama feeds into the existing merge/dedup pipeline identically to the old OCR backends

### Code Cleanup

- [x] **CLN-01**: Apple Vision OCR Swift script (`src-tauri/scripts/vision_ocr.swift`) is deleted
- [x] **CLN-02**: Tesseract CLI integration code is removed from `platform.rs`
- [x] **CLN-03**: Windows OCR stub is removed from `platform.rs`
- [x] **CLN-04**: All `#[cfg(target_os)]` conditional compilation blocks for OCR are removed
- [x] **CLN-05**: New `ollama.rs` module contains all Ollama HTTP logic, independently testable

### Async Integration

- [x] **ASY-01**: `commit_selection` Tauri command handler is async
- [x] **ASY-02**: State mutex is not held across `.await` boundaries (clone-before-await pattern)

## v2 Requirements

Requirements for the in-progress ambient tray milestone.

### Tray Lifecycle

- [ ] **TRAY-01**: App launches without showing a main window and remains reachable from the system tray
- [ ] **TRAY-02**: User can reveal and hide the status panel from tray affordances without restarting the app
- [ ] **TRAY-03**: Closing the status panel hides it and keeps the background process alive

### Configuration

- [x] **CFG-01**: App loads configuration from the platform-correct config directory and writes defaults when missing
- [x] **CFG-02**: Config exposes the watched screenshot directory
- [x] **CFG-03**: Config exposes the global hotkey
- [x] **CFG-04**: Config exposes the Ollama endpoint

### Batch Intake

- [x] **BAT-01**: Directory watcher detects new PNG/JPEG screenshots in the watched directory with debounce
- [x] **BAT-02**: Pending screenshot count is tracked in state and surfaced through the tray affordance

### Batch Processing

- [ ] **HOT-01**: Global hotkey triggers batch OCR, fuzzy merge, and clipboard copy
- [ ] **BAT-03**: Pending screenshots are processed in filesystem modification-time order
- [ ] **BAT-04**: Processed screenshots are archived and removed from the pending batch after a successful run

### Status Panel

- [ ] **UI-01**: Status panel shows pending screenshots and the last merged-text preview
- [ ] **UI-02**: Status panel exposes explicit `Process Now` and `Clear Batch` actions

## Future Requirements

Tracked but deferred beyond v2.0.

### Quality

- [ ] **QUA-01**: Prompt tuning for different content types (code vs prose vs mixed)
- [ ] **QUA-02**: Connection health check on app startup with status indicator
- [ ] **CFG-05**: OCR model name is configurable (swap GLM-OCR for other vision models)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Fallback to local OCR | User explicitly chose hard fail over degraded experience |
| Settings GUI window | Deferred to v2.1+; TOML config keeps v2.0 scope tight |
| Clipboard history | Separate product scope from the ambient batch workflow |
| Windows support | Linux/macOS only for the current milestone |
| App bundling/distribution | Dev mode only (`bundle.active = false`) |
| In-app screenshot capture | v2.0 depends on external screenshot tools and a watched directory |
| Marquee selection in tray mode | Full screenshots plus dedup replace manual crop workflow in v2.0 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation and phase completion.

| Requirement | Phase | Status |
|-------------|-------|--------|
| OCR-01 | Phase 1 | Complete |
| OCR-02 | Phase 1 | Complete |
| OCR-03 | Phase 1 | Complete |
| OCR-04 | Phase 1 | Complete |
| OCR-05 | Phase 1 | Complete |
| OCR-06 | Phase 1 | Complete |
| OCR-07 | Phase 1 | Complete |
| CLN-01 | Phase 2 | Complete |
| CLN-02 | Phase 2 | Complete |
| CLN-03 | Phase 2 | Complete |
| CLN-04 | Phase 2 | Complete |
| CLN-05 | Phase 1 | Complete |
| ASY-01 | Phase 3 | Complete |
| ASY-02 | Phase 3 | Complete |
| TRAY-01 | Phase 4 | Complete |
| TRAY-02 | Phase 4 | Complete |
| TRAY-03 | Phase 4 | Complete |
| CFG-01 | Phase 5 | Planned |
| CFG-02 | Phase 5 | Planned |
| CFG-03 | Phase 5 | Planned |
| CFG-04 | Phase 5 | Planned |
| BAT-01 | Phase 6 | Planned |
| BAT-02 | Phase 6 | Planned |
| HOT-01 | Phase 7 | Planned |
| BAT-03 | Phase 7 | Planned |
| BAT-04 | Phase 7 | Planned |
| UI-01 | Phase 8 | Planned |
| UI-02 | Phase 8 | Planned |

**Coverage:**
- v1 requirements: 14 total, 14 complete
- v2 requirements: 12 total, 3 complete, 9 planned
- Future requirements: 3 deferred

---
*Requirements defined: 2026-03-20*
*Last updated: 2026-03-23 for v2.0 roadmap planning*
