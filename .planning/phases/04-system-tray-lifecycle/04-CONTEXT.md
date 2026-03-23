# Phase 4: System Tray + App Lifecycle Foundation - Context

**Gathered:** 2026-03-23
**Status:** Ready for execution

<domain>
## Phase Boundary

Convert the app from window-first launch behavior to tray-first lifecycle behavior. After this phase, the process starts in the background, the existing panel UI is created on demand, and closing that panel no longer quits the app.

</domain>

<decisions>
## Implementation Decisions

### Tray lifecycle
- **D-01:** Set the main window config to `create: false` and create it manually with `WebviewWindowBuilder::from_config` when the user opens the panel
- **D-02:** Keep the existing v1 panel UI unchanged for this phase; Phase 8 owns the tray-specific UI rewrite
- **D-03:** Intercept `CloseRequested` and hide the panel instead of destroying it
- **D-04:** Intercept `RunEvent::ExitRequested` and only allow exit after an explicit tray `Quit` action flips an exit guard

### Platform behavior
- **D-05:** Use tray click toggle on platforms where Tauri emits tray click events, but record a tray-menu fallback for Linux because `TrayIconEvent` is unsupported there in Tauri 2.10.3
- **D-06:** Use the existing `src-tauri/icons/icon.png` as the tray icon for the foundation phase; numeric badge icon variants are deferred to Phase 6

### Claude's Discretion
- Exact tray menu wording
- Whether the panel should be `alwaysOnTop` in the initial tray-lifecycle pass
- Minor window geometry adjustments needed to make hidden/manual creation stable

</decisions>

<canonical_refs>
## Canonical References

### Product and phase scope
- `.planning/PROJECT.md` — v2.0 milestone goal and active feature list
- `.planning/REQUIREMENTS.md` — `TRAY-01` through `TRAY-03`
- `.planning/ROADMAP.md` — Phase 4 scope, success criteria, and follow-on phase boundaries

### Research constraints
- `.planning/research/SUMMARY.md` — phase ordering rationale and tray lifecycle pitfalls
- `.planning/research/FEATURES.md` — tray-only UX expectations and status-panel lifecycle notes
- `.planning/research/STACK.md` — required `tauri` tray feature and tray runtime notes

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src-tauri/src/lib.rs` already owns the Tauri builder, command registration, and clipboard plugin wiring
- `src-tauri/src/state.rs` already provides the shared app state used by the existing panel
- `ui/index.html` and `ui/app.js` already form a working panel for the current OCR flow
- `src-tauri/icons/icon.png` is an existing asset that can serve as the initial tray icon

### Established Patterns
- Tauri commands return `Result<_, String>` and front-end `invoke(...)` already handles async responses
- Existing panel commands rely on a single managed `SharedState`
- OCR/capture backend remains untouched in this phase

### Integration Points
- `src-tauri/Cargo.toml` — enable `tray-icon`
- `src-tauri/tauri.conf.json` — move main window to manual creation and hidden/taskbar-skip behavior
- `src-tauri/src/lib.rs` — add tray setup, menu handlers, window show/hide helpers, and guarded exit logic

</code_context>

<specifics>
## Specific Ideas

- Preserve the current panel workflow as the tray-opened surface for now; do not mix Phase 8 UI redesign work into the lifecycle phase
- The tray should always expose a `Quit` path so guarded `prevent_exit()` does not trap the user in the background process

</specifics>

<deferred>
## Deferred Ideas

- TOML config loading and watch-directory bootstrap — Phase 5
- Directory watcher, pending batch state, and numeric tray badges — Phase 6
- Global hotkey and batch OCR pipeline — Phase 7
- Tray-native status-panel redesign — Phase 8

</deferred>

---
*Phase: 04-system-tray-lifecycle*
*Context gathered: 2026-03-23*
