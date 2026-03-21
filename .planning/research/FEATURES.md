# Feature Research: Ambient Tray OCR App (v2.0)

**Domain:** Ambient system tray app with directory watcher, global hotkey, batch OCR, and TOML config
**Researched:** 2026-03-21
**Confidence:** HIGH (Tauri 2 plugin APIs verified via official docs and crates.io; UX patterns from comparable apps)
**Milestone:** v2.0 — Transform modal window app to ambient tray app

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features that must exist or the ambient tray paradigm is broken or worse than v1.0.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Tray-only launch (no main window on startup) | Ambient apps must not interrupt the workflow they serve; a window appearing on launch destroys the ambient premise | LOW | Empty `windows` array in `tauri.conf.json` + keep event loop alive; `prevent_exit` required or Tauri quits when no windows are present |
| Tray icon with visual batch count | Users need passive awareness of pending items without opening anything; a plain icon with no state feedback is an ambient app that provides no ambient information | MEDIUM | Tauri 2 has no native badge API; must generate count images dynamically (pre-render numbered PNGs or draw via image crate at runtime); `TrayIcon::set_icon()` accepts dynamic icons |
| Directory watcher for screenshot detection | The entire premise is hands-off detection; if users must manually add files, this is worse than v1.0 | MEDIUM | Use `notify` crate (already transitive via `tauri-plugin-fs`); use `notify-debouncer-full` with ~500ms debounce; watch for `Create` and `Rename` events only (screenshot tools write then rename atomically) |
| Global hotkey to trigger batch processing | The one deliberate action the user takes; if it requires opening a UI to click a button, ambient value is lost | MEDIUM | `tauri-plugin-global-shortcut` (official Tauri 2 plugin); register in `setup()` hook; default `Ctrl+Shift+C`; must handle registration failure gracefully (hotkey already taken by another app) |
| Batch OCR: send full screenshots to Ollama | v2.0 eliminates marquee selection; the watcher collects full-screen PNGs and the hotkey sends them all | MEDIUM | Reuse existing `ollama.rs` `recognize_text` call; loop over pending snapshots sequentially; existing image resize guard (max 1800px) still required for full screenshots |
| Fuzzy merge across all batch items | Multiple screenshots of the same content (e.g., scrolled view) must deduplicate; this is the core value of the existing `merge.rs` — it must apply across all batch items, not just pairs | LOW | `merge.rs` contract is unchanged; call `append_text()` sequentially through the batch list; already validated in v1.0 |
| Copy merged result to clipboard after batch | The workflow completes with text on the clipboard; if the user must open a panel and click Copy, the hotkey workflow is incomplete | LOW | Existing `tauri-plugin-clipboard-manager` call; invoke after final merge result is computed |
| Archive processed screenshots after batch | Processed files must not re-enter the next batch; leaving them in the watch directory means they get OCR'd again on the next hotkey press | LOW | Move files to `{watch_dir}/processed/YYYY-MM-DD/` after successful batch; create subdirectory if absent; preserve original filenames |
| Status panel via left-click tray | Users need a way to see what's queued and what was last merged without running the full batch; left-click is the universal tray convention for "show me status" | MEDIUM | Create `WebviewWindow` on demand (not at launch); `set_visible(true)` / `set_focus()` on tray left-click; hide on blur or close; the existing vanilla JS frontend is repurposed here |
| TOML config file at `~/.config/justfuckingcopy/config.toml` | Power users must be able to change watch directory and hotkey without recompiling; absence of any config forces recompilation for basic customization | LOW | Use `toml` + `serde` crates; read at startup, apply values; create default config if absent; fields: `watch_dir`, `hotkey`, `ollama_endpoint` |
| Clear error if Ollama unreachable during batch | Inherited from v1.0 requirements; batch context makes it more critical — user pressed hotkey expecting result, silence is worse than an error | LOW | Surface error via tray tooltip update or status panel; same `reqwest` failure path as v1.0 |

### Differentiators (Competitive Advantage)

Features that distinguish this from generic screenshot-to-clipboard tools.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Fuzzy dedup across full-screenshot batch | No other tray OCR tool handles the "same region captured twice" problem; the user can take 5 overlapping screenshots and get one clean deduplicated result | LOW | Already built in `merge.rs`; zero new code; just wire the batch loop through the existing merge function |
| File filter for screenshot detection | Watch dir may contain non-screenshot files (PDFs, RAW images, app downloads); filtering to PNG/JPG with screenshot-like naming prevents spurious OCR triggers | LOW | Pattern match on file extension and optionally filename prefix (e.g., `Screenshot`, `screen`) in the watcher event handler |
| Debounced watcher (not immediate) | Screenshot tools write files in two steps (temp write + rename); an immediate watcher fires on the temp file and gets a partial PNG; debouncing to 500ms captures the final stable file | LOW | `notify-debouncer-full` with `Duration::from_millis(500)`; only process `Create`/`Rename` event kinds |
| Status panel shows batch queue + merged preview | The user can inspect what's queued before triggering OCR, and see the last merged result; gives confidence without being intrusive | MEDIUM | Repurpose existing vanilla JS frontend; new state shape: `pending_files[]`, `last_merged_text`, `batch_count`; IPC command `get_tray_state()` |
| "Clear batch" action without processing | User may want to discard accumulated screenshots without OCR; right-click tray menu item or status panel button | LOW | Move files to `processed/` dir (or delete — configurable) without calling Ollama |
| Config hot-reload (watch config file) | Dev/power-user workflow: change watch dir or hotkey in config.toml and have it apply without restarting the app | HIGH | Requires watching the config file with `notify` and re-registering global shortcut on change; hotkey re-registration is non-trivial (must unregister old, register new); defer to v2.1 |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Settings GUI window | Feels like polish; users expect apps to have settings screens | Doubles scope; settings UI requires its own window lifecycle, form state, validation, and persistence — all orthogonal to the core workflow | TOML config file; it's what power users actually prefer for background utilities |
| Per-file OCR progress bar | Users want to know "which file is being processed" | Requires a persistent visible window, defeating the ambient premise; a progress bar demands attention | Update tray tooltip during processing ("Processing 3 of 5..."); zero-friction status |
| Retry loop when Ollama unreachable | Seems resilient | On a local LAN, if Ollama is down it is genuinely down; retrying silently delays error feedback; user needs to know NOW, not after 3 failed attempts | Hard fail immediately; show error in status panel; let user fix Ollama and press hotkey again |
| Clipboard history panel | Natural extension of clipboard features | Requires persistent storage, search, dedup across sessions — a separate product, not a feature | Out of scope for v2.0; one merged result per batch, always fresh |
| OCR on all image types (PDF, HEIC, BMP) | Broad format support sounds better | Each format requires a different decode path; HEIC is Apple-specific; PDF is multi-page with its own complexity; GLM-OCR only accepts PNG/JPEG | Filter watch dir to PNG and JPEG only; document the limitation |
| Live OCR as files arrive (no hotkey) | "Why wait for hotkey?" | Sends every screenshot immediately without letting user accumulate a batch; merging partial state is undefined; network churn on rapid-fire screenshots | Watcher accumulates; hotkey triggers; explicit user intent is the correct model |
| Marquee selection inside status panel | Regression to v1.0 interaction model | Contradicts the ambient premise; if user must make a selection, v1.0 was simpler | Full-screenshot OCR + dedup is the replacement; if a region is needed, user should crop before saving to watch dir |
| Windows support | Completeness | Windows screenshot tools write to different paths; global hotkey registration on Windows has privilege edge cases; PowerShell screenshot stub is already incomplete | Linux/macOS only for v2.0; scope constraint is explicit in PROJECT.md |

---

## Feature Dependencies

```
[Tray-only launch]
    └──enables──> [Directory watcher] (watcher starts in setup hook, not window lifecycle)
    └──enables──> [Global hotkey registration] (registered in setup hook)
    └──enables──> [Status panel on demand] (panel created/destroyed dynamically, not at launch)

[Directory watcher]
    └──requires──> [File filter] (PNG/JPEG only, debounced)
    └──populates──> [Batch state] (pending_files list in AppState)
    └──drives──> [Tray icon badge count] (badge updates on every watcher event)

[Global hotkey]
    └──triggers──> [Batch OCR loop] (sequential, one file at a time)
                       └──requires──> [Ollama HTTP client] (existing ollama.rs, unchanged)
                       └──requires──> [Image resize guard] (existing, max 1800px)
                       └──feeds──> [Fuzzy merge] (existing merge.rs, unchanged)
    └──triggers──> [Clipboard copy] (existing tauri-plugin-clipboard-manager)
    └──triggers──> [Archive processed files] (move to processed/ subdir)
    └──triggers──> [Badge count reset] (batch cleared, badge → 0)

[TOML config]
    └──configures──> [Directory watcher] (which path to watch)
    └──configures──> [Global hotkey] (which key combination)
    └──configures──> [Ollama endpoint] (host:port)

[Status panel]
    └──requires──> [Batch state] (reads pending_files, last_merged_text)
    └──enables──> [Clear batch action] (manual discard without OCR)
    └──enables──> [Process now button] (same as hotkey, alternative trigger)

[Archive processed files]
    └──requires──> [Batch OCR loop completes] (only archive on success or explicit clear)
    └──prevents──> [Re-processing on next batch] (files no longer in watch dir)
```

### Dependency Notes

- **Directory watcher requires tray-only launch:** The watcher must start in the Tauri `setup()` hook, which runs before any window. If a window is present at startup, the app lifecycle ties window close to app exit, breaking the background-daemon model.
- **Badge count requires tray-only launch:** Badge updates happen on file system events in a background thread; the tray icon is the only persistent UI surface.
- **Batch OCR loop requires image resize guard:** Full screenshots are larger than marquee crops; the confirmed GLM-OCR 1800px image size limit is MORE likely to be triggered with full screenshots than with v1.0 marquee crops. This is a higher priority in v2.0.
- **Archive requires batch completion:** Do not move files mid-batch. Move atomically after all OCR + merge completes, or on explicit "clear batch" action. Partial archives create inconsistent watcher state.
- **Global hotkey conflicts with other apps:** `tauri-plugin-global-shortcut` registration returns an error if the hotkey is already registered by another process. The app must surface this clearly at startup and fall back to tray-menu-only triggering if registration fails.

---

## MVP Definition

### Launch With (v2.0)

Minimum viable product — what's needed to validate the ambient tray paradigm.

- [ ] Tray-only launch with no main window — establishes ambient premise
- [ ] Directory watcher with PNG/JPEG filter and 500ms debounce — passive accumulation
- [ ] Tray icon badge count (pre-rendered number PNGs, 0–9+) — ambient awareness
- [ ] Global hotkey `Ctrl+Shift+C` (configurable) — intentional trigger
- [ ] Batch OCR: sequential Ollama calls for all pending files, existing resize guard applied — core pipeline
- [ ] Fuzzy merge across batch via existing `merge.rs` — core value, already built
- [ ] Clipboard copy of merged result — workflow completion
- [ ] Archive processed files to `{watch_dir}/processed/YYYY-MM-DD/` — prevents re-processing
- [ ] Status panel (left-click tray): shows pending file count + last merged text + "Process now" + "Clear batch" — minimal feedback loop
- [ ] TOML config at `~/.config/justfuckingcopy/config.toml`: `watch_dir`, `hotkey`, `ollama_endpoint` — essential customization

### Add After Validation (v2.1)

Features to add once the ambient workflow is confirmed working.

- [ ] Config hot-reload — trigger once users report friction from restarting to change settings
- [ ] Desktop notification on batch completion — add if users miss the clipboard update
- [ ] Per-file error reporting in status panel — add if users encounter OCR failures on specific files
- [ ] Filename-pattern filter for watch dir — add if non-screenshot files trigger unwanted OCR

### Future Consideration (v3+)

Features to defer until product-market fit is established.

- [ ] Settings GUI window — defer; TOML is sufficient for power users
- [ ] Clipboard history panel — separate product scope
- [ ] Windows support — requires dedicated work on global hotkey registration and privilege handling
- [ ] App bundling and distribution — `bundle.active = false` until v2.0 is stable

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Tray-only launch | HIGH | LOW | P1 |
| Directory watcher | HIGH | MEDIUM | P1 |
| Tray badge count | HIGH | MEDIUM | P1 |
| Global hotkey | HIGH | MEDIUM | P1 |
| Batch OCR loop | HIGH | LOW (reuses ollama.rs) | P1 |
| Fuzzy merge across batch | HIGH | LOW (reuses merge.rs) | P1 |
| Clipboard copy | HIGH | LOW (reuses plugin) | P1 |
| Archive processed files | HIGH | LOW | P1 |
| TOML config | HIGH | LOW | P1 |
| Status panel | MEDIUM | MEDIUM | P1 |
| File filter (PNG/JPEG) | MEDIUM | LOW | P1 |
| Watcher debounce | MEDIUM | LOW | P1 |
| Clear batch action | MEDIUM | LOW | P2 |
| Hotkey conflict graceful degradation | MEDIUM | LOW | P2 |
| Config hot-reload | LOW | HIGH | P3 |
| Desktop notification | LOW | LOW | P3 |

**Priority key:**
- P1: Must have for v2.0 launch
- P2: Should have, add within v2.0 if not blocking
- P3: Nice to have, deferred to v2.1+

---

## Implementation Notes Per Feature

### Tray Badge Count
Tauri 2 has no native badge count API (confirmed: no native badge in `tauri-plugin-system-tray` or `tray-icon` crate as of 2026). The standard approach is to pre-render a set of small PNG icons (0, 1–9, "9+") with a number overlaid, and call `TrayIcon::set_icon()` when the count changes. Pre-rendering avoids runtime image generation complexity. Linux requires writing the icon to `$XDG_RUNTIME_DIR/tray-icon` path; Tauri handles this internally but the icon swap triggers a visual blink — acceptable for a count-change event.

### Directory Watcher Edge Cases
Screenshot tools (macOS `screencapture`, Linux `grim`, GNOME Screenshot) write files using an atomic rename pattern: the file is written as a temp name then renamed to the final name. A watcher configured for only `Create` events misses these. Must watch both `Create` AND `Rename` (specifically rename-to events). The `notify-debouncer-full` crate consolidates multiple events per file into one, which handles the temp-write + rename + attribute-update sequence that a single screenshot generates.

### Global Hotkey Failure
`tauri-plugin-global-shortcut` returns `Err` from `register()` if the OS rejects the combination (already registered by another app). The app must: (1) log the failure, (2) display a tray tooltip warning like "Hotkey unavailable — use tray menu", and (3) still be fully functional via the status panel "Process now" button. This is not a fatal error.

### Status Panel Window Lifecycle
Create the `WebviewWindow` on first left-click, not at launch. Subsequent clicks toggle visibility (`is_visible()` → `set_visible(false)` or `set_focus()`). Do not destroy and recreate on each toggle — creation has noticeable latency. The window should have `decorations: false`, `always_on_top: true`, and be positioned near the tray icon (use `TrayIconEvent` cursor position).

### TOML Config Bootstrap
On first launch with no config file: create `~/.config/justfuckingcopy/` directory, write a default `config.toml` with documented comments, and proceed with defaults. Never fail on missing config. Malformed TOML should log a warning and fall back to defaults rather than crashing.

### Batch Processing Order
Process files in filesystem modification-time order (oldest first). This ensures the merge algorithm receives screenshots in capture sequence, which is required for correct overlap deduplication. Do not rely on filename sort order — screenshot tools may produce non-chronological names.

---

## Existing Code Reuse Map

| New Feature | Existing Module | Reuse Notes |
|-------------|----------------|-------------|
| Batch OCR per file | `ollama.rs` `recognize_text()` | Call unchanged; loop over files externally |
| Image resize guard | `platform.rs` `crop_png()` resize logic | Extract as standalone function or call before OCR in new batch loop |
| Fuzzy merge across batch | `merge.rs` `append_text()` | Call sequentially per file; same contract as v1.0 |
| Clipboard copy | `tauri-plugin-clipboard-manager` | Same plugin, same call site pattern |
| App state | `state.rs` `AppState` | Extend with `pending_files: Vec<PathBuf>`, `last_merged_text: String`, `batch_count: usize` |

---

## Sources

- [Tauri 2 System Tray documentation](https://v2.tauri.app/learn/system-tray/) — tray icon API, dynamic icon updates
- [tauri-plugin-global-shortcut on crates.io](https://crates.io/crates/tauri-plugin-global-shortcut) — official plugin, platform support
- [Tauri 2 Global Shortcut documentation](https://v2.tauri.app/plugin/global-shortcut/) — registration API, permissions
- [notify-rs GitHub](https://github.com/notify-rs/notify) — cross-platform filesystem notification library
- [notify-debouncer-full docs](https://docs.rs/notify-debouncer-full) — debounced watcher with event consolidation
- [How to Create a Tray-Only Tauri App](https://dev.to/daanchuk/how-to-create-a-tray-only-tauri-app-2ej9) — tray-only lifecycle pattern
- [Tauri discussion: Start app with hidden main window](https://github.com/tauri-apps/tauri/discussions/5364) — window hide on startup patterns
- [ShareX GitHub issue #3431: Hotkey registration failed](https://github.com/ShareX/ShareX/issues/3431) — hotkey conflict real-world behavior
- [How to Build a File Watcher with Debouncing in Rust](https://oneuptime.com/blog/post/2026-01-25-file-watcher-debouncing-rust/view) — debouncer patterns for file-system noise
- [EasyScreenOCR App Store](https://apps.apple.com/jp/app/easyscreenocr-image-to-text/id1359663922) — comparable ambient OCR tray app UX reference

---
*Feature research for: Ambient tray OCR app — v2.0 milestone*
*Researched: 2026-03-21*
