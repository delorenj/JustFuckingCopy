# Testing Patterns

**Analysis Date:** 2026-03-20

## Test Framework

**Runner:**
- Rust: Built-in `cargo test` (no external test framework)
- JavaScript: No test framework detected or configured

**Assertion Library:**
- Rust: Built-in `assert_eq!`, `assert!` macros
- JavaScript: Not applicable (no tests)

**Run Commands:**
```bash
# Rust: Run all unit tests in merge module
cargo test --manifest-path src-tauri/Cargo.toml

# Rust: Run tests with output
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture

# Rust: Run specific test
cargo test --manifest-path src-tauri/Cargo.toml merge::tests::dedupes_line_overlap
```

## Test File Organization

**Location:**
- Tests are **co-located** with production code in the same file
- Test module declared at end of file using `#[cfg(test)]` conditional

**File with tests:**
- `src-tauri/src/merge.rs` - Contains 2 unit tests for the merge algorithm

**Naming:**
- Test files: No separate test files; tests in `tests` submodule within source files
- Test functions: snake_case, descriptive verb-noun pattern (e.g., `dedupes_line_overlap`, `appends_sequentially_without_overlap`)

**Structure:**
```
src-tauri/src/
├── lib.rs                 # No tests
├── merge.rs               # Contains #[cfg(test)] mod tests
├── platform.rs            # No tests (platform-specific, I/O-heavy)
├── state.rs               # No tests (pure data structure)
└── main.rs                # No tests (minimal entry point)
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::{append_text, MergeStrategy};

    #[test]
    fn dedupes_line_overlap() {
        // Setup
        let first = "alpha\nbeta\ngamma";
        let second = "beta\ngamma\ndelta";

        // Execute
        let outcome = append_text(first, second);

        // Assert
        assert_eq!(outcome.strategy as u8, MergeStrategy::OverlapDeduped as u8);
        assert_eq!(outcome.overlap_lines, 2);
        assert_eq!(outcome.merged_text, "alpha\nbeta\ngamma\ndelta");
    }

    #[test]
    fn appends_sequentially_without_overlap() {
        // Setup
        let first = "alpha\nbeta";
        let second = "delta\nepsilon";

        // Execute
        let outcome = append_text(first, second);

        // Assert
        assert_eq!(
            outcome.strategy as u8,
            MergeStrategy::SequentialAppend as u8
        );
        assert_eq!(outcome.merged_text, "alpha\nbeta\ndelta\nepsilon");
    }
}
```

**Patterns:**
- **Setup:** Input data is inline in test function (no fixtures or factories used)
- **Execute:** Single function call on pure logic (no state mutation)
- **Assert:** Multiple assertions per test verifying both output and side effects (strategy + lines + text)
- **Teardown:** None needed; tests are side-effect free

## Mocking

**Framework:** Not used

**What to Mock:**
- No mocking currently employed; tests are integration tests of pure functions
- Future strategy: Mock Tauri commands in frontend would require test harness (currently no tests exist for frontend)

**What NOT to Mock:**
- Core merge algorithm (`append_text`) is tested directly with no mocks
- Pure functions (`similarity`, `canonical_line`, `levenshtein`) are tested indirectly through public API
- Platform-specific code (screenshot, OCR) has no tests and would require mocks for reliable testing

## Fixtures and Factories

**Test Data:**
- No test fixture files or factories
- Test data is hardcoded inline in test functions:

```rust
#[test]
fn dedupes_line_overlap() {
    let first = "alpha\nbeta\ngamma";
    let second = "beta\ngamma\ndelta";
    // ...
}
```

**Location:**
- Test data exists only in `src-tauri/src/merge.rs`
- No shared fixtures directory

## Coverage

**Requirements:** Not enforced

**View Coverage:**
```bash
# Generate coverage report (requires tarpaulin or similar)
cargo tarpaulin --manifest-path src-tauri/Cargo.toml --out Html --output-dir coverage/
```

**Current Coverage:**
- Merge algorithm: ~30% (2 happy-path tests, no edge cases)
- Tauri commands: 0% (no tests)
- Platform backends: 0% (no tests)
- JavaScript frontend: 0% (no tests)

## Test Types

**Unit Tests:**
- **Scope:** Pure functions in `merge.rs` (normalize_text, append_text, find_overlap, canonical_line, similarity, levenshtein)
- **Approach:** Black-box testing via public API (`append_text`)
  - Tests pass concrete input strings and verify output
  - Tests verify both merge strategy selection and result text
  - Tests are deterministic and isolated

**Integration Tests:**
- Not yet implemented
- Would require:
  - Tauri command invocation harness
  - Platform-specific test doubles for screenshot/OCR
  - Frontend integration with mock backend

**E2E Tests:**
- Not used

**Manual Testing:**
- Application is tested manually via `cargo tauri dev`
- Workflow: Capture screenshot → draw marquee → commit selection → verify merged text → copy to clipboard
- No automated E2E test suite exists

## Common Patterns

**Async Testing:**
Not applicable; tests are synchronous Rust. No async/await patterns.

**Error Testing:**
Not implemented. Tests currently:
- Do not test error paths (empty strings, invalid selections, lock poisoning)
- Do not test boundary conditions (single line, very long text)
- Do not test encoding/decoding failures in platform layer
- Do not test OCR failures or malformed input

Example of missing error test:
```rust
// This test does NOT exist but should:
#[test]
fn handles_empty_incoming_text() {
    let first = "alpha\nbeta";
    let second = "";  // Empty input
    let outcome = append_text(first, second);

    assert_eq!(outcome.strategy, MergeStrategy::SequentialAppend);
    assert_eq!(outcome.merged_text, "alpha\nbeta");
}
```

## Test Coverage Gaps

**Untested areas:**

1. **Merge Algorithm Edge Cases:**
   - Empty or whitespace-only input
   - Single-line overlaps
   - Very similar lines (near 93% threshold)
   - All lines overlapping (incoming is subset of existing)
   - Case sensitivity and special character normalization

2. **Tauri Commands (entire `lib.rs`):**
   - `get_app_state()` - No tests
   - `reset_session()` - No tests
   - `capture_snapshot()` - No tests (requires platform mock)
   - `commit_selection()` - No tests (requires OCR mock)
   - `undo_last_segment()` - No tests
   - `copy_merged_text()` - No tests (requires clipboard mock)

3. **State Management (`state.rs`):**
   - `store_snapshot()` - No tests
   - `push_segment()` - No tests
   - `undo_last_segment()` - No tests
   - `rebuild_merge()` - No tests (indirectly tested via append_text but state mutations not verified)

4. **Platform Layer (`platform.rs`):**
   - `crop_png()` - No tests (image manipulation)
   - `recognize_text_from_png()` - No tests (OCR backend invocation)
   - `capture_snapshot()` - No tests (platform-specific)
   - `sanitize_ocr_output()` - No tests (text normalization)
   - Linux/Windows/macOS branches - No tests (platform-specific)

5. **Frontend (`ui/app.js`):**
   - No tests for DOM manipulation
   - No tests for pointer event handling
   - No tests for Tauri command invocation
   - No tests for error handling/flash messages
   - No tests for canvas rendering

6. **Integration Paths:**
   - Snapshot capture → OCR → merge → update UI flow - No end-to-end tests
   - Multi-snapshot deduplication - Manual testing only
   - Error recovery (invalid selections, missing snapshots) - No automated tests

**Priority for Testing:** High
- Merge algorithm should have 100% coverage and edge case tests (it's the core innovation)
- State mutations should be unit tested independently
- Platform layer should have mock-based tests for reliability

---

*Testing analysis: 2026-03-20*
