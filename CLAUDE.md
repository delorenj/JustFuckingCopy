# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

**JustFuckingCopy** is a Tauri 2 desktop app that captures a screenshot, lets you draw a marquee over text you want, OCRs it, and copies the merged result to the clipboard. The key innovation is intelligent deduplication of overlapping text across multiple captures using fuzzy line matching.

## Commands

```bash
# Run the app
cargo tauri dev

# Or directly:
cargo run --manifest-path src-tauri/Cargo.toml

# Run tests (merge algorithm unit tests)
cargo test --manifest-path src-tauri/Cargo.toml
```

macOS requires Screen Recording permission on first run. Linux requires `tesseract` installed for OCR.

## Architecture

The project is split into a Rust/Tauri backend (`src-tauri/`) and a static vanilla JS frontend (`ui/`). The frontend communicates with the backend exclusively via Tauri's `invoke()` IPC bridge.

### Backend Structure (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `lib.rs` | Tauri command handlers — the public API surface between frontend and backend |
| `state.rs` | `SharedState` (Mutex-wrapped `AppState`), holds snapshots, segments, and merged text |
| `merge.rs` | **Core algorithm** — fuzzy line deduplication with Levenshtein similarity |
| `platform.rs` | Pluggable screenshot capture and OCR backends per OS |

### Tauri Commands (frontend ↔ backend)

- `get_app_state()` — fetch current session state
- `capture_snapshot()` — hide window → screencapture → show window, returns image as base64 data URL
- `commit_selection(snapshotId, selectionRect)` — crop PNG → OCR → merge → update state
- `undo_last_segment()` — pop last OCR result and rebuild merge
- `reset_session()` — clear all state
- `copy_merged_text()` — write merged text to native clipboard

### Merge Algorithm (`merge.rs`)

The most important module. It uses fuzzy suffix-prefix overlap detection to deduplicate text when the same lines appear in consecutive captures:

1. Normalize text (line endings, trailing whitespace)
2. Convert lines to canonical form (lowercase, alphanumeric only)
3. Find longest suffix-prefix overlap using Levenshtein similarity
   - Threshold: **93%** for 1–2 line overlaps, **78%** for 3+ lines
4. Strategy applied: `Initial`, `OverlapDeduped`, or `SequentialAppend`

Tests in `merge.rs` cover the core dedup and sequential-append cases.

### Platform Backends (`platform.rs`)

**Screenshot:**
- macOS: `screencapture -x -t png`
- Linux: tries `grim` → `gnome-screenshot` → `import` (ImageMagick)
- Windows: PowerShell + `System.Drawing`

**OCR:**
- macOS: Swift subprocess running `src-tauri/scripts/vision_ocr.swift` (Apple Vision framework)
- Linux: `tesseract` CLI (`-l eng --psm 6`)
- Windows: stub (not yet implemented)

### Frontend (`ui/`)

Vanilla JS with no framework. State mirrors backend `AppState`. Key flow: Capture → draw marquee on canvas (pointer events) → Commit → results appear in timeline and merged textarea → Copy.

## Key Design Decisions

- **Backend-agnostic merge layer:** `merge.rs` is pure logic with no platform dependencies — easy to unit test.
- **OCR is a subprocess:** Vision framework (Swift) and Tesseract are invoked as child processes, not linked as libraries, keeping build complexity low.
- **No bundling yet:** `bundle.active = false` in `tauri.conf.json` — run only via `cargo tauri dev`.
- **Image data stays in memory:** Screenshots are passed as base64-encoded PNG data URLs; no persistent temp files after each operation completes.

<!-- GSD:project-start source:PROJECT.md -->
## Project

**JustFuckingCopy**

A Tauri 2 desktop app that captures screenshots, lets you draw a marquee over text regions, OCRs the selected area, and copies the merged result to the clipboard. Multiple captures are intelligently deduplicated using fuzzy line matching so you can grab text across overlapping regions without duplicates.

**Core Value:** Capture visible text from any screen region and get clean, deduplicated clipboard content in as few clicks as possible.

### Constraints

- **Network**: Ollama must be reachable at 192.168.1.12 for OCR to function
- **Model**: GLM-OCR model must be loaded in the Ollama instance
- **Tech stack**: Rust/Tauri backend, vanilla JS frontend (no changes to frontend stack)
- **Scope**: Only the OCR pipeline changes; screenshot capture and merge logic stay as-is
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2021 edition - Backend application logic, OCR integration, state management, merge algorithm
- JavaScript (Vanilla) - Frontend UI, no frameworks or build tools
- Swift - macOS OCR subprocess (`src-tauri/scripts/vision_ocr.swift`)
- PowerShell - Windows screenshot capture via System.Drawing
- Shell scripts - Linux screenshot backend fallback logic
## Runtime
- Tauri 2 - Desktop application framework bridging Rust backend with web frontend
- Desktop platforms: macOS, Linux, Windows
- Cargo (Rust) - Manages Rust dependencies
- No npm/Node.js in main application (Tauri handles frontend bundling)
## Frameworks
- Tauri 2 - Desktop app framework with IPC bridge between Rust and JavaScript
- tauri-plugin-clipboard-manager 2 - Native clipboard read/write operations
- tauri-build 2 - Build-time configuration and context generation for Tauri
## Key Dependencies
- `tauri` 2 - Core framework for window management, command routing, asset serving
- `tauri-plugin-clipboard-manager` 2 - Write merged text to native clipboard via `app.clipboard().write_text()`
- `image` 0.25 - PNG decoding/encoding, image cropping, dimension queries
- `serde` 1 - JSON serialization/deserialization for command arguments and state payloads
- `base64` 0.22 - Encodes PNG bytes as data URLs for frontend display
- `objc2`, `objc2-app-kit`, `objc2-foundation`, `objc2-core-graphics` - macOS Objective-C bindings (transitive via `tauri`)
- `windows-sys` 0.60.2 - Windows API bindings for PowerShell screenshot (transitive via `tauri`)
## Configuration
- No environment variables required for runtime
- macOS requires Screen Recording permission (prompted on first `capture_snapshot()`)
- Linux requires `tesseract` OCR binary in PATH
- Windows OCR backend not yet implemented
- `src-tauri/tauri.conf.json` - Tauri configuration schema
- `src-tauri/Cargo.toml` - Rust dependencies and crate configuration
## Platform Requirements
- Rust toolchain with Tauri 2 support
- macOS: Xcode Command Line Tools (for native compilation)
- Linux: `tesseract` OCR binary, `grim`/`gnome-screenshot`/`import` for screenshot capture
- Windows: PowerShell (built-in)
- macOS: Native code signed and notarized (bundling not yet active)
- Linux: `tesseract` binary must be installed before running
- Windows: .NET/Windows.Forms assembly available (PowerShell screenshot)
## Deployment
- No bundling active (`bundle.active = false` in `tauri.conf.json`)
- Run via `cargo tauri dev` for development
- Production bundling will target OS-native installers once enabled
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Rust module files: lowercase with underscores (`lib.rs`, `merge.rs`, `platform.rs`, `state.rs`, `main.rs`)
- JavaScript files: camelCase with `.js` extension (`app.js`)
- CSS files: lowercase with `.css` extension (`styles.css`)
- HTML files: lowercase with `.html` extension (`index.html`)
- **Rust:** snake_case for all functions
- **JavaScript:** camelCase for all functions
- **Rust:** snake_case for locals and struct fields
- **JavaScript:** camelCase for all variables
- **Rust:** PascalCase for all types (enums, structs, traits)
- **JavaScript:** No explicit type definitions; inferred from context and usage
## Code Style
- **Rust:** Standard Rust formatting conventions (implied via `cargo fmt`, not explicitly configured)
- **JavaScript:** Inferred formatting conventions (no formatter detected in repo)
- No ESLint, Prettier, or equivalent JavaScript configuration detected
- No explicit Rust linting configuration beyond standard `cargo clippy` warnings
- Code appears hand-formatted and linted against conventions
## Import Organization
- Single import at top for Tauri bridge: `const invoke = window.__TAURI__.core.invoke;`
- No modularization; single-file architecture
- No path aliases or explicit import statements beyond Tauri
## Error Handling
- **Rust:** Pervasive use of `Result<T, String>` type for fallible operations
- **JavaScript:** Try-catch blocks with frontend error messaging via `flash()` helper
## Logging
- **Rust:** No `println!`, `dbg!`, or logging crate used
- **JavaScript:** No logging library; all user feedback via `flash()` utility function
## Comments
- Comments are minimal throughout the codebase
- Self-documenting code is preferred (descriptive function/variable names)
- No JSDoc or RustDoc comments found in active code
- **Rust:**
- **JavaScript:**
## Function Design
- **Rust:** Public API functions in `lib.rs` are typically 10-20 lines (focused on request → command → response)
- **JavaScript:** Frontend async functions typically 15-25 lines (try-catch-finally wrapping)
- Helpers and pure logic functions (merge algorithm, geometry, UI rendering) range 5-30 lines
- **Rust:**
- **JavaScript:**
- **Rust:**
- **JavaScript:**
## Module Design
- **Rust:**
- **JavaScript:**
- Not used (codebase is too small and modular)
- Each Rust file (`merge.rs`, `platform.rs`, `state.rs`) is self-contained
- No `mod.rs` or `__init__.py` pattern
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Frontend-backend separation via Tauri `invoke()` IPC (all communication serialized as JSON)
- Backend maintains single centralized `SharedState` (Mutex-wrapped `AppState`) for all session data
- Pure functional merge algorithm decoupled from platform/UI concerns
- Pluggable platform backends for screenshot and OCR (compile-time feature selection)
- Stateless request/response pattern (backend commands return full `AppStatePayload` after each mutation)
## Layers
- Purpose: Expose Tauri commands as the sole interface between frontend and backend
- Location: `src-tauri/src/lib.rs`
- Contains: 6 command functions decorated with `#[tauri::command]`
- Depends on: `state`, `platform`, `merge` modules
- Used by: Frontend via `invoke("command_name")`
- Examples: `capture_snapshot`, `commit_selection`, `get_app_state`, `copy_merged_text`
- Purpose: Hold and mutate session state; orchestrate merge algorithm on state changes
- Location: `src-tauri/src/state.rs`
- Contains: `SharedState` (Mutex wrapper), `AppState` (inner mutable state), serializable payloads
- Depends on: `merge` module for algorithm execution
- Used by: IPC command handlers
- Key methods: `push_segment()` (adds OCR result and triggers `rebuild_merge()`), `undo_last_segment()`, `store_snapshot()`
- Purpose: Fuzzy deduplication of overlapping text using Levenshtein similarity
- Location: `src-tauri/src/merge.rs`
- Contains: Pure functions with no side effects
- Depends on: Nothing (no other modules)
- Used by: `AppState.rebuild_merge()` to compute merged text
- Key function: `append_text(existing: &str, incoming: &str) -> MergeOutcome`
- Algorithm: Longest suffix-prefix overlap detection with thresholds (93% for 1-2 lines, 78% for 3+ lines)
- Purpose: Hide OS-specific screenshot capture and OCR behind a unified interface
- Location: `src-tauri/src/platform.rs`
- Contains: `capture_snapshot()`, `crop_png()`, `recognize_text_from_png()` (public); OS-specific private fns
- Depends on: `image` crate for PNG decoding/encoding; platform CLIs (screencapture, tesseract, powershell, swift)
- Used by: IPC command handlers (`capture_snapshot`, `commit_selection`)
- Compile-time selection: `#[cfg(target_os = "...")]` blocks define macOS, Linux, Windows implementations
- Purpose: Render app state, handle user interactions, invoke backend commands
- Location: `ui/` (vanilla JS, no framework)
- Contains: Single-page HTML + JS, CSS styles
- Depends on: Tauri global `__TAURI__.core.invoke` for backend communication
- Used by: User (GUI interactions)
## Data Flow
- All state mutations are deterministic: command receives request, acquires lock on `SharedState.inner`, mutates `AppState`, releases lock, returns new full state
- Each command call includes potential race condition check (e.g., snapshot ID validation in `commit_selection`)
## Key Abstractions
- Purpose: Single source of truth for entire session
- Examples: `src-tauri/src/state.rs` lines 11-22
- Pattern: Mutex protects inner `AppState` from concurrent access
- Fields:
- Purpose: Captures immutable record of one OCR operation
- Examples: `src-tauri/src/state.rs` lines 62-71
- Fields: `id`, `order`, `snapshot_id`, `selection` (rect), `recognized_text`, `merge_strategy`, `overlap_lines`, `created_at_epoch_ms`
- Serialized to frontend as `SegmentPayload` (identical structure, JSON-compatible)
- Purpose: Tag each segment with how it was merged
- Examples: `src-tauri/src/merge.rs` lines 1-17
- Variants:
- Purpose: Same function signatures, different implementations per OS
- Examples: `recognize_text_from_file()` in `src-tauri/src/platform.rs` has 3 implementations (lines 67-97 for macOS, 100-123 for Linux, 126-128 for Windows stub)
- Pattern: `#[cfg(target_os = "...")]` attributes on functions with identical signatures
## Entry Points
- Location: `src-tauri/src/main.rs` line 4
- Triggers: Binary execution (`cargo tauri dev` or app launch)
- Responsibilities: Calls `just_fucking_copy_lib::run()`
- Location: `src-tauri/src/lib.rs` lines 141-155
- Triggers: Called from `main.rs`
- Responsibilities:
- Location: `ui/app.js` lines 36-39 and 318-320
- Triggers: HTML loads script, `main()` called
- Responsibilities:
## Error Handling
## Cross-Cutting Concerns
- **OCR text**: Checked non-empty after recognition
- **Selection rect**: Checked non-zero dimensions on canvas (line 264 in `app.js`)
- **Snapshot ID**: Validated in `commit_selection` to prevent stale selections
- **PNG bounds**: Checked in `crop_png()` to prevent out-of-bounds crops
- Frontend: All Tauri `invoke()` calls are `async` (return Promise)
- Backend: All commands are sync (acquire lock, compute, release)
- No background tasks or channels (simple request/response model)
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
