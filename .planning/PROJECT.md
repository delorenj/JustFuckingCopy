# JustFuckingCopy

## What This Is

An ambient system tray app that watches a directory for screenshots, OCRs them in batch via Ollama GLM-OCR, deduplicates overlapping text with fuzzy line matching, and copies the merged result to the clipboard on a global hotkey. No windows, no marquee selection, no friction.

## Core Value

Capture visible text from any screen region and get clean, deduplicated clipboard content with zero workflow interruption.

## Current Milestone: v2.0 Ambient Tray

**Goal:** Transform from modal window app to ambient system tray app with directory watcher and global hotkey.

**Target features:**
- System tray with badge count as primary UI
- Directory watcher for automatic screenshot detection
- Global hotkey to trigger batch OCR + merge + clipboard copy
- Status panel (left-click tray) showing batch contents and merged preview
- Archive + clear batch lifecycle after processing
- Config via TOML file (no GUI settings)

## Requirements

### Validated

- ✓ User can capture a full-screen screenshot via hotkey/button — v1.0
- ✓ User can draw a marquee rectangle over a screenshot to select a text region — v1.0
- ✓ User can commit a selection to OCR the cropped region and add it to the session — v1.0
- ✓ Multiple overlapping captures are merged with fuzzy line deduplication (Levenshtein similarity) — v1.0
- ✓ User can copy the merged text to the native clipboard — v1.0
- ✓ User can undo the last segment and rebuild the merge — v1.0
- ✓ User can reset the session to start fresh — v1.0
- ✓ Platform-specific screenshot capture works on macOS/Linux — v1.0
- ✓ OCR via Ollama GLM-OCR HTTP call replaces all platform-specific backends — v1.0
- ✓ Hard fail with clear error if Ollama is unreachable — v1.0
- ✓ Apple Vision, Tesseract, and Windows OCR stubs removed — v1.0

### Active

- [ ] App runs as system tray icon with badge count showing pending screenshots
- [ ] Directory watcher detects new screenshots in configurable watched directory
- [ ] Badge count increments per new screenshot detected in batch
- [ ] Global hotkey (default Ctrl+Shift+C, configurable) triggers batch OCR + merge + clipboard
- [ ] Full screenshots sent to Ollama GLM-OCR (no marquee cropping)
- [ ] Batch results archived to subdirectory after processing, badge resets
- [ ] Left-click tray icon opens status panel with batch contents and merged text preview
- [ ] Status panel has action buttons (process now, clear batch)
- [ ] Configuration via ~/.config/justfuckingcopy/config.toml (watch dir, hotkey, Ollama endpoint)
- [ ] No main window on launch; tray-only by default

### Out of Scope

- Settings GUI window — deferred to v2.1+, config.toml only for v2.0
- In-app screenshot capture — users use their OS screenshot tools
- Marquee selection — full screenshots with dedup handles overlap
- Clipboard history panel — future feature, not v2.0
- Windows support — Linux/macOS only for now
- App bundling/distribution — `bundle.active = false`, dev mode only
- Mobile or web support — desktop only
- Fallback to local OCR — if Ollama is down, the app errors

## Context

- Existing Tauri 2 app with Rust backend and vanilla JS frontend
- v1.0 shipped: Ollama GLM-OCR integration, fuzzy dedup merge, async command patterns
- `ollama.rs` (HTTP client), `merge.rs` (fuzzy dedup algorithm) carry forward unchanged
- `platform.rs` now contains only screenshot capture and `crop_png` after v1.0 cleanup
- v2.0 is a paradigm shift: modal window with marquee → ambient tray with directory watcher
- Watch directory default: ~/data/ssbnk/hosted (already exists for user's screenshot workflow)
- Ollama instance running GLM-OCR at 192.168.1.12 on local network

## Constraints

- **Network**: Ollama must be reachable at 192.168.1.12 for OCR to function
- **Model**: GLM-OCR model must be loaded in the Ollama instance
- **Tech stack**: Rust/Tauri 2 backend, vanilla JS frontend (no framework changes)
- **Tray API**: Tauri 2 system tray plugin required for tray icon and menu
- **Hotkey**: Global hotkey registration requires OS-level APIs (Tauri plugin or direct)
- **File watching**: Must handle rapid successive writes (screenshot tools write then rename)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Ollama over local OCR | Consistent quality across platforms, simpler single code path | ✓ Good |
| Hardcoded endpoint | Simplicity; only one Ollama instance exists on the network | ✓ Good (now configurable via TOML) |
| Hard fail on unreachable | No degraded experience; if OCR can't run, tell the user clearly | ✓ Good |
| Delete old OCR code | Clean codebase, no dead code | ✓ Good |
| Ambient tray over modal window | Eliminates friction of window blocking screenshots and forcing in-app capture | — Pending |
| Archive + clear batch lifecycle | Preserves history on disk while auto-resetting for next batch | — Pending |
| Config.toml over settings GUI | Keep v2.0 scope tight; GUI settings deferred to v2.1+ | — Pending |
| Full screenshots over marquee crop | Dedup algorithm handles overlap; removes need for precise selection | — Pending |

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
*Last updated: 2026-03-21 after v2.0 milestone initialization*
