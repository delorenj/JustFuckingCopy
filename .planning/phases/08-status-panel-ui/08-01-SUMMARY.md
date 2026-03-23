---
phase: 08-status-panel-ui
plan: 01
subsystem: frontend-ui
tags: [ui, tauri-commands, batch-workflow, status-panel]
dependency_graph:
  requires: [07-01]
  provides: [status-panel-ui, process_batch_now-command, clear_batch-command]
  affects: [ui/index.html, ui/app.js, ui/styles.css, src-tauri/src/watcher.rs, src-tauri/src/lib.rs]
tech_stack:
  added: []
  patterns: [invoke-ipc, auto-refresh-on-focus, batch-status-render]
key_files:
  created: []
  modified:
    - src-tauri/src/watcher.rs
    - src-tauri/src/lib.rs
    - ui/index.html
    - ui/app.js
    - ui/styles.css
decisions:
  - "Reuse existing process_batch async fn by wrapping it as process_batch_now Tauri command — zero duplication"
  - "Auto-refresh batch state on window focus so panel reflects screenshots dropped while panel was hidden"
  - "Additive CSS only — all existing design system variables and rules preserved, new rules appended"
metrics:
  duration: 109s
  completed: "2026-03-23T07:40:35Z"
  tasks_completed: 2
  tasks_total: 3
  files_modified: 5
---

# Phase 08 Plan 01: Status Panel UI Summary

Status panel UI replacing the marquee/canvas workflow with a batch-status dashboard, plus two new Tauri commands to trigger and reset the batch pipeline from the panel.

## What Was Built

**Backend additions (watcher.rs + lib.rs):**
- `BatchState::clear()` — clears pending list without returning it, used by `clear_batch` command
- `process_batch_now` Tauri command — async wrapper around existing `process_batch()` private fn, callable from the frontend
- `clear_batch` Tauri command — calls `state.clear()` and resets tray tooltip to "JustFuckingCopy"
- Both commands registered in `invoke_handler!` macro

**Frontend replacements (index.html + app.js):**
- Removed all marquee/canvas/snapshot/selection elements and logic
- New status dashboard: Ambient Tray header, Process Now + Clear Batch buttons, Pending / Processed / Clipboard status pills
- Pending screenshots list (batch queue panel) renders filenames as queued cards
- Merged output textarea (read-only, full clipboard-bound text)
- Session timeline section shows processed segments from `get_app_state`
- Auto-refresh on `window.focus` event so panel reflects changes made while hidden
- `process_batch_now` disables Process Now button when pending count is 0

**CSS additions (styles.css):**
- `.status-workspace` — 0.8fr / 1.2fr two-column layout for pending + output panels
- `.pending-file-card` — compact padding for queued file cards
- `.pending-badge` — blue-tinted badge for queued items
- Responsive: single-column at ≤1100px

## Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add BatchState::clear() and new commands | 3d25f6e | watcher.rs, lib.rs |
| 2 | Replace marquee UI with status dashboard | ffa2307 | index.html, app.js, styles.css |

## Verification Results

- `cargo build` exits 0 (pre-existing `dead_code` warning for `run_capture_command` only)
- `cargo test` — 21 passed, 0 failed
- `grep -c "canvas|marquee|captureButton|commitSelection|onPointerDown" ui/app.js` → 0
- `grep -c "process_batch_now|clear_batch|get_batch_state" ui/app.js` → 3
- All four required IDs present in index.html: `process-now-button`, `clear-batch-button`, `pending-count`, `pending-list`
- No old IDs (`snapshot-canvas`, `capture-button`, `commit-selection-button`) in index.html
- `.status-workspace` and `.pending-badge` present in styles.css
- Task 3 (checkpoint:human-verify) skipped per autonomous workflow instructions

## Deviations from Plan

None — plan executed exactly as written. Task 3 checkpoint skipped per executor instructions.

## Known Stubs

None. All UI elements are wired to live Tauri commands. The merged output textarea reads from `appState.mergedText` which is populated by the real OCR pipeline. The pending list reads from `batchState.pendingFiles` which is populated by the filesystem watcher.

## Self-Check: PASSED

Files confirmed present:
- src-tauri/src/watcher.rs — FOUND (clear() method at line 54)
- src-tauri/src/lib.rs — FOUND (process_batch_now at line 344, clear_batch at line 350)
- ui/index.html — FOUND (process-now-button, pending-list, no canvas elements)
- ui/app.js — FOUND (process_batch_now, clear_batch, get_batch_state — no canvas/marquee code)
- ui/styles.css — FOUND (status-workspace, pending-badge appended)

Commits confirmed:
- 3d25f6e: feat(JFC-1): add BatchState::clear() and expose process_batch_now + clear_batch commands
- ffa2307: feat(JFC-1): replace marquee UI with status dashboard
