---
phase: 05-toml-config
plan: "01"
subsystem: config
tags: [config, toml, rust, tauri-state, dirs]
dependency_graph:
  requires: []
  provides: [AppConfig, load_or_create, config_path]
  affects: [lib.rs, downstream phases 06 and 07]
tech_stack:
  added: [dirs = "5", toml = "0.8"]
  patterns: [Tauri managed state, platform-correct config path via dirs::config_dir()]
key_files:
  created:
    - src-tauri/src/config.rs
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
key_decisions:
  - "Used AtomicU64 counter for unique temp paths in parallel tests (process::id() alone causes race conditions)"
  - "load_or_create_at() testable variant separates path resolution from I/O for clean unit testing"
  - "All three config fields required in TOML file (no Option fields); partial overrides not supported in v2.0"
metrics:
  duration: "3m 1s"
  completed: "2026-03-23"
  tasks_completed: 2
  files_changed: 3
---

# Phase 05 Plan 01: TOML Config Module Summary

TOML-backed AppConfig module with platform-correct path resolution, default-write-on-missing, malformed-TOML fallback, and AppConfig wired as Tauri-managed state.

## What Was Built

- `src-tauri/src/config.rs` — `AppConfig` struct (`watch_dir`, `hotkey`, `ollama_endpoint`) with `Default` impl, `load_or_create()` public function, `load_or_create_at()` testable variant, and `write_defaults()` helper
- `AppConfig` registered via `.manage(app_config)` in `run()` so any command handler can access it via `State<'_, AppConfig>`
- 5 unit tests covering all behavioral contracts

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add toml and dirs dependencies to Cargo.toml | 97b8f5a | src-tauri/Cargo.toml |
| 2 | Create config.rs with AppConfig struct, load_or_create fn, and unit tests | 53ccd47 | src-tauri/src/config.rs, src-tauri/src/lib.rs |

## Verification Results

- `cargo test`: 14 passed, 0 failed (includes 5 new config tests + 9 existing tests)
- `cargo build`: exits 0, no new warnings
- All 5 acceptance criteria met:
  - `test_default_values` — passes
  - `test_load_or_create_missing_file_writes_defaults` — passes
  - `test_load_partial_overrides_uses_defaults_for_missing_fields` — passes
  - `test_malformed_toml_returns_defaults_no_panic` — passes
  - `test_written_default_config_contains_all_keys` — passes

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Parallel test race condition on shared temp path**
- **Found during:** Task 2, GREEN phase
- **Issue:** `temp_path()` used `std::process::id()` alone, causing all parallel tests to share the same temp directory. Two tests writing/cleaning up concurrently caused "No such file or directory" failures.
- **Fix:** Added `AtomicU64 TEST_COUNTER` to generate a unique numeric suffix per test invocation, making each test use a distinct temp directory.
- **Files modified:** `src-tauri/src/config.rs` (test module only)
- **Commit:** 53ccd47

## Known Stubs

None — all config fields have real default values that flow to production behavior. No placeholder text.

## Self-Check: PASSED

- [x] `src-tauri/src/config.rs` exists
- [x] `src-tauri/src/lib.rs` contains `mod config;` at line 1
- [x] `src-tauri/src/lib.rs` contains `.manage(app_config)` at line 326
- [x] Commit 97b8f5a exists (Task 1)
- [x] Commit 53ccd47 exists (Task 2)
- [x] `cargo test` passes (14/14)
- [x] `cargo build` exits 0
