# Architecture

**Analysis Date:** 2026-03-20

## Pattern Overview

**Overall:** Tauri IPC Bridge Architecture with Backend State Machine + Frontend Event Loop

**Key Characteristics:**
- Frontend-backend separation via Tauri `invoke()` IPC (all communication serialized as JSON)
- Backend maintains single centralized `SharedState` (Mutex-wrapped `AppState`) for all session data
- Pure functional merge algorithm decoupled from platform/UI concerns
- Pluggable platform backends for screenshot and OCR (compile-time feature selection)
- Stateless request/response pattern (backend commands return full `AppStatePayload` after each mutation)

## Layers

**IPC Command Handler Layer (Public API):**
- Purpose: Expose Tauri commands as the sole interface between frontend and backend
- Location: `src-tauri/src/lib.rs`
- Contains: 6 command functions decorated with `#[tauri::command]`
- Depends on: `state`, `platform`, `merge` modules
- Used by: Frontend via `invoke("command_name")`
- Examples: `capture_snapshot`, `commit_selection`, `get_app_state`, `copy_merged_text`

**State Management Layer:**
- Purpose: Hold and mutate session state; orchestrate merge algorithm on state changes
- Location: `src-tauri/src/state.rs`
- Contains: `SharedState` (Mutex wrapper), `AppState` (inner mutable state), serializable payloads
- Depends on: `merge` module for algorithm execution
- Used by: IPC command handlers
- Key methods: `push_segment()` (adds OCR result and triggers `rebuild_merge()`), `undo_last_segment()`, `store_snapshot()`

**Merge Algorithm Layer (Core Logic):**
- Purpose: Fuzzy deduplication of overlapping text using Levenshtein similarity
- Location: `src-tauri/src/merge.rs`
- Contains: Pure functions with no side effects
- Depends on: Nothing (no other modules)
- Used by: `AppState.rebuild_merge()` to compute merged text
- Key function: `append_text(existing: &str, incoming: &str) -> MergeOutcome`
- Algorithm: Longest suffix-prefix overlap detection with thresholds (93% for 1-2 lines, 78% for 3+ lines)

**Platform Abstraction Layer:**
- Purpose: Hide OS-specific screenshot capture and OCR behind a unified interface
- Location: `src-tauri/src/platform.rs`
- Contains: `capture_snapshot()`, `crop_png()`, `recognize_text_from_png()` (public); OS-specific private fns
- Depends on: `image` crate for PNG decoding/encoding; platform CLIs (screencapture, tesseract, powershell, swift)
- Used by: IPC command handlers (`capture_snapshot`, `commit_selection`)
- Compile-time selection: `#[cfg(target_os = "...")]` blocks define macOS, Linux, Windows implementations

**Frontend UI Layer:**
- Purpose: Render app state, handle user interactions, invoke backend commands
- Location: `ui/` (vanilla JS, no framework)
- Contains: Single-page HTML + JS, CSS styles
- Depends on: Tauri global `__TAURI__.core.invoke` for backend communication
- Used by: User (GUI interactions)

## Data Flow

**Capture → Select → Merge → Copy Flow:**

1. User clicks "Capture Snapshot" button → Frontend calls `invoke("capture_snapshot")`
2. Backend `capture_snapshot()` command:
   - Hides Tauri window
   - Calls `platform::capture_snapshot()` (OS-specific subprocess)
   - Receives PNG bytes + dimensions
   - Stores snapshot in `AppState` via `store_snapshot()`
   - Returns `SnapshotPayload` (base64-encoded data URL)
3. Frontend receives snapshot, loads image into canvas, displays
4. User draws marquee on canvas (pointer events)
5. User clicks "Commit Selection" button → Frontend calls `invoke("commit_selection", {snapshotId, selection})`
6. Backend `commit_selection()` command:
   - Fetches stored snapshot from state
   - Calls `platform::crop_png()` to extract selection
   - Calls `platform::recognize_text_from_png()` (OS-specific OCR)
   - Calls `AppState::push_segment()` with OCR text
   - Inside `push_segment()`: calls `rebuild_merge()`
   - `rebuild_merge()` iterates segments, calls `merge::append_text()` on each
   - Returns updated `AppStatePayload` with merged text + segment metadata
7. Frontend receives updated state, renders timeline + merged textarea
8. User clicks "Copy Merged Text" → Frontend calls `invoke("copy_merged_text")`
9. Backend `copy_merged_text()` command:
   - Reads merged text from state
   - Writes to native clipboard via `tauri_plugin_clipboard_manager`
   - Returns merged text string

**State Mutations:**
- All state mutations are deterministic: command receives request, acquires lock on `SharedState.inner`, mutates `AppState`, releases lock, returns new full state
- Each command call includes potential race condition check (e.g., snapshot ID validation in `commit_selection`)

## Key Abstractions

**SharedState / AppState:**
- Purpose: Single source of truth for entire session
- Examples: `src-tauri/src/state.rs` lines 11-22
- Pattern: Mutex protects inner `AppState` from concurrent access
- Fields:
  - `next_snapshot_id`, `next_segment_id`: Auto-incrementing IDs
  - `current_snapshot`: Latest full PNG bytes + metadata
  - `segments`: Vec of `StoredSegment` (OCR results + merge metadata)
  - `merged_text`: Computed final string

**StoredSegment:**
- Purpose: Captures immutable record of one OCR operation
- Examples: `src-tauri/src/state.rs` lines 62-71
- Fields: `id`, `order`, `snapshot_id`, `selection` (rect), `recognized_text`, `merge_strategy`, `overlap_lines`, `created_at_epoch_ms`
- Serialized to frontend as `SegmentPayload` (identical structure, JSON-compatible)

**MergeStrategy Enum:**
- Purpose: Tag each segment with how it was merged
- Examples: `src-tauri/src/merge.rs` lines 1-17
- Variants:
  - `Initial`: First segment, no merge
  - `OverlapDeduped`: Fuzzy suffix-prefix overlap detected and trimmed
  - `SequentialAppend`: No overlap found, appended as-is

**Platform Trait Pattern (Compile-Time):**
- Purpose: Same function signatures, different implementations per OS
- Examples: `recognize_text_from_file()` in `src-tauri/src/platform.rs` has 3 implementations (lines 67-97 for macOS, 100-123 for Linux, 126-128 for Windows stub)
- Pattern: `#[cfg(target_os = "...")]` attributes on functions with identical signatures

## Entry Points

**Tauri Runtime Entry:**
- Location: `src-tauri/src/main.rs` line 4
- Triggers: Binary execution (`cargo tauri dev` or app launch)
- Responsibilities: Calls `just_fucking_copy_lib::run()`

**Tauri Builder Setup:**
- Location: `src-tauri/src/lib.rs` lines 141-155
- Triggers: Called from `main.rs`
- Responsibilities:
  - Register clipboard plugin
  - Create `SharedState` as managed state
  - Register all 6 command handlers with `invoke_handler`
  - Run Tauri message loop

**Frontend Entry:**
- Location: `ui/app.js` lines 36-39 and 318-320
- Triggers: HTML loads script, `main()` called
- Responsibilities:
  - Bind DOM event listeners to UI buttons and canvas
  - Call `refreshState()` to fetch initial app state
  - Set up `canvasContext` for drawing

## Error Handling

**Strategy:** Result-based (Rust `Result<T, String>`), converted to JSON error responses

**Patterns:**

1. **Platform Errors Propagate Up:**
   ```rust
   pub fn capture_snapshot() -> Result<(Vec<u8>, u32, u32), String> {
       let bytes = fs::read(&path).map_err(|error| format!("Failed to read: {error}"))?;
   ```
   If `fs::read` fails, error string is returned to frontend

2. **State Lock Poisoning Check:**
   ```rust
   let guard = state.inner.lock()
       .map_err(|_| "State lock was poisoned.".to_string())?;
   ```
   Mutex poisoning is checked explicitly (rare but guards against panics)

3. **Validation Before Processing:**
   ```rust
   if snapshot.id != request.snapshot_id {
       return Err("The snapshot changed before this selection was committed.".into());
   }
   ```
   Race condition detection in `commit_selection`

4. **OCR Validation:**
   ```rust
   if recognized_text.trim().is_empty() {
       return Err("OCR returned no text. Try a tighter marquee or a clearer zoom level.".into());
   }
   ```

5. **Frontend Error Display:**
   ```javascript
   catch (error) {
       flash(String(error), true);  // Renders red error banner
   }
   ```
   All backend errors become user-visible flash messages

## Cross-Cutting Concerns

**Logging:** None. Status updates via flash messages (UI notifications). Platform commands log to stderr via subprocess output.

**Validation:**
- **OCR text**: Checked non-empty after recognition
- **Selection rect**: Checked non-zero dimensions on canvas (line 264 in `app.js`)
- **Snapshot ID**: Validated in `commit_selection` to prevent stale selections
- **PNG bounds**: Checked in `crop_png()` to prevent out-of-bounds crops

**Authentication:** None. Desktop app, no user login. Clipboard write requires OS permission (macOS Screen Recording, Linux `/dev/dri` access, Windows UAC).

**Async/Sync Boundary:**
- Frontend: All Tauri `invoke()` calls are `async` (return Promise)
- Backend: All commands are sync (acquire lock, compute, release)
- No background tasks or channels (simple request/response model)

---

*Architecture analysis: 2026-03-20*
