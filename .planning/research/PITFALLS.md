# Pitfalls Research

**Domain:** Tauri 2 ambient tray app — system tray, directory watcher, global hotkey, batch OCR, TOML config
**Researched:** 2026-03-21
**Confidence:** MEDIUM-HIGH (Tauri 2 official docs + GitHub issues + community reports; Linux AppIndicator limits verified via multiple sources; file watcher race conditions from notify-rs issue tracker)

---

## Critical Pitfalls

### Pitfall 1: App Exits When the Status Panel Window Is Closed

**What goes wrong:**
By default Tauri exits the process when the last window closes. The status panel is a `WebviewWindow`. When the user closes it, the whole app quits — tray icon disappears, watcher stops, hotkey unregisters. Looks like a crash. The process is gone.

**Why it happens:**
Tauri's default `RunEvent::ExitRequested` behavior terminates the process. Developers wire up `on_window_event` with `prevent_close` but forget to also handle `RunEvent` at the app level. The v1 pattern alone is not sufficient in Tauri 2 — closing a window *and* having no remaining windows triggers a separate exit path.

**How to avoid:**
Handle both layers in `lib.rs` `run()`:
```rust
.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        window.hide().unwrap();
        api.prevent_close();
    }
})
.build(tauri::generate_context!())?
.run(|_app, event| {
    if let RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();
    }
});
```
Always use `window.hide()` to dismiss the panel, never `window.close()`.

**Warning signs:**
- Status panel opens/closes correctly in testing, but "crash" reports appear when users close the panel
- Tray icon disappears after closing the panel for the first time (particularly on macOS)
- `RunEvent::ExitRequested` fires in a loop on macOS when closing windows inside `prevent_exit` — known Tauri issue #11489

**Phase to address:** System Tray + App Lifecycle — must be the first phase; every other feature depends on the process staying alive.

---

### Pitfall 2: Duplicate Tray Icons on Startup

**What goes wrong:**
Two tray icons appear simultaneously — one transparent/non-functional with click events wired, one visible but inert. On some Linux desktop environments this manifests as no icon at all.

**Why it happens:**
Tray is initialized in two places: both the `systemTray` block in `tauri.conf.json` (declarative) and inside the `setup()` hook via Rust code. Tauri 2 creates one `TrayIcon` per initialization path. They coexist silently with no warning.

**How to avoid:**
Use exactly one initialization path. For a tray-only app with dynamic badge updates, initialize in Rust `setup()` and remove any `systemTray` block from `tauri.conf.json`. Never construct `TrayIcon::new()` more than once in the same process.

**Warning signs:**
- Two icons visible in the tray bar during `cargo tauri dev`
- Icon flickers/blinks when calling `set_icon()` — symptom of competing icon managers
- `icon_as_template` is reset on every `set_icon()` call, requiring a follow-up `set_icon_as_template(true)` — blink is visible

**Phase to address:** System Tray + App Lifecycle

---

### Pitfall 3: File Watcher Fires Before the PNG Is Fully Written

**What goes wrong:**
The watcher receives a `Create` or `Write` event for a new screenshot PNG. The app immediately opens and decodes the file. The image decoder gets a truncated or corrupt file because the screenshot tool (grim, gnome-screenshot, macOS screencapture) is still writing. OCR fails silently or returns garbage text.

**Why it happens:**
Screenshot tools typically write to a temp path then rename atomically. The `notify` event fires on the rename completion, but that may precede OS buffer flush to disk. On Linux with inotify, `IN_CLOSE_WRITE` is the correct signal — but `notify`'s default event kind is `Modify` or the debounced equivalent of `Create`, not a guaranteed close event. On macOS with FSEvents, the event coalescing window means events batch but arrival timing varies.

Additionally: `notify` has a documented issue (#267) where if many files are created in rapid succession, early events come through but later ones are delayed — causing the listener to see a partial batch initially.

**How to avoid:**
Use `notify-debouncer-full` with a minimum 500ms debounce window. After receiving the debounced event, attempt to open the file in a retry loop before processing:
```rust
for attempt in 0..3 {
    match image::open(&path) {
        Ok(img) if img.width() > 0 => { /* proceed */ break; }
        _ => tokio::time::sleep(Duration::from_millis(200)).await,
    }
}
```
On Linux specifically, prefer watching for `EventKind::Access(AccessKind::Close(AccessMode::Write))` events where available, as this is the closest to "file is done being written."

**Warning signs:**
- OCR returns empty strings or partial text for screenshots that clearly contain text
- `image::open` errors in logs: `UnexpectedEof`, `InvalidSignature`, `IoError`
- Problem is intermittent — worse on slower storage, disappears on fast NVMe

**Phase to address:** Directory Watcher

---

### Pitfall 4: Batch OCR Results Arrive in Network-Latency Order, Not File-Creation Order

**What goes wrong:**
Five screenshots are pending. The user presses the hotkey. All five Ollama requests fire concurrently via `join_all`. Responses return in variable network order. The merge algorithm receives them in wrong temporal sequence — deduplication treats later text as "new" content and appends it before earlier text. Merged clipboard output is scrambled.

**Why it happens:**
`tokio::spawn` + `join_all` gives maximum throughput but no ordering guarantee. The existing `merge.rs` algorithm is order-sensitive — it was designed to receive captures in temporal sequence. Nothing in the current batch dispatch path preserves file-creation order.

**How to avoid:**
Sort the batch by file modification time (or inode creation time where available) before dispatching. Tag each OCR future with its source index. After `join_all` completes, sort results by that index before feeding into the merge pipeline:
```rust
// Sort paths by mtime before dispatch
paths.sort_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());

// Tag futures with index
let futures: Vec<_> = paths.iter().enumerate()
    .map(|(i, path)| async move { (i, ocr(path).await) })
    .collect();

let mut results = join_all(futures).await;
results.sort_by_key(|(i, _)| *i);
```

**Warning signs:**
- Merged clipboard text has paragraphs in wrong order compared to screenshot creation sequence
- Problem only manifests with 3+ screenshots in a batch (1-2 rarely reorder)
- Reproducible under Ollama load (when some requests take longer than others)

**Phase to address:** Batch OCR + Merge Pipeline

---

### Pitfall 5: Global Hotkey Silently Fails to Register

**What goes wrong:**
`register()` returns `Ok(())` but pressing the hotkey does nothing. No error is raised, no log entry is written. The handler is never invoked. The user sees the tray icon but triggering OCR is impossible.

**Why it happens:**
Three distinct causes:
1. Another application already owns the key combination. On Linux/GNOME, `Ctrl+Shift+C` is a "Copy" shortcut inside GNOME Terminal and conflicts with some Wayland compositor configurations. On macOS, system-level accessibility shortcuts can silently preempt registration.
2. The plugin is initialized before the Tauri runtime is fully started — calling `register()` outside the `setup()` hook.
3. On some Linux configurations (Wayland without XWayland, or missing `xdo` dependency), the underlying key interception layer silently no-ops.

**How to avoid:**
- Call `is_registered()` immediately after `register()` and treat `false` as a failure — do not trust `Ok(())` alone.
- Surface a tray menu notification ("Hotkey failed to register — check config.toml") when `is_registered()` returns false.
- Make the hotkey string configurable in `config.toml` so users can choose a non-conflicting combination.
- Register exclusively inside the `setup()` closure, never before `Builder::build()` returns.
- On macOS, verify accessibility permission is granted before attempting registration; prompt if not.

**Warning signs:**
- Hotkey works on developer machine (macOS) but not on user machine (Linux)
- `register()` succeeds but `is_registered()` returns `false` immediately after
- Registration succeeds after a reboot but fails after another app is launched

**Phase to address:** Global Hotkey

---

### Pitfall 6: Badge Count Has No Native Overlay Support on Linux

**What goes wrong:**
The project plan requires a badge count on the tray icon showing pending screenshot count. On macOS, `set_title("3")` works and renders cleanly. On Linux with `libayatana-appindicator`, `set_title()` renders as a text label *beside* the icon in the system tray bar — not as an overlay badge. The visual result is completely different from the macOS design, and on some desktop environments (KDE, XFCE) the title label does not appear at all. `libappindicator3` has no tooltip support whatsoever.

**Why it happens:**
AppIndicator on Linux has no concept of badge overlays. The Tauri tray API calls through to the platform layer, which cannot compensate for missing AppIndicator capabilities. This is a platform limitation, not a Tauri bug.

**How to avoid:**
Implement badge count as a pre-rendered icon set. Generate small PNG icon variants at build time: `tray-0.png`, `tray-1.png` ... `tray-9.png`, `tray-9plus.png` — with count baked into the icon image. Call `set_icon()` with the appropriate pre-rendered icon when the count changes. This works identically on macOS and Linux. Use tooltip text as a secondary indicator where available.

Avoid relying on `set_title()` for anything critical on Linux — treat it as a best-effort hint.

**Warning signs:**
- Badge count looks correct on macOS dev machine, looks wrong (or missing) on Linux test
- `set_title()` call succeeds (no error) but no visible text appears in tray
- KDE/Plasma shows icon but no text at all; GNOME Shell with AppIndicator extension shows text inline

**Phase to address:** System Tray + App Lifecycle

---

### Pitfall 7: MutexGuard Held Across Await Points in Batch Processing

**What goes wrong:**
The batch OCR path locks `SharedState` to read the pending file list, then holds the guard across multiple async Ollama HTTP calls. Either:
- The compiler rejects it: "future cannot be sent between threads safely" (`std::sync::MutexGuard` is not `Send`)
- Or the developer switches to `tokio::sync::Mutex` and the batch processing deadlocks when a second hotkey press fires while the first batch is in progress

**Why it happens:**
This bit v1 (per project memory: "Clone-before-await mutex pattern"). The batch path is more complex — multiple await points, state reads mid-flight — making the footgun larger. The instinct is to lock once and hold throughout the entire batch to prevent interleaving. That instinct is correct for intent but wrong for implementation.

**How to avoid:**
Use the established v1 clone-before-await pattern. Lock, clone out what you need, unlock, await, lock again to write back:
```rust
// Lock, clone, release
let pending: Vec<PathBuf> = {
    let state = app_state.lock().unwrap();
    state.pending_batch.clone()
}; // guard dropped here

// All await points happen with no lock held
let results = process_batch_concurrently(pending).await;

// Lock again to write results
{
    let mut state = app_state.lock().unwrap();
    state.apply_batch_results(results);
}
```
Never hold `std::sync::MutexGuard` across an `.await`.

**Warning signs:**
- "Future is not `Send`" compile error anywhere in batch processing command
- App hangs on second hotkey press when first batch is in progress
- Works in single-threaded tests but deadlocks under Tauri's multi-threaded Tokio runtime

**Phase to address:** Batch OCR + Merge Pipeline

---

### Pitfall 8: Config File Missing or Malformed Silently Uses Wrong Defaults

**What goes wrong:**
`~/.config/justfuckingcopy/config.toml` does not exist on first run (expected), or has a key with the wrong type (user typo). `toml::from_str()` on a plain struct returns `Err`. The app calls `unwrap_or_default()` and silently falls back to hardcoded defaults — including `watch_dir = ~/data/ssbnk/hosted`, which does not exist on any machine except the developer's. The watcher starts on a non-existent path, fires no events, and the app appears to do nothing at all.

**Why it happens:**
`#[serde(default)]` on individual fields applies when a key is *absent* — not when a key is present but the wrong type. A single malformed value fails the entire deserialization. `unwrap_or_default()` on the top-level result silently swallows parse errors. Developers test with their own config file and never hit this path.

**How to avoid:**
- Use `#[serde(default = "fn_name")]` on every field so absent keys use safe, portable defaults.
- On deserialization error, log explicitly (to stderr, a log file, or a tray notification) — never silently fall through.
- After loading config (or using defaults), validate that `watch_dir` exists and create it if absent; do not start the watcher on a non-existent path.
- Create the default config file on first run so users can discover the config format and edit it.

**Warning signs:**
- App works perfectly on dev machine, does nothing on any other machine
- `watch_dir` is a user-specific path (e.g., `/home/delorenj/data/ssbnk/hosted`) baked into `Default` impl
- No user-visible feedback when config has a typo

**Phase to address:** TOML Config

---

### Pitfall 9: macOS Dock Icon Appears for a Tray-Only App

**What goes wrong:**
On macOS, a Tauri app with no main window still shows a dock icon. For an ambient tray app, this is wrong UX — it creates a phantom dock presence, appears in Cmd+Tab, and confuses users who try to click it.

**Why it happens:**
macOS defaults all GUI applications to "regular" activation policy, which includes a dock icon and Cmd+Tab presence. Tauri does not change this automatically when no main window is created.

**How to avoid:**
Set the activation policy to `Accessory` in `setup()` on macOS:
```rust
#[cfg(target_os = "macos")]
app.set_activation_policy(tauri::ActivationPolicy::Accessory);
```
This hides the dock icon and removes the app from Cmd+Tab. Must be called after the app is built, inside `setup()`.

**Warning signs:**
- Dock icon visible on macOS during testing
- App appears in Cmd+Tab switcher despite having no window
- Clicking dock icon does nothing (no window to raise)

**Phase to address:** System Tray + App Lifecycle

---

### Pitfall 10: Linux Tray Requires Runtime Library Not Present by Default

**What goes wrong:**
On Linux, the tray icon never appears. No error is surfaced in the Tauri logs. The app launches, the watcher starts, but there is no tray icon and no way to interact with the app or quit it cleanly.

**Why it happens:**
Tauri's tray on Linux requires either `libayatana-appindicator` or `libappindicator3` at runtime. Neither is installed by default on many distributions (bare Debian, Arch, some Ubuntu variants). Tauri determines which library to use at runtime — if neither is present, the tray silently fails to initialize.

**How to avoid:**
- Document the dependency requirement prominently.
- At startup, attempt to detect tray initialization success. If tray icon creation fails, write a clear error to stderr and exit rather than running silently without a tray.
- In CI or testing environments, install `libayatana-appindicator3-dev` explicitly.

**Warning signs:**
- App launches silently but no tray icon appears on Linux
- `cargo tauri dev` succeeds with no errors but tray is absent
- Works on Ubuntu (which ships AppIndicator) but not on Arch or minimal Debian

**Phase to address:** System Tray + App Lifecycle

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Hardcode `watch_dir` in `Default` impl | Simpler first run | Breaks on every machine that isn't dev's | Never — load from config from day one |
| Skip debounce, process on first `Create` event | Simpler watcher | Intermittent corrupt PNG / empty OCR | Never — 500ms debounce is 5 lines |
| `unwrap_or_default()` on config parse result | No error handling | Silent wrong config, zero user feedback | Never in the production path |
| Fire all OCR concurrently, process results as they arrive | Maximum throughput | Non-deterministic merge order with 3+ files | Never — sort by mtime, trivial fix |
| Skip `is_registered()` check after hotkey registration | Fewer lines | Silent failure, user has no feedback | Never |
| Use `set_title()` for badge count instead of pre-rendered icons | Simple implementation | Broken on Linux, no overlay on any platform | Acceptable as Phase 1 scaffold; fix before milestone complete |
| Create tray in both `tauri.conf.json` and `setup()` | Follows some examples | Duplicate icon on every platform | Never |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `notify-rs` + screenshots | Watch for `Create`, process immediately | `notify-debouncer-full` + 500ms window + retry-open loop |
| `notify-rs` on macOS (kqueue) | Expect `Create` events for new files | Watch parent dir; `Access(Close(Write))` is the reliable event on kqueue |
| `tauri-plugin-global-shortcut` | Call `register()` in `main()` before `run()` | Register inside `app.setup()` closure only |
| `tauri-plugin-global-shortcut` | Treat `Ok(())` as "hotkey is active" | Verify with `is_registered()` after every `register()` call |
| Ollama batch requests | `join_all` over all files, merge results as they arrive | Tag with index, sort by index before merge pipeline |
| Tray icon initialization | Init in both `tauri.conf.json` AND `setup()` Rust code | Pick one path — Rust `setup()` for dynamic behavior |
| macOS tray-only app | Default activation policy shows dock icon | Set `ActivationPolicy::Accessory` in `setup()` |
| Linux tray visibility | Assume `libayatana-appindicator` installed | Document requirement; detect failure at startup |
| TOML config deserialization | `unwrap_or_default()` on parse result | Explicit error logging + watch_dir existence check |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| No debounce on watcher | CPU spike when screenshot tool writes temp files + renames | `notify-debouncer-full` with 500ms window | Immediately on any screenshot capture |
| Sequential OCR per file | 5-screenshot batch takes 5× single-image time | `join_all` over concurrent requests | Noticeable at 3+ screenshots |
| Re-scanning batch dir on every watcher event | O(n) scan grows as archive accumulates | Track pending set in `AppState`; do not re-scan | When archive subdir has 100+ old screenshots |
| PNG decode before spawning async OCR | Decode blocks executor; UI unresponsive | Decode inside the spawned future | 4K screenshots on slower hardware |
| Constructing `reqwest::Client` per OCR call | 50–100ms overhead per call even when Ollama is fast | Store client in `AppState` or use `OnceLock` singleton | Every OCR request in the batch |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Watching a world-writable directory (e.g., `/tmp`) | Attacker drops a crafted PNG to trigger OCR and clipboard injection | Default watch dir should be user-owned; validate ownership at startup |
| Config file readable by other users | Ollama endpoint / hotkey leaks | `~/.config/justfuckingcopy/` should be mode 700 on creation |
| No size check before sending PNG to Ollama | Attacker drops 500MB PNG causing OOM in decode step | Cap file size before decode (e.g., reject files > 20MB) |

---

## "Looks Done But Isn't" Checklist

- [ ] **Tray lifecycle:** Close the status panel window — confirm tray icon remains, hotkey still fires, watcher still detects new files
- [ ] **No dock icon (macOS):** Launch on macOS — confirm no icon in Dock and app absent from Cmd+Tab
- [ ] **Badge count:** Add screenshots, confirm badge increments; trigger batch, confirm badge resets to zero
- [ ] **Badge on Linux:** Verify badge count is visible on Linux using pre-rendered icon approach (not `set_title()`)
- [ ] **Hotkey registration:** Call `is_registered()` after `register()` and confirm it returns `true`; test with GNOME Terminal running
- [ ] **File ordering:** Add 3 screenshots with distinct text, trigger OCR — merged output order matches file creation time order
- [ ] **Config first run:** Delete `~/.config/justfuckingcopy/config.toml`, run app — confirm it creates default config and default watch dir, logs clearly
- [ ] **Config malformed:** Put invalid TOML in config — confirm tray or log shows parse error, does not silently proceed with wrong defaults
- [ ] **Watch dir non-existent:** Set `watch_dir` to non-existent path — confirm app creates it or surfaces a clear error
- [ ] **Archive lifecycle:** After processing, confirm screenshots move to archive subdir and pending count resets; confirm watcher continues for new files
- [ ] **Linux tray dependency:** On Linux without `libayatana-appindicator`, confirm app surfaces clear error rather than silent invisible tray
- [ ] **Concurrent hotkey press:** Press hotkey twice in quick succession while batch is running — confirm no deadlock, second press either queues or returns "already processing"

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| App exits on window close | LOW | Add `prevent_close` + `prevent_exit` handlers in `lib.rs`, rebuild |
| Duplicate tray icon | LOW | Remove `systemTray` from `tauri.conf.json`, rebuild |
| PNG corrupt on early read | LOW | Add `notify-debouncer-full` + retry loop in watcher handler |
| OCR results out of order | LOW | Sort by mtime index before merge pipeline — one sort call |
| Hotkey silent fail | MEDIUM | Add `is_registered()` check, surface tray notification, expose config key |
| Badge count broken on Linux | MEDIUM | Generate pre-rendered icon set (0–9+), replace `set_title` calls with `set_icon` |
| Config parse silent failure | LOW | Wrap in explicit error log, validate watch_dir exists post-parse |
| MutexGuard across await | MEDIUM | Refactor to clone-before-await pattern (established pattern from v1) |
| macOS dock icon | LOW | Add `set_activation_policy(Accessory)` in setup, one line |
| Linux tray missing | MEDIUM | Document dependency, add startup detection with clear error message |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| App exits on window close | System Tray + App Lifecycle | Close panel; tray remains; hotkey still fires |
| Duplicate tray icons | System Tray + App Lifecycle | Visual inspection; single icon on all platforms |
| Badge no Linux overlay | System Tray + App Lifecycle | Test on Linux; pre-rendered icon shows correct count |
| macOS dock icon visible | System Tray + App Lifecycle | Launch on macOS; Dock shows no icon |
| Linux tray missing silently | System Tray + App Lifecycle | Remove libayatana; app errors clearly rather than silently |
| PNG incomplete write | Directory Watcher | Add screenshot; OCR result never empty/corrupt across 10 runs |
| Config silent failure | TOML Config | Delete config; run; defaults applied with visible warning |
| Config malformed silently | TOML Config | Bad TOML; error surfaced in tray or logs |
| Watch dir non-existent | TOML Config | Non-existent watch_dir; app creates it or shows clear error |
| Hotkey silent fail | Global Hotkey | `is_registered()` true after register; competing app present |
| OCR out of order | Batch OCR + Merge Pipeline | 3-file batch; merged text order matches file creation order |
| MutexGuard across await | Batch OCR + Merge Pipeline | Concurrent hotkey presses; no deadlock, compiles cleanly |
| Concurrent OCR no ordering | Batch OCR + Merge Pipeline | 5-screenshot batch repeated 10 times; output order always consistent |

---

## Sources

- [Tauri 2 System Tray official docs](https://v2.tauri.app/learn/system-tray/)
- [Tauri 2 Global Shortcut plugin](https://v2.tauri.app/plugin/global-shortcut/)
- [Tauri GitHub Discussion #11489 — tray-only app, close all windows without exit](https://github.com/tauri-apps/tauri/discussions/11489)
- [Tauri GitHub Issue #13511 — prevent exit when all windows closed](https://github.com/tauri-apps/tauri/issues/13511)
- [Tauri GitHub Issue #8982 — multiple tray icons bug](https://github.com/tauri-apps/tauri/issues/8982)
- [Tauri GitHub Issue #8825 — tray menu does not open without second menu](https://github.com/tauri-apps/tauri/issues/8825)
- [Tauri plugins-workspace Issue #965 — global-shortcut handler parameter change](https://github.com/tauri-apps/plugins-workspace/issues/965)
- [Tauri Discussion #10017 — unable to register global shortcuts](https://github.com/tauri-apps/tauri/discussions/10017)
- [notify-rs GitHub — cross-platform filesystem notification library](https://github.com/notify-rs/notify)
- [notify-rs Issue #267 — fails to report creation events if they happen too quickly](https://github.com/notify-rs/notify/issues/267)
- [notify-rs Issue #365 — Create event for kqueue missing](https://github.com/notify-rs/notify/issues/365)
- [notify-debouncer-full docs](https://docs.rs/notify-debouncer-full)
- [Tokio shared state — std::sync::Mutex vs tokio::sync::Mutex](https://tokio.rs/tokio/tutorial/shared-state)
- [Tauri Discussion #6531 — async function inside state management](https://github.com/tauri-apps/tauri/discussions/6531)
- [tray-icon crate README — Linux AppIndicator requirements](https://github.com/tauri-apps/tray-icon)
- [Serde field attributes — #[serde(default)]](https://serde.rs/field-attrs.html)
- Project memory: JFC Ollama OCR Migration — clone-before-await mutex pattern (verified in v1 implementation)

---
*Pitfalls research for: Tauri 2 ambient tray app (system tray, directory watcher, global hotkey, batch OCR, TOML config)*
*Researched: 2026-03-21*
