# External Integrations

**Analysis Date:** 2026-03-20

## APIs & External Services

**Screenshot Capture (Platform-Specific):**
- macOS: `screencapture` CLI tool (built-in)
  - Invoked with `-x -t png` flags
  - Implementation: `src-tauri/src/platform.rs` line 16-22
  - Used by: `capture_snapshot()` Tauri command

- Linux: Multi-fallback strategy attempting three screenshot tools in order
  - Primary: `grim` with `-t png` flags
  - Secondary: `gnome-screenshot` with `-f` flag
  - Tertiary: ImageMagick `import` with `-window root` flag
  - Implementation: `src-tauri/src/platform.rs` line 164-199
  - Used by: `capture_snapshot()` Tauri command

- Windows: PowerShell System.Drawing API
  - Uses `System.Windows.Forms.SystemInformation.VirtualScreen` for bounds
  - Uses `System.Drawing.Graphics.CopyFromScreen()` for screenshot
  - Implementation: `src-tauri/src/platform.rs` line 201-223
  - Used by: `capture_snapshot()` Tauri command

**OCR Services (Platform-Specific):**
- macOS: Apple Vision Framework (via Swift subprocess)
  - Subprocess execution: Swift script `src-tauri/scripts/vision_ocr.swift`
  - Vision API: `VNRecognizeTextRequest` with accurate recognition level
  - Configuration: English US language, language correction enabled
  - Implementation: `src-tauri/src/platform.rs` line 67-97
  - Used by: `recognize_text_from_png()` called from `commit_selection()` Tauri command

- Linux: Tesseract OCR binary
  - Subprocess execution: `tesseract` CLI with arguments `-l eng --psm 6`
  - Configuration: English language, PSM mode 6 (uniform text blocks)
  - Implementation: `src-tauri/src/platform.rs` line 100-123
  - Error handling: Must be installed in system PATH
  - Used by: `recognize_text_from_png()` called from `commit_selection()` Tauri command

- Windows: Not yet implemented
  - Stub returns error: "Windows OCR backend is not implemented yet."
  - Implementation: `src-tauri/src/platform.rs` line 126-128

## Data Storage

**Databases:**
- None. State is in-memory only.

**File Storage:**
- Temporary filesystem only
  - Snapshots: PNG bytes passed as in-memory base64 data URLs
  - OCR crops: Temporary PNG files in `std::env::temp_dir()` with timestamped names
  - Cleanup: Files deleted immediately after OCR completes
  - No persistent image storage

**Caching:**
- In-memory session state via `SharedState` (Mutex-wrapped `AppState`)
  - Location: `src-tauri/src/state.rs`
  - Holds: Current snapshot, OCR segments list, merged text
  - Lifetime: Per application session

## Authentication & Identity

**Auth Provider:**
- None. Application requires no user authentication.
- macOS: Requires Screen Recording permission at OS level (granted by user interaction)

## Clipboard Integration

**Outgoing Clipboard Write:**
- Service: Native OS clipboard via `tauri-plugin-clipboard-manager`
- Method: `app.clipboard().write_text(merged_text)`
- Implementation: `src-tauri/src/lib.rs` line 117-138 (`copy_merged_text` Tauri command)
- Trigger: User clicks "Copy Merged Text" button
- Payload: Plain text string (merged OCR results with deduplication applied)

## Monitoring & Observability

**Error Tracking:**
- None. Errors are returned to frontend as `Result<T, String>`.

**Logs:**
- None configured. All errors return as Result strings to frontend or stderr from subprocesses.

## CI/CD & Deployment

**Hosting:**
- Desktop application only. No remote hosting.
- Runs entirely locally on user's machine.

**CI Pipeline:**
- None detected. No GitHub Actions or CI configuration present.

## Environment Configuration

**Required env vars:**
- None required at runtime.

**Conditional Requirements:**
- Linux: `tesseract` binary must exist in PATH (checked at runtime during OCR)
- macOS: Screen Recording permission must be granted (prompted at first screenshot capture)
- Windows: PowerShell must be available (built-in)

**Secrets location:**
- No secrets. Application contains no authentication tokens or API keys.

## Subprocess Management

**Active Subprocesses:**
- `screencapture` (macOS) - screenshot capture
- `grim`, `gnome-screenshot`, or `import` (Linux) - screenshot capture
- `powershell` (Windows) - screenshot capture
- `swift` (macOS) - Vision framework OCR
- `tesseract` (Linux) - OCR

**Subprocess Handling:**
- All subprocesses invoked via `std::process::Command`
- Stdout captured for output
- Stderr captured for error reporting
- Exit status checked and errors propagated to frontend

## Webhooks & Callbacks

**Incoming:**
- None. Application is event-driven only (user button clicks).

**Outgoing:**
- None. No remote API calls made.

---

*Integration audit: 2026-03-20*
