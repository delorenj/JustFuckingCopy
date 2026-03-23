# Phase 6: Directory Watcher + Batch State + Badge - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a filesystem watcher that detects new PNG/JPEG screenshots in the configured watch directory. Accumulate detected files as a pending batch in app state. Surface the pending count through the tray icon (tooltip or badge). Debounce rapid writes from screenshot tools.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion -- infrastructure phase.

Key constraints:
- Use `notify` crate (mature, cross-platform filesystem watcher)
- Watch directory comes from `AppConfig.watch_dir` (Phase 5)
- Debounce rapid writes (screenshot tools write temp file then rename)
- Track pending files in app state as `Vec<PathBuf>` sorted by modification time
- Only detect PNG/JPEG by extension (`.png`, `.jpg`, `.jpeg`)
- Tray tooltip updated with pending count (e.g., "JustFuckingCopy (3 pending)")
- Tray icon badge if Tauri supports it, otherwise tooltip-only is acceptable
- Watcher runs in background thread, sends events to Tauri event system
- Existing OCR session state in SharedState remains untouched -- batch is a parallel concern

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `AppConfig` in `config.rs` provides `watch_dir` path
- `SharedState` pattern in `state.rs` for Mutex-wrapped state
- Tray icon built in `setup_tray()` in `lib.rs` with tooltip support
- `TrayIconBuilder::with_id(MAIN_WINDOW_LABEL)` gives access to tray icon for updates

### Established Patterns
- State managed via `tauri::manage()` and `State<'_,T>` injection
- Background work uses `std::thread` (see `capture_snapshot` sleep pattern)
- Error handling: `Result<T, String>` throughout

### Integration Points
- `lib.rs:run()` -- start watcher after config is loaded
- `lib.rs:setup_tray()` -- tray tooltip/badge updates
- `state.rs` -- extend with batch tracking (or create separate BatchState)
- Phase 7 will consume the pending batch for OCR processing

</code_context>

<specifics>
## Specific Ideas

No specific requirements -- infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope.

</deferred>
