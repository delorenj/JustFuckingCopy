# Roadmap: JustFuckingCopy

## Milestones

- ✅ **v1.0 Ollama OCR Migration** - Phases 1-3 (shipped 2026-03-21)
- 🚧 **v2.0 Ambient Tray** - Phases 4-8 (in progress)

## Overview

JustFuckingCopy shipped its v1.0 OCR backend migration and now moves into a workflow-level rewrite: from a modal marquee tool to an ambient tray utility. The v2.0 milestone keeps the proven OCR and merge backend, but changes how input is gathered and how output is surfaced: tray-first lifecycle, watched screenshot directory, hotkey-triggered batch processing, and a lightweight status panel.

## Phases

<details>
<summary>✅ v1.0 Ollama OCR Migration (Phases 1-3) - SHIPPED 2026-03-21</summary>

### Phase 1: Ollama HTTP Module
**Goal**: A tested `ollama.rs` module exists that can reliably OCR a PNG via Ollama GLM-OCR
**Depends on**: Nothing (first phase)
**Requirements**: OCR-01, OCR-02, OCR-03, OCR-04, OCR-05, OCR-06, OCR-07, CLN-05
**Plans**: 1 plan

Plans:
- [x] 01-01-PLAN.md — Add `reqwest 0.12` and `serde_json 1` dependencies; create `ollama.rs` with `recognize_text` implementing all request guards, timeout, and error classification

### Phase 2: Platform Cleanup
**Goal**: All legacy OCR code is deleted; `platform.rs` contains only screenshot capture and `crop_png`
**Depends on**: Phase 1
**Requirements**: CLN-01, CLN-02, CLN-03, CLN-04
**Plans**: 1 plan

Plans:
- [x] 02-01-PLAN.md — Delete `vision_ocr.swift`; remove Tesseract, Apple Vision, and Windows stub OCR code from `platform.rs`; remove `recognize_text_from_png` from `lib.rs` import

### Phase 3: Command Wiring
**Goal**: The full pipeline works end-to-end — marquee selection triggers Ollama OCR and produces correct deduplicated clipboard text
**Depends on**: Phase 2
**Requirements**: ASY-01, ASY-02
**Plans**: 1 plan

Plans:
- [x] 03-01-PLAN.md — Convert `commit_selection` to `async fn` with clone-before-await state pattern; replace `recognize_text_from_png` with `ollama::recognize_text(&crop).await?`; verify `cargo build` and `cargo test` pass

</details>

### 🚧 v2.0 Ambient Tray (In Progress)

**Milestone Goal:** Transform the app into an ambient tray workflow with watched screenshots, batch OCR, and clipboard-ready output.

#### Phase 4: System Tray + App Lifecycle Foundation
**Goal**: The app runs as a tray-first background process and the status panel can be shown and hidden without quitting the app
**Depends on**: Phase 3
**Requirements**: TRAY-01, TRAY-02, TRAY-03
**Success Criteria** (what must be TRUE):
  1. App launch does not show the main window, but the process remains available from the tray
  2. Tray affordances can reveal and hide the existing panel UI; on Linux, tray-menu fallback is provided because Tauri tray click events are unsupported there
  3. Closing the panel hides it instead of quitting the application
  4. `cargo build` passes with Tauri tray support enabled
**Plans**: 1 plan

Plans:
- [x] 04-01-PLAN.md — Enable tray support, switch the main window to manual creation, add tray/menu lifecycle wiring, and guard app exit

#### Phase 5: TOML Config
**Goal**: Runtime configuration is loaded from disk with sane defaults for watch directory, hotkey, and Ollama endpoint
**Depends on**: Phase 4
**Requirements**: CFG-01, CFG-02, CFG-03, CFG-04
**Success Criteria** (what must be TRUE):
  1. App loads config from the platform-correct config path and writes a default file when missing
  2. `watch_dir`, `hotkey`, and `ollama_endpoint` are available as typed runtime config values
  3. Malformed config produces a clear warning and falls back to defaults instead of crashing
**Plans**: 1 plan

Plans:
- [x] 05-01-PLAN.md — Add `config.rs`, load TOML config at startup, and manage typed config in the Tauri app

#### Phase 6: Directory Watcher + Batch State + Badge
**Goal**: Incoming screenshots are detected automatically and accumulated as a pending batch
**Depends on**: Phase 5
**Requirements**: BAT-01, BAT-02
**Success Criteria** (what must be TRUE):
  1. New PNG/JPEG screenshots in the watched directory are detected with debounce and without duplicate pending entries
  2. Pending file count is tracked in app state and reflected through the tray affordance
  3. Existing OCR session state remains intact while batch intake is added
**Plans**: 1 plan

Plans:
- [x] 06-01-PLAN.md — Add watcher module, extend app state for pending files, and wire tray count updates

#### Phase 7: Global Hotkey + Batch OCR Pipeline
**Goal**: A single hotkey processes the current screenshot batch into deduplicated clipboard text
**Depends on**: Phase 6
**Requirements**: HOT-01, BAT-03, BAT-04
**Success Criteria** (what must be TRUE):
  1. Pressing the configured hotkey OCRs all pending screenshots in modification-time order
  2. Batch OCR output reuses the existing fuzzy merge algorithm and ends on the clipboard
  3. Successfully processed screenshots are archived and the pending batch resets
  4. Errors from Ollama surface clearly without crashing the tray process
**Plans**: 1 plan

Plans:
- [ ] 07-01-PLAN.md — Register the global hotkey, add async batch processing, archive processed files, and reset batch state

#### Phase 8: Status Panel UI
**Goal**: The panel reflects the tray workflow instead of the old marquee workflow
**Depends on**: Phase 7
**Requirements**: UI-01, UI-02
**Success Criteria** (what must be TRUE):
  1. Status panel shows pending files and the last merged-text preview
  2. `Process Now` and `Clear Batch` controls are wired to backend commands
  3. Error and empty states match the tray-driven workflow
**Plans**: 1 plan

Plans:
- [ ] 08-01-PLAN.md — Replace the marquee-centric UI with the status panel workflow and wire new backend events/commands

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Ollama HTTP Module | v1.0 | 1/1 | Complete | 2026-03-21 |
| 2. Platform Cleanup | v1.0 | 1/1 | Complete | 2026-03-21 |
| 3. Command Wiring | v1.0 | 1/1 | Complete | 2026-03-21 |
| 4. System Tray + App Lifecycle Foundation | v2.0 | 1/1 | Complete | 2026-03-23 |
| 5. TOML Config | v2.0 | 1/1 | Complete   | 2026-03-23 |
| 6. Directory Watcher + Batch State + Badge | v2.0 | 0/1 | Not started | - |
| 7. Global Hotkey + Batch OCR Pipeline | v2.0 | 0/1 | Not started | - |
| 8. Status Panel UI | v2.0 | 0/1 | Not started | - |

---
*Roadmap updated: 2026-03-23 for v2.0 Ambient Tray continuation*
