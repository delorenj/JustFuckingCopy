# JustFuckingCopy

If you can see it, you can copy it.

This is a greenfield Rust/Tauri prototype for "universal copy/paste" that works from pixels instead of application-specific clipboard APIs. The idea is simple:

1. Drag a marquee around the text you want.
2. OCR the crop.
3. Merge it into the running session, deduping overlap when the next crop repeats lines from the previous one.
4. Push the merged result into the native clipboard.

That makes it useful for awkward contexts like remote shells, nested terminal multiplexers, tunneled sessions, and apps that do not cooperate with normal clipboard semantics.

## What This Prototype Does

- Builds a desktop app with Tauri and a dependency-light static frontend.
- Screenshots are added to a batch
- Runs OCR on the crop.
- Appends the recognized text in selection order.
- Detects repeated overlap between adjacent captures and trims duplicated lines before merging.
- Copies the merged text into the native clipboard.

## Current Platform Support

- macOS: full prototype path implemented with `screencapture` + Vision OCR via Swift.
- Linux: screenshot capture fallback is wired for `grim`, `gnome-screenshot`, or ImageMagick `import`; OCR uses `tesseract` when installed.
- Windows: screenshot capture is wired through PowerShell; OCR backend is not implemented yet.

## Run It

```bash
cargo run --manifest-path src-tauri/Cargo.toml
```

On macOS you will likely need to grant Screen Recording permission the first time the app tries to capture the display.

## Project Layout

- `/Users/delorenj/code/JustFuckingCopy/ui`: static HTML/CSS/JS frontend
- `/Users/delorenj/code/JustFuckingCopy/src-tauri/src`: Rust app logic
- `/Users/delorenj/code/JustFuckingCopy/src-tauri/scripts/vision_ocr.swift`: macOS OCR bridge

## Design Notes

- The merge layer is intentionally backend-agnostic: OCR providers can change without changing the session model.
- Overlap handling is heuristic today: it uses fuzzy line matching rather than an LLM. That keeps the prototype local and predictable.
- The server-side interception idea you mentioned is a good next phase. A companion daemon that intercepts remote copy attempts and relays them to the local native clipboard would complement this pixel-driven mode rather than replace it.

## Good Next Steps

- Add a global hotkey so the app can hide itself, snap the display, and reopen directly into selection mode.
- Add an inline crop editor to fix OCR mistakes before committing.
- Implement a Windows OCR backend.
- Add a secondary "relay" mode for remote copy interception.
