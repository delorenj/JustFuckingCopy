# Codebase Concerns

**Analysis Date:** 2026-03-20

## Tech Debt

**Windows OCR Backend Not Implemented:**
- Issue: `recognize_text_from_file()` on Windows (line 126 in `src-tauri/src/platform.rs`) returns a stub error: "Windows OCR backend is not implemented yet."
- Files: `src-tauri/src/platform.rs`
- Impact: Application cannot perform OCR on Windows; feature is completely non-functional for Windows users. User captures screenshot → commit selection → OCR fails with error message.
- Fix approach: Implement Windows OCR backend using Windows.Media.Ocr API or integrate Tesseract for Windows. Requires adding `windows` crate and conditional compilation logic. Consider wrapping output similarly to macOS Swift subprocess pattern.

**Bundle Configuration Disabled:**
- Issue: `"bundle": { "active": false }` in `src-tauri/tauri.conf.json` (line 25) means the app cannot be packaged into distributable binaries.
- Files: `src-tauri/tauri.conf.json`
- Impact: Application cannot be shipped to end users. Currently only runnable via `cargo tauri dev`. No .app, .exe, or .dmg artifacts can be generated.
- Fix approach: Enable bundler, configure appropriate bundle targets per OS (macOS .dmg, Windows .msi or .exe, Linux .AppImage). Requires icon assets and code signing setup for production.

**Missing Frontend Error Boundaries:**
- Issue: Frontend JS error handling is minimal. Functions catch errors and flash them but don't distinguish error types or provide recovery context.
- Files: `ui/app.js` - all async functions (captureSnapshot, commitSelection, copyMergedText, etc.) catch errors generically with `catch (error) { flash(String(error), true) }`
- Impact: Users see raw error strings ("State lock was poisoned", "Failed to hide window"). No actionable guidance when operations fail. Unclear if error is recoverable.
- Fix approach: Create error taxonomy (user error, system error, recoverable vs fatal). Map backend error strings to user-friendly messages with recovery steps. Consider retry logic for transient failures.

## Known Bugs

**Potential Race Condition in Snapshot State:**
- Symptoms: `commit_selection` validates that `snapshot.id == request.snapshot_id` (line 84 in `src-tauri/src/lib.rs`), but if two snapshots are captured rapidly, the stale snapshot ID check may not catch all race conditions if frontend doesn't properly validate.
- Files: `src-tauri/src/lib.rs` (line 84-86), `ui/app.js` (line 91-96)
- Trigger: Capture snapshot → immediately capture another snapshot before committing selection → commit first selection. Possible the second snapshot ID could overwrite state.
- Workaround: Frontend currently prevents this by rendering commit button as disabled if no selection exists, but no explicit snapshot lifecycle lock on backend.

**Temp File Cleanup Silently Fails:**
- Symptoms: `fs::remove_file()` calls have `let _ = ` prefix, suppressing errors (line 34 in `src-tauri/src/platform.rs`, line 62 in `src-tauri/src/platform.rs`). If cleanup fails, temp files accumulate silently.
- Files: `src-tauri/src/platform.rs` (lines 34, 62)
- Trigger: Repeated captures or OCR operations on a system with file system issues (permissions, disk full, concurrent access).
- Workaround: None. Temp files will accumulate in `std::env::temp_dir()`.

## Security Considerations

**CSP Disabled:**
- Risk: `"security": { "csp": null }` in `src-tauri/tauri.conf.json` (line 21) disables Content Security Policy. Frontend can execute arbitrary inline scripts and load from any source.
- Files: `src-tauri/tauri.conf.json`, `ui/index.html`
- Current mitigation: Frontend is static HTML/vanilla JS with no dynamic content loading. No user-controlled input rendered into DOM. However, OCR results are rendered into `<pre>` tag via `escapeHtml()` (line 311 in `ui/app.js`).
- Recommendations: Re-enable CSP with sensible defaults (self-only for scripts, disable unsafe-inline). Frontend currently doesn't need dynamic content loading. Add nonce-based inline scripts if future refactoring requires them.

**Base64 Image Data in Memory:**
- Risk: Full PNG screenshots stored as base64-encoded strings in state (line 47 in `src-tauri/src/state.rs`). Each snapshot can be 2-5 MB for modern displays. Multiple snapshots can exhaust memory.
- Files: `src-tauri/src/state.rs`, `src-tauri/src/lib.rs` (lines 47, 66)
- Current mitigation: Only one "current_snapshot" is kept. Previous snapshots are not retained, only OCR results.
- Recommendations: Implement snapshot limit (max 5 snapshots). Add memory pressure detection. Consider writing large images to temporary disk storage instead of keeping in-memory base64.

**OCR Output Not Validated:**
- Risk: OCR results are passed directly from subprocess stderr/stdout to merged text without sanitization for malicious input (though OS subprocess model provides isolation).
- Files: `src-tauri/src/platform.rs` (lines 94-96, 120-122)
- Current mitigation: Tesseract and Vision framework are trusted OS-provided tools. No user-controlled input fed to them. But malformed OCR data could theoretically cause issues downstream.
- Recommendations: Validate OCR output is valid UTF-8 and within reasonable size limits (e.g., max 100KB). Consider rate-limiting clipboard writes.

## Performance Bottlenecks

**Levenshtein Distance Recalculated on Every Merge:**
- Problem: `find_overlap()` in `merge.rs` (lines 94-114) runs similarity calculations on every segment append, including recalculating Levenshtein distance for all suffix-prefix pairs.
- Files: `src-tauri/src/merge.rs` (lines 94-114, 132-145, 147-175)
- Cause: No memoization. For N segments, overlap detection is O(N * max_overlap * avg_line_length). As segments grow, rebuild_merge() in state.rs (lines 182-193) reprocesses all prior segments.
- Improvement path: Cache Levenshtein distances per line pair. Implement incremental merge: only recalculate for new segment vs prior segments, not all prior segments again. For 10-20 segments this is not noticeable, but at 100+ segments, performance degrades.

**Full State Clone on Every Command Response:**
- Problem: `to_payload()` in `state.rs` (lines 118-131) clones merged_text, all segments, and all selections. This happens on every command that returns state.
- Files: `src-tauri/src/state.rs` (lines 120-121), `src-tauri/src/lib.rs` (commands return AppStatePayload)
- Cause: Serde serialization requires owned data. No lazy serialization or reference-based payloads possible with current IPC design.
- Improvement path: For large state, serialize directly to bytes without intermediate Vec allocations. Or implement partial state queries (e.g., get_segments() instead of full state).

**Screenshot Base64 Encoding on Every Capture:**
- Problem: Every capture creates a base64-encoded data URL string (line 47 in `src-tauri/src/state.rs`). For a 4K screenshot (~5MB), this adds ~33% memory overhead and CPU time.
- Files: `src-tauri/src/state.rs` (line 47)
- Cause: Tauri IPC requires JSON-compatible payloads. Binary PNG data must be base64-encoded.
- Improvement path: Implement binary IPC for snapshot transfer using Tauri's invoke handler with custom serialization. Or cache encoded form and only decode on demand in UI.

## Fragile Areas

**Platform Abstraction Layer (`platform.rs`):**
- Files: `src-tauri/src/platform.rs` (235 lines)
- Why fragile: Contains three separate platform implementations (macOS, Linux, Windows) in one file. Linux implementation tries multiple backends in fallback chain (grim → gnome-screenshot → import). If any tool is missing or fails unexpectedly, error messages can be cryptic. macOS Swift subprocess spawning requires piping script to stdin—if Swift is not available, entire macOS backend fails with generic subprocess error.
- Safe modification: Before changing capture/OCR backends, add comprehensive platform-specific tests for each OS. Test with missing tools (e.g., tesseract not installed). Consider extracting platform modules into separate files (platform/macos.rs, platform/linux.rs, platform/windows.rs).
- Test coverage: Only merge.rs has unit tests (2 tests). No platform-specific capture/OCR tests. No tests for error paths.

**Merge Algorithm Hard-Coded Thresholds:**
- Files: `src-tauri/src/merge.rs` (line 107: threshold `0.78` for 3+ lines, `0.93` for 1-2 lines)
- Why fragile: Thresholds are magic numbers with no explanation or configuration. Different languages, fonts, OCR quality will break these assumptions. A typo in a numeric constant silently produces wrong dedup behavior.
- Safe modification: Extract thresholds to named constants with inline comments explaining rationale. Add merge strategy logging/debugging mode. Consider exposing threshold as optional parameter.
- Test coverage: 2 unit tests cover basic overlap cases but not edge cases (punctuation handling, unicode, mixed-case fuzzy matching).

**Frontend State Synchronization:**
- Files: `ui/app.js` (context object lines 21-32)
- Why fragile: Frontend `context.appState` is manually kept in sync with backend via `refreshState()`. If a backend state change occurs without corresponding UI refresh, frontend state becomes stale. No built-in consistency check.
- Safe modification: Implement explicit state refresh after every command that modifies state. Or implement backend-driven state push (WebSocket or watch pattern) instead of frontend pull.
- Test coverage: No frontend tests. No E2E tests validating UI reflects backend state.

## Scaling Limits

**In-Memory Snapshot Accumulation:**
- Current capacity: One snapshot in memory (PNG bytes + base64 string). Conservative estimate: 5 MB per 4K screenshot.
- Limit: If user captures 10 snapshots in one session and each is ~5 MB base64, ~50 MB heap usage just for images. Modern devices have plenty, but long sessions could approach GC pressure.
- Scaling path: Implement snapshot persistence to SQLite (or temp directory with cleanup). Implement LRU eviction. Add memory usage monitoring.

**Merge Algorithm Complexity:**
- Current capacity: Handles ~20-50 segments smoothly (Levenshtein distance is O(n*m) per pair).
- Limit: At 100+ segments with 10+ lines each, rebuild_merge() becomes slow (~100-500ms per rebuild). User perceives lag when adding segment.
- Scaling path: Implement incremental merge (only compare new segment against last N segments, not all). Cache normalized lines. Consider approximate similarity (rolling hash) for large datasets.

## Dependencies at Risk

**Tauri 2 Ecosystem Immaturity:**
- Risk: Tauri 2.0 is relatively new. Some plugins (clipboard_manager, plugins) may have breaking changes in minor versions. Vision framework on macOS is OS-dependent; Swift subprocess pattern is fragile if Swift toolchain changes.
- Impact: Update breaking Tauri or a plugin → rewrite command signatures and IPC. Deprecate Swift Vision OCR → need Windows equivalent.
- Migration plan: Pin exact versions in Cargo.toml (already done). Monitor Tauri release notes. Have fallback OCR strategy (tesseract on all platforms, not just Linux).

**Image Crate (No WASM, Desktop-Only):**
- Risk: `image` crate v0.25 is pinned but future versions may change API. Used for PNG decoding and cropping only.
- Impact: Low risk. Image formats stable. But no alternative pure-Rust image libs if issues arise.
- Migration plan: Image crate is stable and well-maintained. No action needed. If needed, could replace with lighter `png` crate if memory is concern.

## Test Coverage Gaps

**OCR Path Not Tested:**
- What's not tested: `recognize_text_from_png()`, `crop_png()`, platform-specific capture backends. No integration tests for full capture → OCR → merge flow.
- Files: `src-tauri/src/platform.rs`, `src-tauri/src/lib.rs` (commit_selection)
- Risk: Subtle bugs in platform-specific code go undetected. Windows backend completely untested.
- Priority: **High** – This is user-facing core functionality.

**Frontend JavaScript Not Tested:**
- What's not tested: All event handlers, canvas drawing, state synchronization, error handling. No unit tests or E2E tests.
- Files: `ui/app.js`
- Risk: UI bugs (marquee drawing incorrect on zoom, state desync, event handler race conditions) undetected until user encounters them.
- Priority: **High** – UI is first thing users interact with.

**State Mutation and Serialization:**
- What's not tested: `to_payload()`, `push_segment()`, `rebuild_merge()` interaction. No property-based testing of merge correctness across random inputs.
- Files: `src-tauri/src/state.rs`, `src-tauri/src/merge.rs`
- Risk: Subtle bugs in merge strategy selection, overlap counting, or serialization go unnoticed.
- Priority: **Medium** – Core logic, but merge.rs has partial coverage.

**Error Paths:**
- What's not tested: State lock poisoning, OCR failures, screenshot capture failures, file system errors, subprocess failures. All error cases return `Err(String)` but error messages not validated.
- Files: `src-tauri/src/lib.rs`, `src-tauri/src/platform.rs`
- Risk: Error messages inconsistent, unhelpful, or panic-prone.
- Priority: **Medium** – Important for user experience but less critical than happy path.

---

*Concerns audit: 2026-03-20*
