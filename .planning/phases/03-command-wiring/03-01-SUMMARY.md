---
phase: 03-command-wiring
plan: 01
subsystem: api
tags: [tauri, rust, async, ollama, ocr, mutex]

# Dependency graph
requires:
  - phase: 01-ollama-http-module
    provides: ollama::recognize_text async fn (pub async fn recognize_text(png_bytes: &[u8]) -> Result<String, String>)
  - phase: 02-platform-cleanup
    provides: recognize_text_from_png removed from platform.rs; intentional broken call site left in lib.rs for this phase to fix
provides:
  - commit_selection is async fn wired to ollama::recognize_text(&crop).await?
  - No MutexGuard held across .await (clone-before-await pattern applied)
  - Full capture-OCR-merge-copy pipeline compiles and is functional end-to-end
affects: [future-feature-phases, tauri-command-handlers]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "clone-before-await: two separate Mutex lock scopes in async fn to prevent MutexGuard held across .await boundary"

key-files:
  created: []
  modified:
    - src-tauri/src/lib.rs

key-decisions:
  - "clone-before-await pattern: extract snapshot data in lock scope 1, drop guard, await ollama, re-lock in scope 2 — required by Rust's Send bound on async fns"

patterns-established:
  - "clone-before-await: any async Tauri command that needs Mutex state must extract all needed data before .await, drop the guard, then re-acquire after"

requirements-completed: [ASY-01, ASY-02]

# Metrics
duration: 1min
completed: 2026-03-21
---

# Phase 3 Plan 1: Command Wiring Summary

**async commit_selection wired to ollama::recognize_text via clone-before-await Mutex pattern, fixing the intentional Phase 2 compile error and completing the OCR pipeline**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-21T05:14:55Z
- **Completed:** 2026-03-21T05:16:10Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Replaced broken `recognize_text_from_png(&crop)?` call site with `ollama::recognize_text(&crop).await?`
- Made `commit_selection` an `async fn` as required by the await call
- Applied clone-before-await pattern: two separate lock scopes ensure no `MutexGuard` is held across the `.await` boundary (required by Rust's `Send` bound)
- `cargo build` exits 0 with zero errors; all 9 tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite commit_selection as async with clone-before-await pattern** - `38979dc` (feat)
2. **Task 2: Verify all existing tests still pass** - no code changes; verified via test run output

**Plan metadata:** (docs commit — pending)

## Files Created/Modified

- `src-tauri/src/lib.rs` - `commit_selection` made async, broken OCR call replaced with `ollama::recognize_text(&crop).await?`, two separate Mutex lock scopes applied

## Decisions Made

- clone-before-await is the only correct pattern here: `std::sync::MutexGuard<AppState>` is not `Send`, so it cannot be held across an `.await` point in an async fn registered with Tauri's async runtime.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The edit was straightforward. The pre-existing `run_capture_command` dead_code warning in `platform.rs` is out-of-scope and was not touched.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The full capture-OCR-merge-copy pipeline is now wired end-to-end
- Ollama at 192.168.1.12:11434 must be running with `glm-ocr` loaded for OCR to function at runtime
- All three phases (01 ollama-http-module, 02 platform-cleanup, 03 command-wiring) are complete
- The milestone v1.0 OCR replacement is done

---
*Phase: 03-command-wiring*
*Completed: 2026-03-21*
