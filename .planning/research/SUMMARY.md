# Project Research Summary

**Project:** JustFuckingCopy — v2.0 Ambient Tray
**Domain:** Tauri 2 desktop app — system tray, directory watcher, global hotkey, batch OCR
**Researched:** 2026-03-21
**Confidence:** HIGH

## Executive Summary

JustFuckingCopy v2.0 transforms the existing modal window OCR tool into an ambient tray-only application. Instead of the user manually capturing a region, a directory watcher passively accumulates screenshots as they arrive, and a single global hotkey (`Ctrl+Shift+C`) triggers batch OCR across all pending files, merges the results via the existing fuzzy dedup algorithm, and writes the output to the clipboard. The entire v1.0 backend — `merge.rs`, `ollama.rs`, `platform.rs` — carries forward unchanged. The v2.0 work is purely additive: two new modules (`config.rs`, `watcher.rs`), targeted extensions to `state.rs` and `lib.rs`, four new Cargo dependencies, and a UI update.

The recommended approach is a strict 5-phase build order driven by compile-time dependencies: Cargo/config baseline first, then tray lifecycle, then watcher, then hotkey + batch processing, then UI. This order is non-negotiable because every downstream component depends on the tray process staying alive (Pitfall 1), which must be solved before any other feature is wired. The batch OCR pipeline reuses `ollama.rs` and `merge.rs` unchanged — the only new logic is a sequential file loop with mtime-sorted ordering to preserve the merge algorithm's temporal assumptions.

The highest-risk area is system tray lifecycle on Linux: `libayatana-appindicator` is not universally installed, badge count overlays do not exist in AppIndicator, and the app will silently run invisible if the library is absent. The second major risk is the "app exits when status panel closes" trap, which requires both `WindowEvent::CloseRequested` + `prevent_close` AND `RunEvent::ExitRequested` + `prevent_exit` to be handled — one layer alone is not sufficient in Tauri 2. Both must be addressed in Phase 1 before any other work.

## Key Findings

### Recommended Stack

The v1.0 stack is fully validated and carries forward without changes (`tauri 2`, `reqwest 0.12`, `tauri-plugin-clipboard-manager 2`, `image 0.25`, `serde 1`, `serde_json 1`, `base64 0.22`). Four additions are needed for v2.0: the `tray-icon` feature flag on the existing `tauri` crate (no new crate), plus new dependencies.

**New dependencies:**
- `tauri = { version = "2", features = ["tray-icon"] }` — tray icon support built into tauri crate, not a separate plugin
- `tauri-plugin-global-shortcut = "2"` (latest 2.3.1) — official first-party plugin; the only option that integrates cleanly with Tauri's event loop
- `notify-debouncer-full = "0.7"` — collapses the 2-4 raw OS events per screenshot write into a single stable event; bare `notify` without debouncing will OCR the same file multiple times
- `toml = "1"` — canonical TOML parser; combined with existing `serde`, parses config in ~5 lines
- `dirs = "6"` — resolves `~/.config` correctly on Linux and `~/Library/Application Support` on macOS; hardcoding `~/.config` is wrong on macOS

**Critical version constraint:** `tauri` and `tauri-plugin-global-shortcut` must both be in the `"2"` semver series. Do not add bare `notify` to Cargo.toml alongside `notify-debouncer-full` — let the debouncer pull it transitively to avoid version conflicts.

### Expected Features

All P1 features must ship in v2.0. P2 features should be included if not blocking the milestone. P3 features are deferred.

**Must have (table stakes — P1):**
- Tray-only launch with no main window — the entire ambient premise fails without this
- Directory watcher (PNG/JPEG, 500ms debounce) — passive accumulation is the core workflow
- Tray badge count via pre-rendered icon set (not `set_title` — broken on Linux) — ambient awareness
- Global hotkey `Ctrl+Shift+C` (configurable) — the single intentional trigger
- Batch OCR: sequential Ollama calls in mtime order with existing resize guard applied
- Fuzzy merge across batch via existing `merge.rs` — already built, zero new code
- Clipboard copy of merged result — workflow completion
- Archive processed files to `{watch_dir}/.jfc-archive/YYYY-MM-DD/` — prevents re-processing
- Status panel (left-click tray): pending files + last merged text + "Process now" + "Clear batch"
- TOML config at platform-correct path with defaults for `watch_dir`, `hotkey`, `ollama_endpoint`

**Should have (P2):**
- Hotkey conflict graceful degradation — `is_registered()` check post-registration, tray tooltip fallback
- Clear batch action (discard without OCR)

**Defer to v2.1+ (P3):**
- Config hot-reload — HIGH complexity (requires hotkey re-registration)
- Desktop notification on batch completion
- Settings GUI window — TOML is sufficient for power users
- Windows support — explicit scope constraint in PROJECT.md
- App bundling/distribution

**Anti-features to reject:**
- Per-file OCR progress bar (defeats ambient premise; use tray tooltip instead)
- Live OCR as files arrive without hotkey (no user control over batch accumulation)
- Marquee selection in status panel (regression to v1.0 model)
- Retry loops on Ollama failure (delay feedback; fail fast instead)

### Architecture Approach

The v2.0 architecture is strictly additive to v1.0. Two new modules are created (`config.rs` for TOML loading, `watcher.rs` for directory watching), two existing modules are extended (`state.rs` gets batch fields, `lib.rs` gets tray setup + plugin wiring + 3 new commands), and all other modules remain untouched. The key architectural patterns are: (1) lock-clone-drop before any async work to prevent `MutexGuard` held across `.await`, (2) `AppHandle::clone()` for background thread access to app state, (3) single `app.manage(SharedState)` with access from both commands and background threads.

**Major components:**
1. `config.rs` — Load/parse TOML config once at startup; provide typed defaults; stored via `app.manage()`
2. `watcher.rs` — `notify-debouncer-full` on background thread; pushes `PathBuf` to `BatchState`; updates tray badge; emits `"batch-updated"` to frontend; never does OCR
3. `state.rs (extended)` — Adds `batch: Vec<PathBuf>` and `batch_merged_text: String` to existing `AppState`; new `BatchPayload` for frontend serialization
4. `lib.rs (extended)` — `TrayIconBuilder` in `setup()`; global shortcut registration; `process_batch_inner()` async fn; 3 new commands (`get_batch_state`, `process_batch`, `clear_batch`)
5. `ui/app.js (extended)` — Listens for `"batch-updated"` and `"batch-processed"` events; renders file list and merged preview; "Process Now" and "Clear" buttons

**Data flow summary:**
```
File arrives in watch_dir
  -> notify-debouncer-full (500ms) -> watcher.rs
  -> state.batch.push(path) + tray badge update + emit("batch-updated")

Hotkey pressed (Ctrl+Shift+C)
  -> tauri-plugin-global-shortcut handler
  -> spawn async: lock->clone batch->drop lock
  -> for each path (mtime sorted): read PNG -> ollama::recognize_text -> merge::append_text
  -> lock->store result->clear batch->drop lock
  -> clipboard write + archive files + tray badge reset + emit("batch-processed")
```

### Critical Pitfalls

1. **App exits when status panel closes** — Handle BOTH `WindowEvent::CloseRequested` (`window.hide()` + `api.prevent_close()`) AND `RunEvent::ExitRequested` (`api.prevent_exit()`). One layer alone is insufficient in Tauri 2. Block-everything risk for Phase 1.

2. **Linux tray silently invisible** — `libayatana-appindicator` is not installed by default on Arch, minimal Debian, and others. Detect tray initialization failure at startup and emit a clear error rather than running silently with no tray icon and no way to quit.

3. **Badge count broken on Linux via `set_title`** — AppIndicator has no badge overlay concept. Use pre-rendered numbered PNG icons (`tray-0.png` through `tray-9plus.png`) and `set_icon()` instead of `set_title()`. This is the only cross-platform approach.

4. **File watcher fires before PNG is fully written** — Use `notify-debouncer-full` with 500ms window AND a 3-attempt retry-open loop after receiving the debounced event. Silent failure mode: empty or corrupt OCR results.

5. **OCR results arrive in network-latency order, not file-creation order** — The merge algorithm is order-sensitive. Sort the batch by file mtime before dispatch, tag concurrent futures with source index, sort results before feeding into merge pipeline. Silent failure mode: scrambled clipboard output that looks plausible.

6. **MutexGuard held across `.await`** — `process_batch_inner()` must use the established v1 clone-before-await pattern. Lock, clone `Vec<PathBuf>`, drop guard, do all async OCR work, re-acquire lock to write results. Compile error with `std::sync::Mutex`; deadlock with `tokio::sync::Mutex`.

7. **Global hotkey silently fails to register** — `register()` returning `Ok(())` does not guarantee the hotkey is active. Always verify with `is_registered()` immediately after. Surface failure via tray tooltip. Register exclusively inside the `setup()` closure.

8. **macOS dock icon for tray-only app** — Set `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` in `setup()` on macOS. One line; easy to miss; produces obvious wrong UX.

9. **Config parse silent failure with wrong defaults** — `unwrap_or_default()` on TOML parse result silently uses developer-specific `watch_dir` path on every other machine. Use field-level `#[serde(default = "fn")]` attributes, log parse errors explicitly, validate that `watch_dir` exists after config load, and create it if absent.

10. **Duplicate tray icons** — Never initialize tray in both `tauri.conf.json` AND Rust `setup()`. Use Rust `setup()` exclusively for dynamic behavior.

## Implications for Roadmap

Based on the dependency graph from ARCHITECTURE.md and the pitfall-to-phase mapping from PITFALLS.md, the build order is clear and non-negotiable. Each phase is independently verifiable before starting the next.

### Phase 1: System Tray + App Lifecycle Foundation
**Rationale:** Every other feature depends on the tray process staying alive. The most critical pitfalls (app exits on window close, Linux tray invisibility, macOS dock icon) must be solved before any other feature is wired. This phase has no external dependencies.
**Delivers:** App launches to tray with no main window, left-click toggles status panel, close button hides (not quits) the panel, `RunEvent::ExitRequested` handled to keep process alive, macOS `ActivationPolicy::Accessory` set, Linux tray dependency detected at startup with clear error.
**Addresses:** Tray-only launch, status panel window lifecycle.
**Avoids:** Pitfalls 1 (app exits on close), 2 (Linux tray silent), 8 (macOS dock icon), 10 (duplicate tray init).
**Cargo changes:** Add `tray-icon` feature to `tauri`; update `tauri.conf.json` (`visible: false`, `skipTaskbar: true`); create `icons/tray.png`.

### Phase 2: TOML Config
**Rationale:** Config must load before watcher and hotkey because both read their parameters (`watch_dir`, `hotkey`, `ollama_endpoint`) from config. Tray is already alive from Phase 1 so `app.manage()` is available. This phase has no async complexity and is independently unit-testable.
**Delivers:** `config.rs` with `AppConfig` struct, platform-correct path via `dirs`, field-level serde defaults, parse error logging with tray notification, `watch_dir` existence check and creation, default config file written on first run.
**Addresses:** TOML config feature.
**Avoids:** Pitfall 9 (config silent failure with developer-specific defaults).
**New crates:** `toml = "1"`, `dirs = "6"`.

### Phase 3: Directory Watcher + State Extension + Badge
**Rationale:** Watcher reads `watch_dir` from config (Phase 2 dependency). State extension must precede watcher because watcher pushes to `BatchState`. Badge count is part of this phase because it is directly triggered by watcher events. Pre-rendered icon set for badge must be created here.
**Delivers:** `watcher.rs` with `notify-debouncer-full` on background thread, PNG-only filter, 500ms debounce, retry-open loop for partial writes, `BatchState` fields added to `state.rs`, pre-rendered badge icons (`tray-0.png` to `tray-9plus.png`), `set_icon()` on file arrival, `"batch-updated"` event emission.
**Addresses:** Directory watcher, tray badge count, file filter, watcher debounce.
**Avoids:** Pitfalls 3 (badge broken on Linux), 4 (PNG incomplete write).
**New crates:** `notify-debouncer-full = "0.7"`.

### Phase 4: Global Hotkey + Batch OCR Pipeline
**Rationale:** Hotkey reads key string from config (Phase 2). Batch processing reads from `BatchState` (Phase 3). This is the core value delivery phase — all prior phases are prerequisites. Most complex function in the codebase lives here.
**Delivers:** Global hotkey registration with `is_registered()` verification and tray fallback on failure, `process_batch_inner()` async function with mtime-sorted file ordering, sequential OCR via `ollama::recognize_text()`, `merge::append_text()` across batch in order, clipboard write, archive to `.jfc-archive/YYYY-MM-DD/`, batch state reset, badge reset, `AtomicBool` guard for concurrent hotkey press protection, `"batch-processed"` event emission.
**Addresses:** Global hotkey, batch OCR, fuzzy merge across batch, clipboard copy, archive processed files, hotkey conflict graceful degradation, clear batch action.
**Avoids:** Pitfalls 5 (hotkey silent fail), 6 (OCR out of order), 7 (MutexGuard across await).
**New crates:** `tauri-plugin-global-shortcut = "2"`.

### Phase 5: Status Panel UI
**Rationale:** Frontend can be verified with direct `invoke()` calls during backend phases. UI is last because all IPC commands it calls must already exist. This phase is purely additive to `ui/app.js`.
**Delivers:** `"batch-updated"` and `"batch-processed"` event listeners in `app.js`, pending file list render, merged text preview, "Process Now" button wired to `process_batch` command, "Clear Batch" button wired to `clear_batch` command, Ollama error display in panel.
**Addresses:** Status panel, error visibility.
**Avoids:** No new pitfalls introduced in UI layer.

### Phase Ordering Rationale

- Phase 1 before everything: process must survive window close before adding any feature that depends on process longevity
- Phase 2 before Phases 3 and 4: both watcher and hotkey read their configuration from `AppConfig`
- Phase 3 before Phase 4: batch processing requires `BatchState` fields and a populated `batch: Vec<PathBuf>`
- Phase 4 before Phase 5: UI commands must exist before the frontend can call them
- Each phase is independently verifiable without the next phase being complete

### Research Flags

Phases with complexity warranting step-level planning attention:
- **Phase 1:** Tray lifecycle on Linux is environment-dependent; verify `libayatana-appindicator` detection logic works in the target environment before declaring done
- **Phase 3:** Badge icon pre-rendering requires build-time asset creation; confirm icon asset pipeline before wiring `set_icon()` calls; `notify-debouncer-full` vs `notify-debouncer-mini` — use `full` (STACK.md is authoritative)
- **Phase 4:** `process_batch_inner()` touches async, Mutex, file I/O, HTTP, merge, clipboard, and filesystem atomically — deserves careful step-by-step implementation; concurrent hotkey press guard (`AtomicBool`) placement needs a decision

Phases with standard, well-documented patterns (skip deep research):
- **Phase 2:** `toml` + `serde` + `dirs` config loading is boilerplate with multiple confirmed examples
- **Phase 5:** Event listener + DOM update pattern is identical to existing `app.js` patterns

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All new crates verified on crates.io; `tauri-plugin-global-shortcut 2.3.1` and `notify-debouncer-full 0.7.0` confirmed; version compatibility verified |
| Features | HIGH | Tauri 2 plugin APIs verified via official docs; feature dependencies clearly mapped; anti-features argued from first principles |
| Architecture | HIGH | Official Tauri 2 docs + confirmed community patterns; build order validated against dependency graph; working code samples provided and reviewed |
| Pitfalls | MEDIUM-HIGH | Most pitfalls verified via Tauri GitHub issues and notify-rs issue tracker; Linux AppIndicator limits confirmed via multiple sources; kqueue `Access(Close(Write))` event availability on macOS is MEDIUM confidence |

**Overall confidence:** HIGH

### Gaps to Address

- **`notify-debouncer-full` vs `notify-debouncer-mini` discrepancy:** ARCHITECTURE.md code samples use `notify-debouncer-mini`; STACK.md specifies `notify-debouncer-full`. Use `notify-debouncer-full = "0.7"` — it is the more complete implementation. Update any code samples referencing the mini variant during Phase 3 execution.
- **Linux badge rendering variation:** `set_icon()` with pre-rendered PNGs is the correct approach, but the visual result varies by desktop environment (GNOME with AppIndicator extension, KDE, XFCE). Accept DE-specific variation as expected behavior rather than a bug.
- **Concurrent hotkey press handling:** The `AtomicBool processing_flag` approach is recommended but the exact behavior (queue second press vs. silently drop) needs a decision during Phase 4. Silently dropping is simpler and correct for this use case.
- **Ollama endpoint config timing:** ARCHITECTURE.md notes endpoint "becomes config-driven in v2.1" but FEATURES.md lists `ollama_endpoint` as a v2.0 config field. FEATURES.md is authoritative for scope — treat it as a v2.0 field.

## Sources

### Primary (HIGH confidence)
- [Tauri 2 System Tray docs](https://v2.tauri.app/learn/system-tray/) — tray-icon feature, TrayIconBuilder, set_title badge, set_icon
- [Tauri 2 Global Shortcut plugin docs](https://v2.tauri.app/plugin/global-shortcut/) — registration pattern, GlobalShortcutExt trait
- [Tauri 2 Calling Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/) — app.emit() pattern
- [tauri-plugin-global-shortcut crates.io](https://crates.io/crates/tauri-plugin-global-shortcut) — version 2.3.1 confirmed
- [notify-debouncer-full docs.rs](https://docs.rs/notify-debouncer-full) — version 0.7.0 confirmed
- [toml crates.io](https://crates.io/crates/toml) — version 1.0.6 confirmed
- [dirs crates.io](https://crates.io/crates/dirs) — version 6.0.0 confirmed, platform paths verified
- [Tauri Discussion #11489](https://github.com/tauri-apps/tauri/discussions/11489) — tray-only app ExitRequested + prevent_exit pattern

### Secondary (MEDIUM confidence)
- [Tauri GitHub Issue #8982](https://github.com/tauri-apps/tauri/issues/8982) — duplicate tray icons bug
- [Tauri GitHub Issue #13511](https://github.com/tauri-apps/tauri/issues/13511) — prevent exit when all windows closed
- [notify-rs Issue #267](https://github.com/notify-rs/notify/issues/267) — rapid-fire creation event loss
- [notify-rs Issue #365](https://github.com/notify-rs/notify/issues/365) — kqueue Create event missing
- [tray-icon crate README](https://github.com/tauri-apps/tray-icon) — Linux AppIndicator runtime requirements
- [Tokio shared state tutorial](https://tokio.rs/tokio/tutorial/shared-state) — std::sync::Mutex vs tokio::sync::Mutex
- Project memory: JFC Ollama OCR Migration — clone-before-await mutex pattern (verified in v1 implementation)

### Tertiary (LOW confidence / needs validation)
- Linux desktop environment badge rendering on KDE/XFCE — accepted as environment-dependent; pre-rendered icon approach mitigates
- macOS kqueue `Access(Close(Write))` event availability — retry-open loop is the safe fallback regardless of event kind

---
*Research completed: 2026-03-21*
*Ready for roadmap: yes*
