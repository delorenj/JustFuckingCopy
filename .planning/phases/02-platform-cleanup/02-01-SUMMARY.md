---
phase: 02-platform-cleanup
plan: 01
subsystem: platform
tags: [rust, ocr, platform, cleanup, swift, tesseract]

# Dependency graph
requires:
  - phase: 01-ollama-http-module
    provides: ollama module registered in lib.rs, ready for wiring in Phase 3
provides:
  - platform.rs as pure screenshot+crop module with zero OCR code
  - vision_ocr.swift deleted
  - lib.rs import cleaned of recognize_text_from_png
  - single intentional compile error at commit_selection call site (Phase 3 target)
affects:
  - 03-wire-ollama-ocr (picks up the compile error and wires ollama::recognize_text)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Dead code removal: strip entire subsystem (OCR) before replacing it, leaving a clean break point (compile error) for the next phase to wire"

key-files:
  created: []
  modified:
    - src-tauri/src/platform.rs
    - src-tauri/src/lib.rs
  deleted:
    - src-tauri/scripts/vision_ocr.swift

key-decisions:
  - "Remove use std::io::Write from platform.rs: only used by OCR Swift stdin write, no longer needed after OCR removal"
  - "Leave recognize_text_from_png call site in commit_selection as intentional compile error: Phase 3 wires replacement"

patterns-established:
  - "Phase-boundary compile break: deliberately leave one error at the call site being replaced, so the next phase has a clear compiler-guided target"

requirements-completed: [CLN-01, CLN-02, CLN-03, CLN-04]

# Metrics
duration: 11min
completed: 2026-03-21
---

# Phase 02 Plan 01: Platform Cleanup Summary

**Stripped all legacy OCR code from platform.rs (Apple Vision Swift, Tesseract, Windows stub), deleted vision_ocr.swift, and left one intentional compile error at the commit_selection call site for Phase 3 to wire Ollama**

## Performance

- **Duration:** 11 min
- **Started:** 2026-03-21T04:56:56Z
- **Completed:** 2026-03-21T05:08:17Z
- **Tasks:** 2
- **Files modified:** 2 (platform.rs, lib.rs) + 1 deleted (vision_ocr.swift)

## Accomplishments
- Deleted `src-tauri/scripts/vision_ocr.swift` entirely
- Removed all OCR functions from `platform.rs`: `recognize_text_from_png`, three `recognize_text_from_file` implementations (macOS/Linux/Windows), `sanitize_ocr_output`, and the `VISION_OCR_SCRIPT` constant with its `include_str!`
- Removed `use std::io::Write` import (only used by OCR Swift stdin write)
- Cleaned `lib.rs` import to only bring in `capture_snapshot` and `crop_png` from platform
- Build confirms exactly one compile error: `E0425: cannot find function 'recognize_text_from_png'` at the `commit_selection` call site — intentional Phase 3 target

## Task Commits

Each task was committed atomically:

1. **Task 1: Delete vision_ocr.swift and strip OCR code from platform.rs** - `8220979` (refactor)
2. **Task 2: Remove recognize_text_from_png from lib.rs import** - `93c0cad` (refactor)

**Plan metadata:** (docs commit — see final_commit below)

## Files Created/Modified
- `src-tauri/src/platform.rs` - OCR functions removed; now exports only capture_snapshot, crop_png, png_dimensions, decode_png
- `src-tauri/src/lib.rs` - Import trimmed to `{capture_snapshot as platform_capture_snapshot, crop_png}`
- `src-tauri/scripts/vision_ocr.swift` - Deleted

## Decisions Made
- Removed `use std::io::Write` from platform.rs: it was exclusively used by the OCR Swift subprocess stdin write and is not needed by any remaining function
- Left the `recognize_text_from_png(&crop)?` call in `commit_selection` body intact as an intentional compile break; Phase 3 will replace it with `ollama::recognize_text`

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `platform.rs` is now a clean screenshot+crop module with no OCR concerns
- `lib.rs` has one compile error at line 94 (`recognize_text_from_png(&crop)?`) — this is the exact wiring point for Phase 3
- Phase 3 (03-wire-ollama-ocr) can proceed directly: replace the call with `ollama::recognize_text(&crop)?` and the codebase will compile clean

---
*Phase: 02-platform-cleanup*
*Completed: 2026-03-21*
