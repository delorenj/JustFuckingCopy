---
phase: 04-system-tray-lifecycle
plan: 01
subsystem: desktop
tags: [tauri, rust, tray, lifecycle, desktop]

# Dependency graph
requires:
  - phase: 03-command-wiring
    provides: working v1 panel UI, Tauri command surface, and completed Ollama OCR pipeline
provides:
  - tray-first startup with manual status-panel creation
  - close-to-hide panel lifecycle instead of app exit
  - explicit tray quit path guarded by runtime exit state
affects: [05-toml-config, 06-directory-watcher-badge, 08-status-panel-ui]

# Tech tracking
tech-stack:
  added: [tauri tray-icon feature]
  patterns:
    - "manual-window-from-config: create the panel lazily via WebviewWindowBuilder::from_config"
    - "guarded-exit: prevent RunEvent::ExitRequested unless an explicit quit path flips an AtomicBool"

key-files:
  created: []
  modified:
    - src-tauri/Cargo.toml
    - src-tauri/src/lib.rs
    - src-tauri/tauri.conf.json

key-decisions:
  - "reuse the existing v1 panel in Phase 4 and defer the tray-native UI rewrite to Phase 8"
  - "Linux tray click events are unavailable in Tauri 2.10.3, so tray menu fallback is the reliable access path there"
  - "explicit quit path is required because ExitRequested is globally guarded to keep the tray process alive"

patterns-established:
  - "tray-first lifecycle: hidden/manual window creation plus close-to-hide instead of close-to-destroy"
  - "future tray phases should treat the panel as an on-demand surface, not the primary app shell"

requirements-completed: [TRAY-01, TRAY-02, TRAY-03]

# Metrics
duration: 35min
completed: 2026-03-23
---

# Phase 4 Plan 1: System Tray + App Lifecycle Foundation Summary

**tray-first app startup with lazy status-panel creation, close-to-hide behavior, and explicit tray quit handling**

## Performance

- **Duration:** 35 min
- **Started:** 2026-03-23T00:00:00Z
- **Completed:** 2026-03-23T00:35:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Enabled Tauri tray support and moved the `main` window to manual creation so app startup no longer shows the panel
- Added tray/menu lifecycle helpers that lazily build the existing panel, toggle visibility, and hide on close requests instead of quitting
- Added guarded exit handling so the process stays alive for tray use, but still exits cleanly through an explicit tray `Quit` action
- Verified `cargo build` and `cargo test` both pass after the lifecycle change

## Task Commits

No task commits were created in this session. Changes remain in the working tree.

## Files Created/Modified

- `src-tauri/Cargo.toml` - enables Tauri `tray-icon` support
- `src-tauri/tauri.conf.json` - switches the main window to manual creation with hidden/taskbar-skipping behavior
- `src-tauri/src/lib.rs` - adds tray setup, panel lifecycle helpers, Linux-safe tray menu fallback, and guarded exit handling

## Decisions Made

- Reused the existing v1 panel instead of redesigning UI during the lifecycle phase to keep scope aligned with the roadmap
- Added tray menu fallback because Tauri 2.10.3 does not emit tray click events on Linux
- Guarded `RunEvent::ExitRequested` with an `AtomicBool` so background tray behavior works without trapping the user in a no-quit state

## Deviations from Plan

None - plan executed as intended.

## Issues Encountered

- Tauri `Builder::run(...)` does not accept a callback in this codebase version; the correct pattern is `build(...).run(...)`
- `cargo fmt` touched `src-tauri/src/ollama.rs` incidentally; that formatting churn was reverted to keep the phase scoped

## User Setup Required

None - no new config or external service setup was introduced in this phase.

## Next Phase Readiness

- Tray-first lifecycle is in place, so config loading can be added cleanly in Phase 5
- The existing panel remains available as the temporary tray-opened surface until Phase 8 replaces the marquee-centric UI
- Manual desktop verification is still needed for tray behavior on the target environment, especially Linux menu fallback and AppIndicator availability

---
*Phase: 04-system-tray-lifecycle*
*Completed: 2026-03-23*
