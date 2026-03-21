# Technology Stack

**Project:** JustFuckingCopy — v2.0 Ambient Tray
**Researched:** 2026-03-21
**Confidence:** HIGH for Tauri plugin choices (official docs confirmed); HIGH for notify/toml crate versions (crates.io confirmed); MEDIUM for badge-via-title pattern (community-confirmed, macOS badge API not natively exposed by Tauri)

---

## Context

This is a **subsequent milestone** document. The existing stack from v1.0 is fully validated and carries forward unchanged:

- `tauri = "2"` — app framework
- `reqwest = "0.12"` — Ollama HTTP client
- `tauri-plugin-clipboard-manager = "2"` — clipboard write
- `image = "0.25"` — PNG processing
- `serde = "1"`, `serde_json = "1"`, `base64 = "0.22"` — serialization

This document covers **only what is new** for v2.0: system tray, directory watching, global hotkey, TOML config, and archive file operations.

---

## New Dependencies

### Tauri Feature Flags (no new crate, just feature additions)

| Change | What To Add | Why |
|--------|-------------|-----|
| System tray | `tauri = { version = "2", features = ["tray-icon"] }` | The `tray-icon` feature is gated in Tauri 2 — it is not included by default. Adds `tauri::tray::TrayIcon`, `TrayIconBuilder`, and menu support. This is the correct Tauri 2 rename of v1's `system-tray` feature. |

The `tray-icon` feature is **built into the `tauri` crate itself** — no separate crate is needed. The `TrayIconBuilder` API lives at `tauri::tray`.

### New Crates to Add

| Library | Version | Purpose | Why This One |
|---------|---------|---------|--------------|
| `tauri-plugin-global-shortcut` | `"2"` | Register OS-level global hotkeys (Ctrl+Shift+C by default) that fire even when the app has no focused window | Official first-party Tauri plugin. Latest stable: 2.3.1. Abstracts `x11` (Linux) and `CGEventTap` (macOS) behind a clean `register()` API. No alternative exists in the Tauri ecosystem that integrates with the event loop cleanly. |
| `notify-debouncer-full` | `"0.7"` | Watch a directory for new PNG/screenshot files, with debounce to suppress duplicate events from screenshot tool write-then-rename sequences | `notify` (the underlying crate, 8.2.0) fires raw OS events that include both `Create` and `Rename` events for a single screenshot save. `notify-debouncer-full` 0.7 collapses these into a single deduplicated event after a configurable timeout, which is exactly the behavior needed for screenshot watchers. Use this over bare `notify` to avoid processing the same file twice. |
| `toml` | `"1"` | Parse `~/.config/justfuckingcopy/config.toml` into a Rust config struct | The canonical TOML parser for Rust, maintained by the TOML spec authors. Latest: 1.0.6+spec-1.1.0. Combined with `serde`'s `#[derive(Deserialize)]` (already in the project), `toml::from_str()` parses a config file into a typed struct in ~5 lines. No alternative needed. |
| `dirs` | `"6"` | Resolve `~/.config/` on both macOS and Linux following XDG / Apple conventions | Latest: 6.0.0. `dirs::config_dir()` returns `~/Library/Application Support` on macOS and `~/.config` on Linux — the correct platform-specific paths. Avoids hardcoding `~/.config` which is wrong on macOS. Tiny dependency, zero risk. |

---

## Cargo.toml Changes

The full `[dependencies]` block after v2.0 additions:

```toml
[dependencies]
base64 = "0.22"
dirs = "6"
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }
notify-debouncer-full = "0.7"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-clipboard-manager = "2"
tauri-plugin-global-shortcut = "2"
toml = "1"

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros"] }
```

Note: `tokio` stays dev-only — Tauri owns the runtime at runtime. The `notify-debouncer-full` crate spawns its own thread internally; no additional runtime configuration is needed.

---

## Integration Points with Existing Code

### System Tray (`tauri::tray`)

**Badge count** is implemented via `TrayIcon::set_title()`. On macOS, `set_title` renders text next to the tray icon in the menu bar — this is how all macOS menu bar badge-style counters work (e.g., "3" next to the icon). On Linux (GTK status icon), `set_title` displays alongside the icon if supported by the desktop environment. There is no native OS badge API exposed by Tauri 2 — `set_title` is the correct approach.

The tray icon itself is created once in `lib.rs` `setup()` and stored via Tauri's managed state or `app.tray_by_id()`. Dynamic icon swapping (e.g., icon changes when batch is non-empty) uses `TrayIcon::set_icon()`.

**No main window on launch**: Remove the window from `tauri.conf.json`. Handle `RunEvent::ExitRequested` with `api.prevent_exit()` to keep the event loop alive when no windows are open. This is documented Tauri 2 pattern for tray-only apps.

### Directory Watcher (`notify-debouncer-full`)

The watcher runs in a background thread spawned via `tauri::async_runtime::spawn_blocking()`. It sends debounced `DebouncedEvent` notifications into a `tokio::sync::mpsc` channel. The Tauri command handler (or a setup hook) receives from that channel and updates `SharedState`.

Key debounce configuration: use a 500ms timeout — enough to let screenshot tools finish their write-then-rename sequence without creating duplicate batch entries.

**Watch only PNG files**: Filter `DebouncedEvent` by checking `event.paths` extensions. Screenshot tools on macOS write `.png` by default; Linux tools write `.png` or `.jpg`. Filter on extension to ignore transient `.tmp` files.

### Global Hotkey (`tauri-plugin-global-shortcut`)

Register in `setup()` after tray creation:

```rust
app.handle().plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
```

Then register the shortcut from Rust (not JS — the app has no persistent window):

```rust
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
app.global_shortcut().register("Ctrl+Shift+C")?;
```

The shortcut string format is `"Modifier+Key"`. The handler fires a Tauri event that triggers `process_batch()` — OCR all pending files, run merge, write to clipboard.

**Hotkey string comes from TOML config** — read the config before registering.

### TOML Config (`toml` + `serde` + `dirs`)

Config file location: `dirs::config_dir().unwrap().join("justfuckingcopy/config.toml")`

This resolves to:
- macOS: `~/Library/Application Support/justfuckingcopy/config.toml`
- Linux: `~/.config/justfuckingcopy/config.toml`

Config is loaded once at startup in `setup()` before tray and watcher initialization. If the file does not exist, use hardcoded defaults (watch dir, hotkey, Ollama endpoint). Do not error on missing config — the app must work out of the box.

### Archive File Operations (std::fs)

No new crate needed. Use `std::fs::rename()` to move processed files into an `archive/` subdirectory alongside the watch directory. `std::fs::create_dir_all()` ensures the archive dir exists before the first move. All of this is sync and runs in a `spawn_blocking` context (already the pattern for the file watcher thread).

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Bare `notify` crate (without debouncer) | Screenshot tools emit 2–4 raw events per file save (Create, Modify, Rename). Without debouncing, the same file gets OCR'd multiple times. | `notify-debouncer-full = "0.7"` |
| `tauri-plugin-system-tray` (v1 name) | Renamed in Tauri 2. Does not exist as a separate plugin in v2. | `tauri = { features = ["tray-icon"] }` |
| `global-hotkey` crate (standalone) | Does not integrate with Tauri's event loop. Managing two event loops causes conflicts. | `tauri-plugin-global-shortcut = "2"` |
| `config` crate (layered config) | Heavyweight — supports env vars, multiple formats, layered merging. Overkill for a single TOML file with 3–5 fields. | `toml = "1"` + `serde` |
| `dirs-next` or `directories` crate | `dirs-next` is abandoned (last release 2021). `directories` adds `ProjectDirs` abstraction not needed here. | `dirs = "6"` (active, maintained) |
| `inotify` / `kqueue` crates directly | Platform-specific. Would require `#[cfg]` splitting. | `notify-debouncer-full` abstracts both |

---

## Version Compatibility

| Package | Version | Compatible With | Notes |
|---------|---------|-----------------|-------|
| `tauri` | `"2"` with `tray-icon` feature | `tauri-plugin-global-shortcut = "2"` | Both must be in the `"2"` semver series; mixing v1 plugin with v2 core causes compile errors |
| `notify-debouncer-full` | `"0.7"` | `notify` (pulled transitively as `"8"`) | Do NOT also add bare `notify` to Cargo.toml — let `notify-debouncer-full` pull it as a dependency to avoid version conflicts |
| `toml` | `"1"` | `serde = "1"` | Requires serde's `derive` feature, which is already present in the project |
| `dirs` | `"6"` | No conflicts | Pure stdlib + platform SDK, no shared dependencies with Tauri |

---

## Alternatives Considered

| Recommended | Alternative | Why Not |
|-------------|-------------|---------|
| `notify-debouncer-full "0.7"` | `notify "8"` (raw) | Raw notify emits multiple events per screenshot write; debouncer is essential for screenshot workflows |
| `toml "1"` | `serde_yaml`, `ron` | Config must be `.toml` per PROJECT.md requirement; other formats not applicable |
| `tauri-plugin-global-shortcut "2"` | `rdev` crate | `rdev` requires a separate event loop thread and does not integrate with Tauri's app lifecycle; plugin is cleaner and officially supported |
| `dirs "6"` | Hardcoded `~/.config` | Wrong on macOS (correct path is `~/Library/Application Support`); `dirs` handles both platforms correctly |

---

## Sources

- [Tauri 2 System Tray docs](https://v2.tauri.app/learn/system-tray/) — `tray-icon` feature flag, `TrayIconBuilder` API, `set_title` for badge, `set_icon` for dynamic updates
- [Tauri 2 tray namespace JS/Rust reference](https://v2.tauri.app/reference/javascript/api/namespacetray/) — `TrayIcon` struct methods confirmed
- [Tauri 2 Global Shortcut plugin docs](https://v2.tauri.app/plugin/global-shortcut/) — registration pattern, `GlobalShortcutExt` trait
- [tauri-plugin-global-shortcut crates.io](https://crates.io/crates/tauri-plugin-global-shortcut) — confirmed latest stable 2.3.1
- [notify-debouncer-full crates.io](https://crates.io/crates/notify-debouncer-full) — confirmed version 0.7.0
- [notify crates.io](https://crates.io/crates/notify) — confirmed latest 8.2.0 (pulled transitively)
- [toml crates.io](https://crates.io/crates/toml) — confirmed latest 1.0.6+spec-1.1.0
- [dirs crates.io / docs.rs](https://docs.rs/crate/dirs/latest) — confirmed latest 6.0.0
- [Tauri 2 tray-only app discussion](https://github.com/tauri-apps/tauri/discussions/11489) — `RunEvent::ExitRequested` + `prevent_exit()` pattern for windowless apps
- [Tauri async_runtime docs](https://docs.rs/tauri/latest/tauri/async_runtime/index.html) — `spawn_blocking` for file watcher thread

---
*Stack research for: JustFuckingCopy v2.0 Ambient Tray — new feature additions only*
*Researched: 2026-03-21*
