# Phase 2: Platform Cleanup - Context

**Gathered:** 2026-03-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Delete all legacy platform-specific OCR code. After this phase, `platform.rs` contains only screenshot capture and `crop_png`. No Apple Vision, Tesseract, or Windows OCR code remains anywhere in the repo.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion — pure cleanup/deletion phase.
- Delete `src-tauri/scripts/vision_ocr.swift` entirely
- Remove all `#[cfg(target_os)]` blocks related to OCR in `platform.rs`
- Remove `recognize_text_from_png` and `recognize_text_from_file` functions from `platform.rs`
- Keep screenshot capture functions (`capture_screenshot`, `crop_png`) intact
- Keep `sanitize_ocr_output` if it still exists in `platform.rs` (it was copied to `ollama.rs` in Phase 1)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ollama.rs` (from Phase 1) now handles all OCR with its own `sanitize_ocr_output`

### Established Patterns
- `platform.rs` uses `#[cfg(target_os = "...")]` for platform-specific code
- Three OCR implementations: macOS (Swift subprocess), Linux (tesseract CLI), Windows (stub)

### Integration Points
- `lib.rs` currently calls `platform::recognize_text_from_png()` in `commit_selection` — this will break until Phase 3 rewires to `ollama::recognize_text()`
- That's expected: Phase 2 is cleanup, Phase 3 wires it back together

</code_context>

<specifics>
## Specific Ideas

No specific requirements — pure deletion phase.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
