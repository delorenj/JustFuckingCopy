# Phase 8: Status Panel UI - Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace the marquee-centric frontend UI with a status panel that reflects the tray-driven workflow. Show pending screenshots, last merged text preview, and wire Process Now + Clear Batch action buttons to backend commands.

</domain>

<decisions>
## Implementation Decisions

### Layout
- Replace the hero card, marquee canvas, and selection controls with batch status view
- Show pending file count and list of pending screenshot filenames
- Show last merged text preview in the textarea (read-only)
- Two action buttons: "Process Now" (triggers batch OCR) and "Clear Batch" (clears pending)

### Interaction
- "Process Now" calls a new `process_batch_now` Tauri command (or reuses the hotkey pipeline)
- "Clear Batch" calls a new `clear_batch` Tauri command
- Status auto-refreshes when panel becomes visible (poll or event-driven)
- Error states show flash banner (existing pattern)

### Visual Style
- Keep existing CSS design system (variables, panel cards, buttons)
- Minimal changes -- repurpose existing elements, don't redesign
- Keep the Session Timeline section for showing processed batch history

### Claude's Discretion
- Specific CSS adjustments and layout tweaks
- Whether to poll or use Tauri events for updates
- How to display pending filenames (list vs count vs thumbnails)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ui/styles.css` -- full design system with panels, buttons, flash banner
- `ui/app.js` -- existing invoke pattern, flash(), setBusy(), render()
- `get_batch_state` command already returns `{ pendingCount, pendingFiles }`
- `process_batch` exists in lib.rs (called by hotkey handler)

### Established Patterns
- `invoke()` for all backend calls
- `render()` function rebuilds UI from context state
- Flash banner for success/error messages
- setBusy() for button loading states

### Integration Points
- `ui/index.html` -- restructure panels
- `ui/app.js` -- new commands, remove old marquee logic
- `lib.rs` -- expose `process_batch_now` and `clear_batch` as Tauri commands
- `watcher.rs:BatchState` -- clear_batch needs a `clear()` method

</code_context>

<specifics>
## Specific Ideas

Per user: "The window is not the focus of this app. Although it looks absolutely beautiful! We will populate it with all the settings, show the clipboard history, etc." -- so keep it clean and minimal for now.

</specifics>

<deferred>
## Deferred Ideas

- Settings GUI pane -- v2.1+
- Clipboard history display -- future
- Thumbnail previews of pending screenshots -- future

</deferred>
