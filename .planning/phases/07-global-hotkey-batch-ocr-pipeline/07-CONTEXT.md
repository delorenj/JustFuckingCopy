# Phase 7: Global Hotkey + Batch OCR Pipeline - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Register a global hotkey (default Ctrl+Shift+C, from config) that triggers batch processing: drain pending screenshots from BatchState, OCR each via ollama::recognize_text in modification-time order, merge/dedup with existing merge algorithm, copy result to clipboard, archive processed files, reset batch, and update tray tooltip. Errors from Ollama surface clearly without crashing.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion -- infrastructure phase.

Key constraints:
- Global hotkey from `AppConfig.hotkey` (default "Ctrl+Shift+C")
- Use `tauri-plugin-global-shortcut` for hotkey registration (Tauri 2 ecosystem)
- On hotkey press: drain pending files from BatchState, read each as PNG bytes
- Process in modification-time order (already sorted by BatchState)
- OCR each screenshot via `ollama::recognize_text()` (async, full PNG, no crop)
- Merge results using `merge::append_text()` for dedup across screenshots
- Copy final merged text to clipboard via `tauri-plugin-clipboard-manager`
- Archive processed files: move to `{watch_dir}/archive/` subdirectory
- Reset batch count and update tray tooltip
- Errors: if Ollama fails on one image, report which one failed, don't crash the batch
- This is where `ollama.rs` hardcoded endpoint could optionally read from config -- but keeping it simple, the endpoint constant is fine for now

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ollama::recognize_text(png_bytes)` -- async OCR, returns Result<String, String>
- `merge::append_text(existing, incoming)` -- fuzzy dedup merge
- `watcher::BatchState` -- `drain_pending()` returns sorted Vec<PathBuf>
- `config::AppConfig` -- `hotkey` field, `watch_dir` for archive path
- `tauri_plugin_clipboard_manager` already in Cargo.toml
- `ClipboardExt` already imported in lib.rs

### Established Patterns
- Async Tauri commands with clone-before-await mutex pattern
- Background thread for watcher (similar pattern for hotkey handler)
- Error propagation as Result<T, String>

### Integration Points
- `lib.rs:run()` -- register global shortcut plugin + hotkey handler
- `watcher.rs:BatchState` -- drain_pending() consumed here
- `ollama.rs:recognize_text()` -- called per screenshot
- `merge.rs:append_text()` -- called to merge results
- Tray tooltip update after processing

</code_context>

<specifics>
## Specific Ideas

No specific requirements -- infrastructure phase.

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope.

</deferred>
