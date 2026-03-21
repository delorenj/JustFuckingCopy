# Coding Conventions

**Analysis Date:** 2026-03-20

## Naming Patterns

**Files:**
- Rust module files: lowercase with underscores (`lib.rs`, `merge.rs`, `platform.rs`, `state.rs`, `main.rs`)
- JavaScript files: camelCase with `.js` extension (`app.js`)
- CSS files: lowercase with `.css` extension (`styles.css`)
- HTML files: lowercase with `.html` extension (`index.html`)

**Functions:**
- **Rust:** snake_case for all functions
  - Public functions: descriptive, action-based (`capture_snapshot`, `crop_png`, `recognize_text_from_png`, `append_text`)
  - Private functions: snake_case (`find_overlap`, `canonical_line`, `similarity`, `levenshtein`, `temp_path`, `sanitize_ocr_output`)
  - Tauri commands: snake_case matching JSON API expectations (`get_app_state`, `capture_snapshot`, `commit_selection`, `undo_last_segment`, `copy_merged_text`, `reset_session`)
- **JavaScript:** camelCase for all functions
  - Async functions clearly labeled (`async function captureSnapshot`, `async function commitSelection`)
  - Pointer event handlers: verb + descriptor (`onPointerDown`, `onPointerMove`, `onPointerUp`)
  - Utility/helper functions: descriptive (`loadSnapshotImage`, `normalizedRect`, `toCanvasPoint`, `setBusy`, `flash`, `escapeHtml`)

**Variables:**
- **Rust:** snake_case for locals and struct fields
  - Configuration constants: SCREAMING_SNAKE_CASE with `const` keyword (`VISION_OCR_SCRIPT`)
  - Struct fields: descriptive, plural for collections (`snapshot`, `segments`, `merged_text`, `recognized_text`, `current_snapshot`)
  - Single-letter variables avoided (use descriptive names like `overlap_lines`, `error`, `path`)
- **JavaScript:** camelCase for all variables
  - Objects and maps: plural when collections (`elements`, `context`, `segments`)
  - Booleans: clear intent (`isDragging`, `hidden`, `disabled`)
  - State variables grouped into objects (`context.snapshot`, `context.appState`)

**Types:**
- **Rust:** PascalCase for all types (enums, structs, traits)
  - Enum variants: PascalCase (`Initial`, `OverlapDeduped`, `SequentialAppend`)
  - Payload structs: suffix with `Payload` (`AppStatePayload`, `SnapshotPayload`, `SegmentPayload`)
  - Internal storage structs: prefix with `Stored` (`StoredSnapshot`, `StoredSegment`)
  - Domain types: PascalCase (`SelectionRect`, `MergeOutcome`, `MergeStrategy`, `SharedState`, `AppState`)
- **JavaScript:** No explicit type definitions; inferred from context and usage

## Code Style

**Formatting:**
- **Rust:** Standard Rust formatting conventions (implied via `cargo fmt`, not explicitly configured)
  - Line length: No explicit limit observed, but lines generally kept under 100 characters
  - Indentation: 4 spaces
  - Brace style: Opening brace on same line (K&R style)
- **JavaScript:** Inferred formatting conventions (no formatter detected in repo)
  - Indentation: 2 spaces
  - Line length: Flexible, pragmatic (up to ~90 characters typically)
  - Brace style: Opening brace on same line

**Linting:**
- No ESLint, Prettier, or equivalent JavaScript configuration detected
- No explicit Rust linting configuration beyond standard `cargo clippy` warnings
- Code appears hand-formatted and linted against conventions

## Import Organization

**Rust (`lib.rs` / `src-tauri/src`):**

Order of imports in files:
1. Crate modules (`mod merge; mod platform; mod state;`)
2. Standard library imports (`use std::*`)
3. External crate imports (`use tauri::*; use image::*; use serde::*`)
4. Internal crate imports (`use crate::platform::*; use crate::state::*; use crate::merge::*`)

Example from `lib.rs`:
```rust
mod merge;
mod platform;
mod state;

use std::thread;
use std::time::Duration;

use tauri::{State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::platform::{ /* ... */ };
use crate::state::{ /* ... */ };
```

**JavaScript (`ui/app.js`):**

- Single import at top for Tauri bridge: `const invoke = window.__TAURI__.core.invoke;`
- No modularization; single-file architecture
- No path aliases or explicit import statements beyond Tauri

## Error Handling

**Patterns:**
- **Rust:** Pervasive use of `Result<T, String>` type for fallible operations
  - All public Tauri commands return `Result<T, String>`
  - Error propagation via `?` operator is standard throughout
  - Error messages are descriptive and human-readable, formatted with context (e.g., `format!("Failed to hide window: {error}")`)
  - Lock poisoning on `Mutex` is treated as fatal (`"State lock was poisoned."`)
  - Invalid state conditions return `Err` with clear message (e.g., "Capture a snapshot before committing a selection.")
  - Fallback chains used in multi-backend scenarios (Linux screenshot attempts grim → gnome-screenshot → import, with accumulated error context)

Example from `lib.rs`:
```rust
fn get_app_state(state: State<'_, SharedState>) -> Result<AppStatePayload, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;
    Ok(guard.to_payload())
}
```

- **JavaScript:** Try-catch blocks with frontend error messaging via `flash()` helper
  - All async Tauri command invocations wrapped in try-catch
  - Errors caught and displayed to user via `flash(String(error), true)` (red banner)
  - Success messages communicated via `flash(message)` (neutral banner)

Example from `app.js`:
```javascript
async function captureSnapshot() {
  setBusy(elements.captureButton, true, "Capturing...");

  try {
    const snapshot = await invoke("capture_snapshot");
    // ... success path
    flash(`Snapshot ${snapshot.id} ready. Drag a marquee and commit it.`);
  } catch (error) {
    flash(String(error), true);
  } finally {
    setBusy(elements.captureButton, false, "Capture Snapshot");
  }
}
```

## Logging

**Framework:** No logging framework detected

**Patterns:**
- **Rust:** No `println!`, `dbg!`, or logging crate used
  - Errors communicated via `Result` type and error messages
  - No debug output or tracing
- **JavaScript:** No logging library; all user feedback via `flash()` utility function
  - Banner-based messaging system (`flash(message, isError)`)
  - No console output in normal operation
  - Error messages passed from backend converted to user-facing text

## Comments

**When to Comment:**
- Comments are minimal throughout the codebase
- Self-documenting code is preferred (descriptive function/variable names)
- No JSDoc or RustDoc comments found in active code

**Attributes and Annotations:**
- **Rust:**
  - `#[tauri::command]` - Marks functions as Tauri command handlers (RPC endpoints)
  - `#[cfg(target_os = "...")]` - Platform-specific code guards (macOS, Linux, Windows)
  - `#[cfg(test)]` - Test module conditional compilation
  - `#[derive(...)]` - Trait derivation (Debug, Clone, Default, Serialize, Deserialize)
  - `#[serde(rename_all = "camelCase")]` - JSON field name transformation
  - `#[allow(dead_code)]` - Suppresses warnings for utility functions not yet used
- **JavaScript:**
  - No decorators or attributes used

## Function Design

**Size:**
- **Rust:** Public API functions in `lib.rs` are typically 10-20 lines (focused on request → command → response)
- **JavaScript:** Frontend async functions typically 15-25 lines (try-catch-finally wrapping)
- Helpers and pure logic functions (merge algorithm, geometry, UI rendering) range 5-30 lines

**Parameters:**
- **Rust:**
  - Functions take state as `State<'_, SharedState>` (Tauri injection)
  - Request structs used for multi-field parameters (e.g., `CommitSelectionRequest`)
  - Owned `String` for error handling (not `&str`)
  - Primitive types passed by value (`u32`, `f32`, `usize`)
  - Slices for bulk data (`&[u8]` for byte arrays)
- **JavaScript:**
  - No parameters beyond event handlers in most cases (state accessed via global `context`)
  - Event objects passed to pointer handlers (`onPointerDown(event)`)

**Return Values:**
- **Rust:**
  - All public functions return `Result<T, String>` for frontend-facing operations
  - Helper/internal functions return `Option<T>` or bare values where fallible or not
  - Tuples for multiple return values (`(Vec<u8>, u32, u32)` for screenshot bytes + dimensions)
- **JavaScript:**
  - Async functions return `Promise` (implicit via async/await)
  - No explicit return values in event handlers (state updates happen via side effects)
  - Helper functions return primitives (`boolean`, `object`, `string`)

## Module Design

**Exports:**
- **Rust:**
  - Public functions marked with `pub` keyword
  - Commands marked `#[tauri::command]` for automatic registration
  - Internal helper functions kept private (`fn`, not `pub fn`)
  - Structs and types exported as public when part of API boundary (payloads, domain types)
- **JavaScript:**
  - Single-file module; no explicit exports
  - `main()` invoked at file bottom to initialize
  - Global `invoke` and `context` objects accessible throughout

**Barrel Files:**
- Not used (codebase is too small and modular)
- Each Rust file (`merge.rs`, `platform.rs`, `state.rs`) is self-contained
- No `mod.rs` or `__init__.py` pattern

---

*Convention analysis: 2026-03-20*
