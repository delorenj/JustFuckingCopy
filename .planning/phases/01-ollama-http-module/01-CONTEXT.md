# Phase 1: Ollama HTTP Module - Context

**Gathered:** 2026-03-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Create a new `ollama.rs` module that handles all Ollama HTTP communication for OCR. This phase delivers the HTTP client, request construction, response parsing, image preprocessing (resize + base64), and error handling. The module is independently testable with no Tauri dependencies.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion — pure infrastructure phase. Key constraints from research:
- Use `reqwest 0.12` with `features = ["json", "rustls-tls"]`
- Use `/api/generate` endpoint (NOT `/api/chat` or OpenAI-compat)
- Raw base64 in `images` array (no `data:` URI prefix)
- `num_ctx: 16384` in every request
- `stream: false` with 60s timeout
- Prompt: `"Text Recognition:"`
- Image dimension cap at 2048px before encoding

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `base64` crate (0.22) already in `Cargo.toml` for PNG encoding
- `image` crate (0.25) already available for resize operations
- `serde` already in dependencies for JSON serialization

### Established Patterns
- `platform.rs` currently has `recognize_text_from_png(png_bytes: &[u8]) -> Result<String, String>` signature
- Error handling uses `Result<T, String>` throughout
- State payloads serialized with serde

### Integration Points
- New `ollama.rs` will be called from `platform.rs` (replacing old OCR) or directly from `lib.rs`
- Must return `Result<String, String>` to match existing error handling pattern

</code_context>

<specifics>
## Specific Ideas

No specific requirements — infrastructure phase. Ollama endpoint hardcoded to `192.168.1.12:11434`.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
