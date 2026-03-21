# Technology Stack

**Project:** JustFuckingCopy — Ollama GLM-OCR Integration
**Researched:** 2026-03-20
**Overall confidence:** HIGH for HTTP client selection; MEDIUM for GLM-OCR API specifics (official docs confirmed, some behavior details from community reports)

---

## Context

This milestone replaces three platform-specific OCR backends (Apple Vision Swift subprocess, Tesseract CLI subprocess, Windows stub) with a single async HTTP call to an Ollama instance running `glm-ocr` at `192.168.1.12:11434`. The change is isolated to `platform.rs` and the `recognize_text_from_png` function. All other code is untouched.

---

## Recommended Stack

### New Dependencies to Add

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| `reqwest` | `0.12` | Async HTTP client — POST to Ollama | Industry-standard Rust HTTP client. Tokio-native. The `0.12` series (latest: 0.12.24) is the stable production line; `0.13` is very new with TLS-backend breaking changes not worth chasing. Use `default-features = false, features = ["json", "rustls-tls"]` to keep binary lean and avoid native-tls link complexity. |
| `tokio` | `1` | Async runtime | Tauri 2 already embeds a Tokio 1 runtime. Add it as a direct dependency with `features = ["rt"]` so `#[tokio::test]` works in tests. Do NOT add `#[tokio::main]` — Tauri owns the runtime entry point. |
| `serde_json` | `1` | Build Ollama request payloads and parse responses | Already pulled transitively via `serde`; make it explicit. The `json!()` macro makes request construction readable and avoids manual struct definitions for a one-off API shape. |

### Libraries to Remove

| Library / Code | Why Remove |
|----------------|-----------|
| `src-tauri/scripts/vision_ocr.swift` | Apple Vision OCR subprocess — replaced entirely |
| `#[cfg(target_os = "macos")]` OCR block in `platform.rs` | Dead code after migration |
| `#[cfg(target_os = "linux")]` OCR block in `platform.rs` | Dead code — Tesseract subprocess gone |
| `#[cfg(target_os = "windows")]` OCR block in `platform.rs` | Stub — remove cleanly |
| Linux `tesseract` binary runtime requirement | No longer needed |

### Existing Dependencies That Stay

| Library | Version | Stays Because |
|---------|---------|---------------|
| `image` | 0.25 | Still needed for `crop_png` — the crop step produces the PNG bytes we send to Ollama |
| `base64` | 0.22 | Still needed for frontend data-URL encoding; also used to encode PNG bytes for Ollama request body |
| `serde` | 1 | Unchanged |
| `tauri` | 2 | Unchanged |
| `tauri-plugin-clipboard-manager` | 2 | Unchanged |

---

## Cargo.toml Changes

```toml
[dependencies]
base64 = "0.22"
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["rt"] }
tauri = { version = "2" }
tauri-plugin-clipboard-manager = "2"
```

---

## Ollama API Contract

### Endpoint

```
POST http://192.168.1.12:11434/api/generate
Content-Type: application/json
```

Use `/api/generate`, not `/api/chat`. The GLM-OCR authors explicitly recommend the native generate endpoint over the OpenAI-compatible chat endpoint for vision requests due to limitations in Ollama's OpenAI-compat layer for image handling.

**Confidence:** MEDIUM — sourced from zai-org/GLM-OCR official repo README and confirmed by multiple community reports; no contradictory evidence found.

### Request Body

```json
{
  "model": "glm-ocr",
  "prompt": "Text Recognition:",
  "images": ["<base64-encoded PNG, no data-URL prefix>"],
  "stream": false
}
```

Key details:
- `model`: `"glm-ocr"` — the tag Ollama uses. Use `"glm-ocr:latest"` if the default tag is not pulled; both resolve identically.
- `prompt`: `"Text Recognition:"` — the canonical prompt string used in GLM-OCR's own Ollama deploy examples. The model's chat template expects this prefix.
- `images`: array with one element — raw base64 string of the PNG bytes. No `data:image/png;base64,` prefix. Strip it if encoding via the `base64` crate.
- `stream`: `false` — essential. Without this, Ollama streams newline-delimited JSON chunks. Setting it to false returns one complete JSON object.
- `num_ctx` is NOT sent in the request body. If context crashes occur, the Ollama operator must configure the modelfile with `PARAMETER num_ctx 16384`. This is an ops concern, not a code concern.

**Confidence:** MEDIUM — prompt string and endpoint sourced from official GLM-OCR repo deploy examples. The `"Text Recognition:"` prompt is what their reference implementation uses; alternative prompts work but may affect output formatting.

### Response Body

```json
{
  "model": "glm-ocr",
  "created_at": "...",
  "response": "The recognized text content here...",
  "done": true,
  "done_reason": "stop"
}
```

The recognized text is in the top-level `"response"` field. Parse only this field; discard the rest. Pass the value directly into the existing `sanitize_ocr_output()` function — it handles `\r\n` normalization, trailing whitespace, and blank line removal already.

**Confidence:** HIGH — standard Ollama `/api/generate` non-streaming response format, documented in official Ollama API docs and confirmed by multiple sources.

### Error Cases to Handle

| Condition | How to Detect | What to Return |
|-----------|--------------|----------------|
| Ollama unreachable | `reqwest` connection error | `Err("OCR failed: Ollama is unreachable at 192.168.1.12:11434. Is it running?")` |
| HTTP non-200 status | `response.status().is_success()` is false | `Err(format!("OCR failed: Ollama returned HTTP {}", status))` |
| `done: false` in response | Parse `done` field | Should not occur with `stream: false`; treat as error |
| Empty `response` field | Empty string after trim | Return `Err("OCR returned empty text")` or pass through — merge handles empty gracefully |
| Model not loaded | Ollama returns 404 or error JSON | Surface Ollama's error message verbatim |

---

## Integration Pattern

The entire change lives in `platform.rs`. The function signature is unchanged:

```rust
pub fn recognize_text_from_png(bytes: &[u8]) -> Result<String, String>
```

This function is called from `lib.rs` in the `commit_selection` Tauri command, which is already `async`. The new implementation will be:

```rust
pub async fn recognize_text_from_png(bytes: &[u8]) -> Result<String, String>
```

The caller in `lib.rs` already runs in an async context (`#[tauri::command]` with `async fn`), so adding `async` to this function and `.await`-ing it in `lib.rs` requires no structural changes.

### Client Instantiation

Do NOT store a `reqwest::Client` in `AppState`. For a low-frequency use case (one OCR call per user marquee commit), constructing a client per call is acceptable and avoids the complexity of injecting it into `SharedState`. Each call creates, uses, and drops a client. If response latency becomes a concern, moving the client to state is a straightforward refactor.

---

## What NOT to Use

| Option | Why Not |
|--------|---------|
| `tauri-plugin-http` | This is a Tauri plugin that exposes HTTP to the JavaScript frontend via IPC. The OCR call is entirely Rust-side — there is no reason to route it through the plugin or involve the frontend at all. |
| `ureq` (synchronous HTTP) | Synchronous blocking in an async Tauri command handler will deadlock the Tokio runtime. `reqwest` is the correct choice for async contexts. |
| `hyper` (direct) | Lower-level than needed. `reqwest` wraps hyper with a clean API and adds JSON support. No benefit to using hyper directly here. |
| `reqwest` 0.13 | Too new. Released recently with breaking changes: `query` and `form` are now opt-in features, and the default TLS backend switched to `aws-lc`. The 0.12 series is the current stable line with 0.12.24 as latest. Adopt 0.13 in a future maintenance pass. |
| `#[tokio::main]` on `main` | Tauri owns the runtime. Adding `#[tokio::main]` creates a second runtime and causes conflicts. Tauri's async commands run on Tauri's embedded Tokio runtime automatically. |
| OpenAI-compatible endpoint `/v1/chat/completions` | Ollama exposes this for compatibility, but it has known limitations for vision/image requests with GLM-OCR. Use `/api/generate`. |

---

## Tauri-Specific Notes

**CSP is irrelevant here.** CSP (`csp: null` in `tauri.conf.json`) governs what the WebView frontend can load. HTTP requests made from Rust via `reqwest` in a command handler bypass the WebView entirely — they are native OS network calls. No CSP or plugin scope configuration is needed for this use case.

**Async command handlers.** Tauri 2 fully supports `async fn` command handlers. The existing `commit_selection` handler is already async. Changing `recognize_text_from_png` to async and awaiting it is the idiomatic path.

---

## Sources

- [Ollama Vision API docs](https://docs.ollama.com/capabilities/vision) — image base64 format for `/api/generate`
- [Ollama API reference (GitHub)](https://github.com/ollama/ollama/blob/main/docs/api.md) — response JSON schema
- [zai-org/GLM-OCR Ollama deploy README](https://github.com/zai-org/GLM-OCR/blob/main/examples/ollama-deploy/README.md) — canonical prompt, endpoint recommendation
- [ollama.com/library/glm-ocr](https://ollama.com/library/glm-ocr) — model name, available tags
- [GLM-OCR PR #14024 in ollama/ollama](https://github.com/ollama/ollama/pull/14024) — confirmed native Ollama support
- [reqwest crates.io](https://crates.io/crates/reqwest) — version 0.12.24 current stable
- [reqwest 0.13 docs.rs](https://docs.rs/crate/reqwest/latest) — confirmed 0.13 exists, reviewed breaking changes
- [serde_json crates.io](https://crates.io/crates/serde_json) — version 1.0.149 current
- [Tauri HTTP client plugin docs](https://v2.tauri.app/plugin/http-client/) — confirmed plugin is JS-frontend-facing only
- [Tauri async runtime docs](https://docs.rs/tauri/latest/tauri/async_runtime/index.html) — Tauri owns Tokio runtime, no `#[tokio::main]` needed
