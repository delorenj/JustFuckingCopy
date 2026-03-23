---
phase: "06"
plan: "01"
subsystem: watcher
tags: [filesystem, batch-state, tray, notify, background-thread]
dependency_graph:
  requires: [config.rs AppConfig managed state (phase 05)]
  provides: [BatchState managed state, start_watcher background watcher, get_batch_state Tauri command]
  affects: [lib.rs (wiring), Phase 07 (hotkey+batch OCR will call drain_pending)]
tech_stack:
  added: [notify = "6" (macos_fsevent feature)]
  patterns: [managed state parallel to SharedState, background thread keepalive for watcher]
key_files:
  created: [src-tauri/src/watcher.rs]
  modified: [src-tauri/Cargo.toml, src-tauri/src/lib.rs]
decisions:
  - "Use notify::recommended_watcher directly (no extra dep) with EventKind::Create + RenameMode::To filtering"
  - "Non-existent watch_dir logs warning and returns Ok (no app crash on startup)"
  - "drain_pending left unused for Phase 07 (acceptable dead_code warning)"
metrics:
  duration: "2m 28s"
  completed_date: "2026-03-23T07:22:59Z"
  tasks_completed: 2
  files_modified: 3
---

# Phase 06 Plan 01: Directory Watcher, BatchState, and Tray Badge Summary

Filesystem watcher backed by BatchState managed state; new PNG/JPEG files in watch_dir are accumulated deduplicated in a pending batch and reflected in the tray tooltip as "JustFuckingCopy (N pending)".

## What Was Built

- `watcher.rs` module with `BatchState` struct (Mutex-wrapped `Vec<PathBuf>`), three methods (`add_pending_file`, `pending_count`, `drain_pending`), and `start_watcher` function
- `notify` crate v6 added to Cargo.toml with `macos_fsevent` feature
- `lib.rs` wired to register `BatchState` as managed state, start the watcher after app build, and expose a `get_batch_state` Tauri command

## Tasks Completed

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add notify dependency and create watcher.rs with BatchState | 6d1639e | src-tauri/src/watcher.rs, src-tauri/Cargo.toml, src-tauri/src/lib.rs (mod decl) |
| 2 | Wire BatchState and watcher into lib.rs; add get_batch_state command | 9fcf43d | src-tauri/src/lib.rs |

## Verification Results

- `cargo test`: 21 passed, 0 failed
- `cargo build`: Finished (dev) with 2 pre-existing warnings only
- All 7 watcher unit tests pass (dedup, extension filter, jpg/jpeg acceptance, pending_count, drain, thread safety, missing-dir graceful)
- All 4 acceptance criteria grep checks pass

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Import] Added Manager trait import to watcher.rs**
- **Found during:** Task 1 (test compilation)
- **Issue:** `AppHandle::state()` requires the `Manager` trait to be in scope
- **Fix:** Added `use tauri::Manager;` to watcher.rs imports
- **Files modified:** src-tauri/src/watcher.rs
- **Commit:** 6d1639e (included in same commit)

## Known Stubs

None. `drain_pending` is fully implemented but unused until Phase 07 consumes the batch. This is intentional — the method is ready but the caller doesn't exist yet.

## Self-Check: PASSED

- [x] src-tauri/src/watcher.rs exists
- [x] src-tauri/Cargo.toml contains notify
- [x] src-tauri/src/lib.rs contains mod watcher, BatchState::default, start_watcher, get_batch_state
- [x] Commits 6d1639e and 9fcf43d exist in git log
