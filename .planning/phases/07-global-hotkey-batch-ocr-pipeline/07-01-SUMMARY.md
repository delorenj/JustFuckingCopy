---
phase: 07-global-hotkey-batch-ocr-pipeline
plan: 01
subsystem: hotkey-batch-pipeline
tags: [hotkey, batch-ocr, clipboard, archive, tray, async]
dependency_graph:
  requires: [06-01]
  provides: [HOT-01, BAT-03, BAT-04]
  affects: [lib.rs, Cargo.toml]
tech_stack:
  added: [tauri-plugin-global-shortcut 2]
  patterns: [clone-before-await, async_runtime::spawn, mtime-sort, archive-on-success]
key_files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
decisions:
  - "Global shortcut registered after app.build() using GlobalShortcutExt::on_shortcut (not in .setup()) to satisfy Tauri 2 plugin init ordering"
  - "ShortcutState::Pressed guard prevents double-fire on key-up"
  - "async work spawned via tauri::async_runtime::spawn because on_shortcut handler is sync"
  - "No Mutex held across .await: drain_pending() acquires and drops lock immediately (clone-before-await)"
  - "Only successfully OCR'd files are archived; failed files remain in watch_dir for retry"
  - "Home dir tilde expansion for watch_dir matches pattern already used by watcher"
metrics:
  duration: 92s
  completed: "2026-03-23"
  tasks_completed: 2
  files_modified: 2
---

# Phase 07 Plan 01: Global Hotkey Batch OCR Pipeline Summary

Global hotkey (Ctrl+Shift+C by default) now triggers async batch OCR of all pending screenshots via Ollama, merges with fuzzy dedup, copies to clipboard, archives processed files, and resets the tray tooltip.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add tauri-plugin-global-shortcut to Cargo.toml | 35f3168 | src-tauri/Cargo.toml |
| 2 | Implement process_batch and wire global hotkey in lib.rs | 1082d1b | src-tauri/src/lib.rs |

## What Was Built

### `process_batch` async fn (lib.rs)

1. Drains `BatchState` immediately (lock acquired and dropped — no Mutex across `.await`)
2. Sorts drained `Vec<PathBuf>` by filesystem mtime (oldest first = natural capture order)
3. Expands `~` in `watch_dir` and creates `{watch_dir}/archive/` if absent
4. For each file: reads PNG bytes, calls `ollama::recognize_text`, merges via `merge::append_text`
5. OCR failures log `[JFC batch] OCR failed for {path}: {err}` and skip without crashing
6. If merged text is non-empty, writes to clipboard via `ClipboardExt::write_text`
7. Archives only successfully processed files via `std::fs::rename`
8. Resets tray tooltip to "JustFuckingCopy"

### Hotkey registration (lib.rs `run()`)

- `tauri_plugin_global_shortcut::Builder::new().build()` added to Builder plugin chain
- After `app.build()`, reads `config.hotkey`, registers via `GlobalShortcutExt::on_shortcut`
- Guards on `ShortcutState::Pressed` to prevent double-fire
- Spawns async work via `tauri::async_runtime::spawn`
- Registration failure logs `[JFC hotkey] Failed to register hotkey '...': {err}` — no crash

## Deviations from Plan

None - plan executed exactly as written.

## Verification

```
cargo build: PASSED (1 pre-existing dead_code warning, not introduced by this plan)
cargo test:  PASSED (21 tests, 0 failed)
```

All acceptance criteria met:
- `tauri-plugin-global-shortcut = "2"` in Cargo.toml [dependencies]
- Plugin registered in Builder chain
- `process_batch` async fn: drain, sort by mtime, OCR, merge, clipboard, archive, tray-reset
- Hotkey registered after `app.build()` from `config.hotkey`
- `ShortcutState::Pressed` guard present
- Async work via `tauri::async_runtime::spawn`
- No Mutex held across `.await`
- Single file OCR failure logs and continues

## Known Stubs

None — all logic is fully wired. The pipeline depends on Ollama being reachable at the configured endpoint; unreachable Ollama produces per-file error logs and results in no clipboard write (expected behavior documented in constraints).

## Self-Check: PASSED

- src-tauri/Cargo.toml: FOUND (tauri-plugin-global-shortcut = "2")
- src-tauri/src/lib.rs: FOUND (process_batch, drain_pending, async_runtime::spawn, ShortcutState::Pressed, archive, append_text)
- Commit 35f3168: FOUND
- Commit 1082d1b: FOUND
- cargo test: 21 passed, 0 failed
