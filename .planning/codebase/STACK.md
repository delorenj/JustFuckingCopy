# Technology Stack

**Analysis Date:** 2026-03-20

## Languages

**Primary:**
- Rust 2021 edition - Backend application logic, OCR integration, state management, merge algorithm
- JavaScript (Vanilla) - Frontend UI, no frameworks or build tools
- Swift - macOS OCR subprocess (`src-tauri/scripts/vision_ocr.swift`)

**Secondary:**
- PowerShell - Windows screenshot capture via System.Drawing
- Shell scripts - Linux screenshot backend fallback logic

## Runtime

**Environment:**
- Tauri 2 - Desktop application framework bridging Rust backend with web frontend
- Desktop platforms: macOS, Linux, Windows

**Package Manager:**
- Cargo (Rust) - Manages Rust dependencies
- No npm/Node.js in main application (Tauri handles frontend bundling)

## Frameworks

**Core:**
- Tauri 2 - Desktop app framework with IPC bridge between Rust and JavaScript

**Plugins:**
- tauri-plugin-clipboard-manager 2 - Native clipboard read/write operations

**Build:**
- tauri-build 2 - Build-time configuration and context generation for Tauri

## Key Dependencies

**Critical:**
- `tauri` 2 - Core framework for window management, command routing, asset serving
- `tauri-plugin-clipboard-manager` 2 - Write merged text to native clipboard via `app.clipboard().write_text()`
- `image` 0.25 - PNG decoding/encoding, image cropping, dimension queries
- `serde` 1 - JSON serialization/deserialization for command arguments and state payloads
- `base64` 0.22 - Encodes PNG bytes as data URLs for frontend display

**Platform Support:**
- `objc2`, `objc2-app-kit`, `objc2-foundation`, `objc2-core-graphics` - macOS Objective-C bindings (transitive via `tauri`)
- `windows-sys` 0.60.2 - Windows API bindings for PowerShell screenshot (transitive via `tauri`)

## Configuration

**Environment:**
- No environment variables required for runtime
- macOS requires Screen Recording permission (prompted on first `capture_snapshot()`)
- Linux requires `tesseract` OCR binary in PATH
- Windows OCR backend not yet implemented

**Build:**
- `src-tauri/tauri.conf.json` - Tauri configuration schema
  - Frontend dist: `../ui` (vanilla HTML/CSS/JS)
  - Window: 1440×960, resizable
  - Security: CSP disabled (`csp: null`)
  - Bundling: disabled (`bundle.active = false` - dev mode only)
- `src-tauri/Cargo.toml` - Rust dependencies and crate configuration
  - Library types: `staticlib`, `cdylib`, `rlib` (for Tauri integration)
  - Edition: 2021

## Platform Requirements

**Development:**
- Rust toolchain with Tauri 2 support
- macOS: Xcode Command Line Tools (for native compilation)
- Linux: `tesseract` OCR binary, `grim`/`gnome-screenshot`/`import` for screenshot capture
- Windows: PowerShell (built-in)

**Production:**
- macOS: Native code signed and notarized (bundling not yet active)
- Linux: `tesseract` binary must be installed before running
- Windows: .NET/Windows.Forms assembly available (PowerShell screenshot)

## Deployment

**Current:**
- No bundling active (`bundle.active = false` in `tauri.conf.json`)
- Run via `cargo tauri dev` for development
- Production bundling will target OS-native installers once enabled

---

*Stack analysis: 2026-03-20*
