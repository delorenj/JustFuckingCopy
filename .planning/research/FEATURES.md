# Feature Landscape: Ollama Vision OCR Integration

**Domain:** Local LLM-based OCR via Ollama HTTP API, integrated into an existing Tauri 2 desktop app
**Researched:** 2026-03-20
**Scope:** Milestone — replace all platform-specific OCR backends (Apple Vision Swift, Tesseract, Windows stub) with a single Ollama GLM-OCR HTTP call

---

## Table Stakes

Features that must exist or the OCR pipeline is broken or worse than the current implementation.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Base64 PNG sent to Ollama `/api/chat` | Core mechanic — Ollama vision API accepts base64 images in the `images` field of the chat message array | Low | POST to `http://192.168.1.12:11434/api/chat`; `stream: false`; `model: "glm-ocr"` |
| `num_ctx` set to at least 16384 in every request | GLM-OCR silently crashes or returns empty/garbled output with Ollama's default 4096-token context — this is a confirmed production bug, not an edge case | Low | Must be passed as `options.num_ctx` in the request body; missing this breaks all non-trivial images |
| GLM-OCR task prefix prompt `"Text Recognition:"` | GLM-OCR is a task-directed model; without the correct task prefix, output is undefined or empty | Low | The model's documented prompt protocol; plain `"Extract text"` is NOT equivalent |
| Plain text output extraction (strip model preamble) | GLM-OCR may emit Markdown tags or conversational wrapper text; `merge.rs` receives raw strings and expects clean line-delimited text | Low | Strip any leading `"Sure, here is..."` or trailing confidence commentary before passing to `push_segment()` |
| Hard fail with clear error message when Ollama is unreachable | Explicitly in project requirements; no degraded experience permitted; user must know exactly why OCR failed | Low | Distinguish connection refused (service down) from model-not-found (404) from timeout |
| Empty OCR result validation | Existing behavior preserved: if model returns whitespace-only text, error propagates to frontend as `"OCR returned no text"` | Low | Already implemented pattern in `platform.rs`; new backend must replicate it |
| Remove Apple Vision Swift backend entirely | Dead code with no callers; macOS CI would need Xcode toolchain; project decision is to delete, not gate | Low | Delete `src-tauri/scripts/vision_ocr.swift` and macOS `#[cfg]` block |
| Remove Tesseract backend entirely | Same as above; new backend is OS-agnostic; keeping Tesseract creates false impression of fallback | Low | Delete Linux `#[cfg]` block and any Tesseract process-spawn code |
| Remove Windows stub entirely | Windows was never implemented; stub adds noise without value | Low | Delete Windows `#[cfg]` block |
| Merge algorithm untouched | `merge.rs` is pure text logic; its contract is `(existing: &str, incoming: &str) -> MergeOutcome`; OCR source is irrelevant to it | None | Zero changes required in `merge.rs` or `state.rs` |

---

## Differentiators

Features that go beyond the minimum viable swap but provide genuine value. All are **optional for this milestone** — include only if they don't increase scope risk.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Image downscale before sending to Ollama | GLM-OCR via Ollama has a confirmed bug processing images larger than ~1800px in either dimension; cropped marquee regions are typically small but full screenshots can be large; pre-downscaling prevents silent failures | Low-Med | Use the existing `image` crate already in `platform.rs`; resize to max 1800px before base64 encoding; document the threshold |
| Structured error classification in Tauri command response | Surface distinct error categories to frontend: `OllamaUnreachable`, `ModelNotLoaded`, `OcrEmpty`, `OcrTimeout` — rather than raw string errors | Medium | Enables future UI differentiation (e.g. "Check your Ollama instance" vs "Try a tighter selection"); not in scope for initial phase |
| Request timeout configured explicitly | Ollama vision inference on local network over WiFi can stall; a 30–60 second explicit `reqwest` timeout prevents the UI from hanging indefinitely | Low | `reqwest::ClientBuilder::timeout(Duration::from_secs(45))` is a single line; include it |
| Log OCR round-trip timing to stderr | GLM-OCR 0.9B is fast but network + inference latency is variable; debug output helps distinguish "slow OCR" from "connection issues" | Low | `eprintln!` with elapsed ms; no dependency addition; zero UI impact |

---

## Anti-Features

Things to deliberately NOT build in this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Configurable Ollama endpoint in UI settings | Adds state, UI complexity, validation logic, and persistence (config file or registry) for a single-user tool with one known Ollama instance | Hardcode `http://192.168.1.12:11434`; revisit only if a second Ollama host becomes necessary |
| Fallback to Tesseract or Apple Vision if Ollama fails | Defeats the purpose of the migration; creates two parallel code paths to maintain; masks Ollama connectivity problems | Hard fail with clear error; let the user fix the Ollama instance |
| Retry loop with exponential backoff | Ollama is on a local LAN — if it's unreachable it's genuinely down, not transiently unavailable; retrying adds latency and hides the real problem from the user | Fail fast; single attempt; surface the error immediately |
| Streaming response consumption (`stream: true`) | Adds response accumulation complexity for no UX benefit — the user sees no partial text during OCR; the result is only used after full completion | Set `stream: false`; read the complete JSON response body |
| Multi-model routing or model selection UI | GLM-OCR is the specified model; building model selection is premature generalization | Hardcode `glm-ocr` as the model name; it can become a constant, not a UI setting |
| Output format variants (Markdown, JSON, LaTeX) | GLM-OCR supports `Table Recognition:` and `Formula Recognition:` modes; these produce non-plain-text output that `merge.rs` cannot process | Use `Text Recognition:` exclusively; other modes require a different downstream pipeline that does not exist |
| Client-side caching of OCR results | Each marquee selection is unique pixel data; caching would require hashing image regions, storing results, and invalidation logic | Not worth the complexity for a desktop app with session-scoped state |
| Health check / ping endpoint polling | A pre-flight `GET /api/tags` check before each OCR call adds latency and a second failure point | Let the OCR call itself fail and surface the error; one network round-trip, not two |

---

## Feature Dependencies

```
Remove old OCR backends  →  New Ollama HTTP OCR function
New Ollama HTTP OCR function  →  num_ctx option in every request
New Ollama HTTP OCR function  →  "Text Recognition:" task prefix
New Ollama HTTP OCR function  →  Response text stripping (preamble/postamble)
New Ollama HTTP OCR function  →  Empty-text validation (existing pattern)
Image downscale (optional)  →  New Ollama HTTP OCR function
Request timeout  →  New Ollama HTTP OCR function
```

`merge.rs` and `state.rs` have no dependencies on OCR implementation — they consume `String` and are unaffected by this change.

---

## MVP Recommendation

The minimal correct implementation for this milestone is:

1. **Delete all three old OCR backends** — Swift, Tesseract, Windows stub; no `#[cfg]` gates, no dead code
2. **Single new `recognize_text_from_png()` implementation** in `platform.rs`:
   - Base64-encode the PNG bytes
   - POST to `http://192.168.1.12:11434/api/chat` with `model: "glm-ocr"`, `stream: false`, task prompt `"Text Recognition:"`, image in `messages[0].images`, and `options.num_ctx: 16384`
   - Set explicit `reqwest` timeout (45 seconds)
   - Strip any conversational wrapper from the response content field
   - Validate non-empty result
   - Return `Err(String)` with meaningful message on any HTTP error or connection failure
3. **Image resize guard** — resize PNG to max 1800px before encoding; prevents the confirmed GLM-OCR image-size bug

Defer: Structured error classification, multi-error-type enum, log timing. Include but do not prioritize.

---

## Known Constraints From Research

- **GLM-OCR requires `num_ctx >= 16384`** — confirmed community-wide issue; default Ollama context of 4096 causes empty/garbled output (HIGH confidence — multiple independent reports)
- **GLM-OCR image size limit ~1800px** — Ollama issue #14114; images larger than ~1800px in either dimension cause `"failed to fully read image"` errors (HIGH confidence — confirmed GitHub issue)
- **`stream: false` with long inference can return 500** — seen in some Ollama versions for very long completions; the 45s timeout mitigates this; GLM-OCR at 0.9B is fast enough that this should not be triggered for marquee-sized regions (MEDIUM confidence)
- **GLM-OCR version instability in Ollama** — issues #14117, #14296, #14494, #14498 indicate the Ollama integration had loading failures in versions 0.15.6–0.17.4; pin to a confirmed working Ollama version in setup notes (MEDIUM confidence — issues are open but may be resolved)
- **Task prefix is required** — `"Text Recognition:"` is not a suggestion; it is the model's dispatch mechanism; other prompts produce unpredictable output (HIGH confidence — documented in GLM-OCR README and Ollama model page)

---

## Sources

- [GLM-OCR Ollama library page](https://ollama.com/library/glm-ocr) — model overview, task prefix documentation
- [GLM-OCR GitHub README](https://github.com/zai-org/GLM-OCR) — authoritative prompt format, architecture
- [GLM-OCR HuggingFace: No text output with Ollama (discussion #8)](https://huggingface.co/zai-org/GLM-OCR/discussions/8) — `num_ctx` fix confirmed
- [Ollama issue #14117: glm-ocr failed to fully read image](https://github.com/ollama/ollama/issues/14117) — image size bug
- [Ollama issue #14114: glm-ocr cannot process images > 2048x2048](https://github.com/ollama/ollama/issues/14114) — image size limit
- [Ollama Vision documentation](https://docs.ollama.com/capabilities/vision) — API request format, base64 encoding
- [Build OCR System with Ollama Vision Models — Markaicode](https://markaicode.com/build-ocr-system-ollama-vision-models/) — practical integration patterns
- [GLM-OCR Technical Report](https://arxiv.org/abs/2603.10910) — model architecture and benchmark claims
- [GLM-OCR Medium: 0.9B model overview](https://medium.com/@gsaidheeraj/glm-ocr-the-tiny-0-9b-vision-language-model-that-reads-documents-like-a-human-e79c458319cc) — capabilities summary
- [Ollama-RS Rust crate](https://github.com/pepperoni21/ollama-rs) — Rust integration patterns
- [HalluText: OCR Hallucination in LVLMs](https://openreview.net/forum?id=LRnt6foJ3q) — failure mode research
