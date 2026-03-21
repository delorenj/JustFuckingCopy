# Codebase Structure

**Analysis Date:** 2026-03-20

## Directory Layout

```
JustFuckingCopy/
├── src-tauri/                    # Rust/Tauri backend
│   ├── src/
│   │   ├── lib.rs              # Tauri command handlers (IPC API)
│   │   ├── main.rs             # Binary entry point
│   │   ├── state.rs            # AppState + serialization
│   │   ├── merge.rs            # Merge algorithm + tests
│   │   └── platform.rs         # Screenshot + OCR abstraction
│   ├── scripts/
│   │   └── vision_ocr.swift    # macOS Vision framework wrapper
│   ├── Cargo.toml              # Rust dependencies + metadata
│   ├── tauri.conf.json         # Tauri window + app config
│   ├── build.rs                # Tauri build script
│   ├── capabilities/
│   │   └── default.json        # Tauri permissions manifest
│   ├── icons/
│   │   └── icon.png            # App icon
│   ├── gen/                    # Tauri generated files (auto)
│   │   └── schemas/            # JSON schemas for capabilities
│   ├── Cargo.lock              # Dependency lock
│   └── target/                 # Build output (git-ignored)
├── ui/                          # Vanilla JS frontend
│   ├── app.js                  # Single app entry + state/events
│   ├── index.html              # HTML structure
│   └── styles.css              # Styling
├── CLAUDE.md                    # Project instructions
├── README.md                    # Public docs
└── .planning/
    └── codebase/               # This analysis output
```

## Directory Purposes

**`src-tauri/src/`:**
- Purpose: Core Rust backend logic
- Contains: Command handlers, state machine, merge algorithm, platform abstraction
- Key files: `lib.rs` (6 commands), `state.rs` (session data), `merge.rs` (deduplication)

**`src-tauri/scripts/`:**
- Purpose: OS-specific helper scripts invoked as subprocesses
- Contains: Swift script for macOS Vision OCR
- Key files: `vision_ocr.swift` (28 lines, read-only image OCR)

**`ui/`:**
- Purpose: Single-page frontend application
- Contains: HTML, vanilla JS, CSS
- Key files: `app.js` (321 lines, stateful event loop), `index.html` (107 lines, DOM structure)

**`src-tauri/capabilities/`:**
- Purpose: Tauri security manifest
- Contains: JSON defining which OS APIs app can use
- Key files: `default.json` (allows clipboard, screenshot, window management)

**`src-tauri/gen/`:**
- Purpose: Auto-generated Tauri schemas and TypeScript types
- Contains: JSON schemas, capability definitions
- Generated: Yes (not edited manually)
- Committed: Yes (to git)

## Key File Locations

**Entry Points:**
- `src-tauri/src/main.rs`: Binary entry, calls `run()` from lib
- `src-tauri/src/lib.rs`: Tauri app initialization, command registration
- `ui/app.js`: Frontend initialization, `main()` function at line 36

**Configuration:**
- `src-tauri/tauri.conf.json`: Window size (1440x960), app identifier, bundling disabled
- `src-tauri/Cargo.toml`: Rust dependencies (tauri 2, image, base64, clipboard plugin)
- `ui/index.html`: `<link>` to styles, `<script type="module">` to app.js

**Core Logic:**
- `src-tauri/src/state.rs`: Session state machine (`SharedState`, `AppState`)
- `src-tauri/src/merge.rs`: Fuzzy deduplication algorithm (250+ lines with tests)
- `src-tauri/src/platform.rs`: Screenshot + OCR dispatch (236 lines with 3 OS implementations)

**Testing:**
- `src-tauri/src/merge.rs` lines 177-204: 2 unit tests (`dedupes_line_overlap`, `appends_sequentially_without_overlap`)
- No separate test files (tests colocated in modules)
- Run: `cargo test --manifest-path src-tauri/Cargo.toml`

## Naming Conventions

**Files:**
- Backend: `snake_case.rs` (e.g., `lib.rs`, `merge.rs`, `platform.rs`, `state.rs`)
- Frontend: `snake_case.js` (e.g., `app.js`) + `snake_case.css` (e.g., `styles.css`)
- Config: `kebab-case.json` (e.g., `tauri.conf.json`)

**Directories:**
- Backend: lowercase plural (e.g., `scripts/`, `capabilities/`, `gen/`)
- Frontend: lowercase singular (e.g., `ui/`)

**Functions (Rust):**
- Public: `snake_case` (e.g., `capture_snapshot()`, `append_text()`)
- Tauri commands: `snake_case` (e.g., `get_app_state()`, `commit_selection()`)
- Private: `snake_case` (e.g., `find_overlap()`, `canonical_line()`)

**Functions (JavaScript):**
- Top-level: `camelCase` (e.g., `captureSnapshot()`, `commitSelection()`)
- Private: `camelCase` (e.g., `toCanvasPoint()`, `normalizedRect()`)

**Types/Structs (Rust):**
- Public: `PascalCase` (e.g., `AppState`, `StoredSegment`, `MergeStrategy`)
- Serializable: `PascalCase` with `#[serde(rename_all = "camelCase")]` for JSON compat

**Variables (JavaScript):**
- Object keys: `camelCase` (e.g., `captureButton`, `appState`, `isDragging`)
- Local: `camelCase` (e.g., `snapshot`, `merged_text`, `segments`)

## Where to Add New Code

**New Feature (End-to-End):**
1. Backend command: Add `#[tauri::command]` fn in `src-tauri/src/lib.rs`, register in handler list
2. State mutation: If modifying session, add method to `AppState` in `src-tauri/src/state.rs`
3. Algorithm: If new dedup logic needed, add fn to `src-tauri/src/merge.rs` with unit tests
4. Platform integration: If new screenshot/OCR, add public fn to `src-tauri/src/platform.rs` with `#[cfg(...)]` blocks
5. Frontend handler: Add `async function` in `ui/app.js`, bind event listener in `bindEvents()`
6. UI: Add button/panel to `ui/index.html`, style in `ui/styles.css`

**New Component/Module (Rust):**
- Create `src-tauri/src/component_name.rs`
- Add `mod component_name;` at top of `src-tauri/src/lib.rs`
- Export public items, keep implementation private
- Add tests as `#[cfg(test)] mod tests { ... }` block at end of file

**Utilities:**
- Shared helpers: `src-tauri/src/lib.rs` (simple functions) or new `src-tauri/src/utils.rs` (if >50 lines)
- Frontend helpers: `ui/app.js` (currently monolithic, consider splitting if >500 lines)

## Special Directories

**`src-tauri/target/`:**
- Purpose: Build output directory for Rust compilation
- Generated: Yes (by `cargo build`)
- Committed: No (.gitignore excludes)

**`src-tauri/gen/`:**
- Purpose: Auto-generated Tauri type definitions and schemas
- Generated: Yes (by `tauri-build` crate)
- Committed: Yes (provides type safety, stable across rebuilds)

**`src-tauri/.venv/` or `venv/`:**
- Purpose: Python virtual environment (if used for build scripts)
- Generated: Yes
- Committed: No (git-ignored)

## Import Organization (Rust)

**Order in `src-tauri/src/lib.rs`:**
```rust
mod merge;              // 1. Internal modules
mod platform;
mod state;

use std::thread;        // 2. Standard library
use std::time::Duration;

use tauri::{State, WebviewWindow};  // 3. External crates
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::platform::{...};  // 4. Crate internals
use crate::state::{...};
```

**Order in `src-tauri/src/state.rs`:**
```rust
use std::sync::Mutex;   // 1. Standard library
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;  // 2. External crates
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::merge::{append_text, MergeStrategy};  // 3. Crate internals
```

## Build Artifacts and Outputs

**Executable:**
- Dev: Run via `cargo tauri dev` (Tauri handles hot-reload)
- Release: `target/release/just-fucking-copy` (after `cargo tauri build`)

**Frontend Distribution:**
- Configured in `tauri.conf.json` as `"frontendDist": "../ui"`
- All files in `ui/` are bundled into the app during `tauri build`

**Generated Types (TypeScript):**
- Located in `src-tauri/gen/` (auto-generated from Cargo.toml and Tauri manifest)
- Used internally by Tauri; frontend uses `__TAURI__.core.invoke()` (untyped)

## Module Dependencies (Rust)

```
lib.rs (commands)
  ├→ state.rs (AppState, SharedState)
  │   └→ merge.rs (append_text)
  ├→ platform.rs (capture_snapshot, crop_png, recognize_text_from_png)
  │   └→ image crate (PNG decode/encode)
  └→ merge.rs (MergeStrategy enum)

merge.rs (pure algorithm)
  └→ (no dependencies)

platform.rs (OS abstraction)
  ├→ image crate
  └→ std::process (subprocess)

state.rs (state machine)
  └→ merge.rs

main.rs (binary)
  └→ lib.rs (run function)
```

## Frontend Code Organization

**`ui/app.js` sections:**
1. Lines 1-2: Import Tauri invoke
2. Lines 3-19: DOM element cache
3. Lines 21-32: Context object (holds snapshot, image, selection, appState)
4. Lines 34-39: Canvas context + main() initialization
5. Lines 41-52: Event binding
6. Lines 54-144: Async command functions (invoke wrappers)
7. Lines 146-201: Render logic (DOM updates)
8. Lines 204-290: Canvas drawing + pointer event handlers
9. Lines 292-320: Utility functions (flash messages, HTML escaping)

## Conventions for Code Placement

**New Tauri Command:**
- Add to `src-tauri/src/lib.rs` at end of file before `pub fn run()`
- Decorate with `#[tauri::command]`
- Add to `invoke_handler![]` macro call in `run()`
- If modifying state, call methods on `state.inner.lock()` guard

**New State Method:**
- Add to `impl AppState` block in `src-tauri/src/state.rs`
- Call `rebuild_merge()` if state change affects merged text
- Return `AppStatePayload` if command-level visibility needed

**New Merge Logic:**
- Add pure function to `src-tauri/src/merge.rs`
- No side effects or mutable state
- Add `#[cfg(test)]` unit test in same module
- Export if called from `state.rs`, keep private if internal

**New Frontend Event Handler:**
- Add `async function` to `ui/app.js`
- Call `invoke()` for backend commands
- Call `refreshState()` or `render()` to sync UI
- Call `flash()` for user notifications
- Bind in `bindEvents()` function

---

*Structure analysis: 2026-03-20*
