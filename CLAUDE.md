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
