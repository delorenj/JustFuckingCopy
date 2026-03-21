# JustFuckingCopy

## What This Is

A Tauri 2 desktop app that captures screenshots, lets you draw a marquee over text regions, OCRs the selected area, and copies the merged result to the clipboard. Multiple captures are intelligently deduplicated using fuzzy line matching so you can grab text across overlapping regions without duplicates.

## Core Value

Capture visible text from any screen region and get clean, deduplicated clipboard content in as few clicks as possible.

## Requirements

### Validated

- ✓ User can capture a full-screen screenshot via hotkey/button — existing
- ✓ User can draw a marquee rectangle over a screenshot to select a text region — existing
- ✓ User can commit a selection to OCR the cropped region and add it to the session — existing
- ✓ Multiple overlapping captures are merged with fuzzy line deduplication (Levenshtein similarity) — existing
- ✓ User can copy the merged text to the native clipboard — existing
- ✓ User can undo the last segment and rebuild the merge — existing
- ✓ User can reset the session to start fresh — existing
- ✓ Platform-specific screenshot capture works on macOS (screencapture), Linux (grim/gnome-screenshot/import) — existing

### Active

- [ ] Replace all platform-specific OCR backends with a single Ollama GLM-OCR HTTP call
- [ ] OCR requests hit Ollama at 192.168.1.12 (hardcoded endpoint)
- [ ] Hard fail with clear error message if Ollama is unreachable
- [ ] Remove Apple Vision OCR Swift script and all references
- [ ] Remove Tesseract OCR integration and all references
- [ ] Remove Windows OCR stub and all references
- [ ] Merge/dedup algorithm continues to work identically with Ollama-sourced text

### Out of Scope

- Unifying screenshot capture across platforms — keep existing per-OS backends, only OCR changes
- Configurable Ollama endpoint — hardcoded is fine for now, revisit if needed
- Fallback to local OCR — if Ollama is down, the app errors rather than degrading
- Windows OCR implementation — was a stub anyway, removing it
- App bundling/distribution — `bundle.active = false`, dev mode only
- Mobile or web support — desktop only

## Context

- Existing Tauri 2 app with Rust backend and vanilla JS frontend
- OCR is currently a subprocess call: Swift Vision framework on macOS, Tesseract CLI on Linux, stub on Windows
- The merge algorithm in `merge.rs` is pure logic with no platform dependencies and remains untouched
- Ollama instance running GLM-OCR is available on the local network at 192.168.1.12
- The OCR swap is isolated to `platform.rs` and `lib.rs` (the command handler that calls it)

## Constraints

- **Network**: Ollama must be reachable at 192.168.1.12 for OCR to function
- **Model**: GLM-OCR model must be loaded in the Ollama instance
- **Tech stack**: Rust/Tauri backend, vanilla JS frontend (no changes to frontend stack)
- **Scope**: Only the OCR pipeline changes; screenshot capture and merge logic stay as-is

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Ollama over local OCR | Consistent quality across platforms, better results than Vision/Tesseract, simpler single code path | — Pending |
| Hardcoded endpoint | Simplicity; only one Ollama instance exists on the network | — Pending |
| Hard fail on unreachable | No degraded experience; if OCR can't run, tell the user clearly | — Pending |
| Delete old OCR code | Clean codebase, no dead code or feature flags for deprecated paths | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? Move to Out of Scope with reason
2. Requirements validated? Move to Validated with phase reference
3. New requirements emerged? Add to Active
4. Decisions to log? Add to Key Decisions
5. "What This Is" still accurate? Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check: still the right priority?
3. Audit Out of Scope: reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-20 after initialization*
