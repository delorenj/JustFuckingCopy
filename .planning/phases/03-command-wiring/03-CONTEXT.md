# Phase 3: Command Wiring - Context

**Gathered:** 2026-03-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Wire `ollama::recognize_text()` into the `commit_selection` Tauri command handler, making it async. After this phase, the full pipeline works end-to-end: marquee selection triggers Ollama OCR and produces correct deduplicated clipboard text.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

All implementation choices are at Claude's discretion — infrastructure wiring phase. Key constraints from research:
- `commit_selection` becomes `async fn`
- State mutex MUST NOT be held across `.await` (clone-before-await pattern)
- Lock → extract PNG bytes → drop lock → await `ollama::recognize_text()` → re-lock → push segment
- `recognize_text_from_png(&crop)?` call in lib.rs line ~94 replaced with `ollama::recognize_text(&crop).await?`
- Error from Ollama propagates as `Result<_, String>` to frontend (existing pattern)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ollama::recognize_text(png_bytes: &[u8]) -> Result<String, String>` (from Phase 1)
- `mod ollama;` already declared in `lib.rs`
- Existing `SharedState` with `std::sync::Mutex`

### Established Patterns
- All Tauri commands currently sync, returning `Result<AppStatePayload, String>`
- State access: `state.inner.lock().unwrap()` then operate on `AppState`
- `commit_selection` flow: validate snapshot → crop → OCR → push_segment → return state

### Integration Points
- `lib.rs:commit_selection` — the one function that changes
- `state.rs:AppState::push_segment` — called after OCR, no changes needed
- Frontend `invoke("commit_selection")` — already handles async (JS `invoke` returns Promise)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — wiring phase. The app should work identically to before, just with Ollama as the OCR backend.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
