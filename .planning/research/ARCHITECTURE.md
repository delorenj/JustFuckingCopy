# Architecture Research

**Domain:** Tauri 2 ambient tray app — system tray, directory watcher, global hotkey, TOML config
**Researched:** 2026-03-21
**Confidence:** HIGH (Tauri 2 official docs + crates.io verified)

---

## Context: v2.0 Integration onto v1.0 Foundation

The existing v1.0 codebase is already correct and should be disturbed as little as possible. The v2.0 changes are additive: four new capabilities slot into the existing architecture without rewriting any existing module logic.

**Existing modules — carry forward unchanged:**

| Module | Status | Notes |
|--------|--------|-------|
| `merge.rs` | Unchanged | Pure algorithm, no integration surface |
| `ollama.rs` | Unchanged | HTTP client for GLM-OCR; called by new `watcher.rs` |
| `platform.rs` | Unchanged | Screenshot capture + crop_png not needed in v2.0 but stays |
| `state.rs` | Extended | Add `BatchState` fields; existing `AppState` fields stay |
| `lib.rs` | Extended | Add tray setup, plugin registration, new commands |

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     OS Desktop Layer                             │
│  ┌──────────┐  ┌────────────────┐  ┌───────────────────────┐   │
│  │  Tray    │  │  Watch Dir     │  │  Global Hotkey        │   │
│  │  Icon    │  │  ~/data/ssbnk/ │  │  Ctrl+Shift+C         │   │
│  └────┬─────┘  └───────┬────────┘  └───────────┬───────────┘   │
└───────┼────────────────┼───────────────────────┼───────────────┘
        │ click          │ new .png file          │ keypress
        ▼                ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Rust Backend (src-tauri/src/)                │
│                                                                  │
│  lib.rs  ──── setup() ─────────────────────────────────────┐   │
│               │                                             │   │
│               ├── TrayIconBuilder (tauri built-in)          │   │
│               ├── tauri-plugin-global-shortcut              │   │
│               └── watcher::start()  ──► spawn OS thread     │   │
│                                          │                  │   │
│  ┌────────────┐    ┌──────────────┐      │ file event       │   │
│  │  config.rs │    │  watcher.rs  │◄─────┘                  │   │
│  │  (TOML)    │    │  (notify)    │                         │   │
│  └────────────┘    └──────┬───────┘                         │   │
│         │                 │ path of new PNG                 │   │
│         │ Config          ▼                                 │   │
│         └──────► state.rs BatchState ◄──── hotkey handler   │   │
│                      │                                      │   │
│                      │ Vec<PathBuf> pending_files           │   │
│                      ▼                                      │   │
│                   lib.rs process_batch()                    │   │
│                      │                                      │   │
│                      ├── ollama::recognize_text() [per file]│   │
│                      ├── merge::append_text() [existing]    │   │
│                      ├── clipboard write                    │   │
│                      └── archive files + reset batch        │   │
└─────────────────────────────────────────────────────────────────┘
        │ emit("batch-updated", payload)
        ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Status Panel (ui/)                           │
│  app.js  listen("batch-updated") → render file list + preview   │
│  Panel opens on tray left-click, hides on close                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## New Modules

### `config.rs` — TOML Configuration

**Responsibility:** Load, parse, and provide defaults for `~/.config/justfuckingcopy/config.toml`. Exposed as `AppConfig` loaded once at startup and stored via `app.manage()`.

```rust
// src-tauri/src/config.rs
#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub watch_dir: PathBuf,
    pub hotkey: String,
    pub ollama_endpoint: String,
    pub archive_subdir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            watch_dir: dirs::home_dir()
                .unwrap_or_default()
                .join("data/ssbnk/hosted"),
            hotkey: "Ctrl+Shift+C".to_string(),
            ollama_endpoint: "http://192.168.1.12:11434".to_string(),
            archive_subdir: ".jfc-archive".to_string(),
        }
    }
}

pub fn load() -> AppConfig {
    // 1. Resolve ~/.config/justfuckingcopy/config.toml
    // 2. If missing, return Default::default()
    // 3. Parse with toml::from_str — on parse error, log and return Default
}
```

**Communicates with:** `lib.rs` (loaded once in `run()`, stored as managed state), `watcher.rs` (gets `watch_dir`), `ollama.rs` (gets `ollama_endpoint`), hotkey registration (gets `hotkey` string).

**Dependencies to add:** `toml = "0.8"`, `dirs = "5"` — both small, no transitive complexity.

---

### `watcher.rs` — Directory File Watcher

**Responsibility:** Watch `config.watch_dir` for new PNG files. On detection, push the file path into `BatchState` and update the tray badge. Runs on a dedicated OS thread (not the async runtime).

**Key design:** The `notify` crate's `new_debouncer` runs a background thread internally. We provide a closure that receives debounced events and sends them via a `std::sync::mpsc::Sender<PathBuf>` to a second thread that calls `AppHandle::emit()` to notify the frontend and updates `SharedState`.

```rust
// src-tauri/src/watcher.rs
pub fn start(app: AppHandle, watch_dir: PathBuf) {
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(500),
            move |res: notify_debouncer_mini::DebounceEventResult| {
                if let Ok(events) = res {
                    for event in events {
                        if event.kind.is_create() || event.kind.is_modify() {
                            for path in event.paths {
                                if path.extension().map(|e| e == "png").unwrap_or(false) {
                                    let _ = tx.send(path);
                                }
                            }
                        }
                    }
                }
            },
        ).expect("Failed to create debouncer");

        debouncer.watcher()
            .watch(&watch_dir, notify::RecursiveMode::NonRecursive)
            .expect("Failed to watch directory");

        // Keep watcher alive; process events
        for path in rx {
            // Push into BatchState, update tray badge, emit to frontend
            let state = app_clone.state::<SharedState>();
            let mut guard = state.inner.lock().unwrap();
            guard.batch.push(path.clone());
            let count = guard.batch.len();
            drop(guard);

            // Update tray badge via title (Linux/macOS compatible)
            if let Some(tray) = app_clone.tray_by_id("main") {
                let _ = tray.set_title(Some(&format!("[{count}]")));
                let _ = tray.set_tooltip(Some(&format!("{count} screenshot(s) pending")));
            }

            // Notify status panel if open
            let _ = app_clone.emit("batch-updated", count);
        }
    });
}
```

**Critical detail:** The debouncer must be kept alive for the watcher to function. Dropping it stops watching. Keep the `debouncer` binding alive for the lifetime of the thread loop. (HIGH confidence — confirmed in `notify-debouncer-mini` docs: "dropping the debouncer also ends the debouncer".)

**Dependencies to add:** `notify = "6"`, `notify-debouncer-mini = "0.4"`.

---

## Modified Modules

### `state.rs` — Add BatchState Fields

The existing `AppState` / `SharedState` pattern stays. Add batch fields to `AppState`:

```rust
// Additions to AppState in state.rs
pub struct AppState {
    // ... existing fields unchanged ...
    pub next_snapshot_id: u64,
    pub next_segment_id: u64,
    pub current_snapshot: Option<StoredSnapshot>,
    pub segments: Vec<StoredSegment>,
    pub merged_text: String,

    // NEW: v2.0 batch workflow
    pub batch: Vec<PathBuf>,       // files pending OCR
    pub batch_merged_text: String, // deduped result from batch
}
```

Add methods to `AppState`:

```rust
impl AppState {
    // ... existing methods unchanged ...

    pub fn push_batch_file(&mut self, path: PathBuf) {
        self.batch.push(path);
    }

    pub fn clear_batch(&mut self) {
        self.batch.clear();
        self.batch_merged_text.clear();
    }

    pub fn set_batch_result(&mut self, text: String) {
        self.batch_merged_text = text;
    }
}
```

Add `BatchPayload` for frontend serialization:

```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchPayload {
    pub file_count: usize,
    pub file_names: Vec<String>,
    pub merged_text: String,
}
```

**What does NOT change:** `StoredSnapshot`, `StoredSegment`, `SegmentPayload`, `AppStatePayload`, `rebuild_merge()` — all untouched. The v1.0 session model can coexist with the v2.0 batch model in the same `AppState` struct.

---

### `lib.rs` — Tray Setup, Plugin Registration, New Commands

Four changes to `lib.rs`:

**1. Add module declarations:**
```rust
mod config;
mod watcher;
```

**2. Register plugins in `run()`:**
```rust
pub fn run() {
    let config = config::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(SharedState::default())
        .manage(config.clone())  // Config accessible in commands via State<'_, AppConfig>
        .setup(move |app| {
            // Build tray icon
            let tray = tauri::tray::TrayIconBuilder::new()
                .id("main")
                .tooltip("JustFuckingCopy — 0 pending")
                .menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left, ..
                    } = event {
                        let app = tray.app_handle();
                        toggle_status_panel(app);
                    }
                })
                .build(app)?;

            // Register global hotkey
            app.handle()
                .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
            let app_handle = app.handle().clone();
            app.handle()
                .global_shortcut()
                .on_shortcut(&config.hotkey, move |_app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            process_batch_inner(&handle).await;
                        });
                    }
                })?;

            // Start directory watcher on background thread
            watcher::start(app.handle().clone(), config.watch_dir.clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            reset_session,
            capture_snapshot,
            commit_selection,
            undo_last_segment,
            copy_merged_text,
            get_batch_state,     // NEW
            process_batch,       // NEW
            clear_batch,         // NEW
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**3. Add `toggle_status_panel()` helper:**
```rust
fn toggle_status_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}
```

**4. Add new commands:**
```rust
#[tauri::command]
fn get_batch_state(state: State<'_, SharedState>) -> Result<BatchPayload, String> { ... }

#[tauri::command]
async fn process_batch(app: AppHandle, state: State<'_, SharedState>) -> Result<BatchPayload, String> {
    process_batch_inner(&app).await
}

#[tauri::command]
fn clear_batch(state: State<'_, SharedState>) -> Result<BatchPayload, String> { ... }
```

---

### `tauri.conf.json` — Tray-Only Window Config

Two changes:

**1. Start window hidden:**
```json
"windows": [
  {
    "label": "main",
    "title": "JustFuckingCopy",
    "width": 480,
    "height": 640,
    "visible": false,
    "skipTaskbar": true,
    "decorations": true
  }
]
```

**2. Add tray icon config (icon file must exist at `icons/tray.png`):**
```json
"trayIcon": {
  "id": "main",
  "iconPath": "icons/tray.png",
  "iconAsTemplate": true,
  "menuOnLeftClick": false
}
```

Note: `iconAsTemplate: true` on macOS makes the icon adapt to light/dark menu bar. On Linux it has no effect.

---

## Data Flow

### New File Detected Flow

```
OS file system event (new PNG in watch_dir)
    │
    ▼
notify-debouncer-mini (500ms debounce window)
    │ filtered: .png files only, create/modify events only
    ▼
watcher.rs mpsc::channel rx loop
    │ PathBuf
    ▼
state.rs AppState::push_batch_file(path)
    │
    ├── tray.set_title("[N]")          — badge count update
    └── app.emit("batch-updated", N)   — notify open status panel
```

### Hotkey Trigger Flow (Batch Processing)

```
Global hotkey (Ctrl+Shift+C)
    │
    ▼
tauri_plugin_global_shortcut handler
    │ spawn async task on Tauri runtime
    ▼
process_batch_inner(app: &AppHandle) [async]
    │
    │ lock → clone batch file paths → drop lock
    │
    ├── for each path:
    │       read PNG bytes from disk
    │       ollama::recognize_text(&bytes).await
    │       merge::append_text(existing, incoming)
    │
    ├── lock → store batch_merged_text → drop lock
    │
    ├── clipboard::write_text(merged_text)
    │
    ├── archive files to watch_dir/.jfc-archive/
    │
    ├── lock → clear batch → drop lock
    │
    ├── tray.set_title(None)         — reset badge
    └── app.emit("batch-processed", payload)
```

### Status Panel Open Flow

```
Tray icon left-click
    │
    ▼
on_tray_icon_event (TrayIconEvent::Click, MouseButton::Left)
    │
    ▼
toggle_status_panel(app)
    │ if hidden → show + set_focus
    │ if visible → hide
    ▼
Frontend app.js
    │ invoke("get_batch_state") on window show
    ▼
render file list + merged text preview
```

### State Lifecycle

```
App start:  BatchState { batch: [], batch_merged_text: "" }
File drop:  batch.push(path), tray badge = batch.len()
Hotkey:     OCR all → merge → clipboard → archive → batch.clear()
Clear:      batch.clear() without processing
```

---

## Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `config.rs` | Load TOML, provide typed defaults | `lib.rs` (startup), `watcher.rs` (watch_dir), `ollama.rs` (endpoint) |
| `watcher.rs` | Watch directory, debounce events, push to state | `state.rs` (push_batch_file), tray API (badge), `app.emit()` (frontend) |
| `state.rs` | Session + batch state machine | `lib.rs` (commands), `watcher.rs` (batch push), `merge.rs` (rebuild) |
| `lib.rs` | Command handlers, tray setup, plugin wiring | All modules |
| `merge.rs` | Pure dedup algorithm | `state.rs` (unchanged) |
| `ollama.rs` | HTTP OCR client | `lib.rs` process_batch_inner, `config.rs` (endpoint) |
| `platform.rs` | Screenshot capture, crop_png | Unused in v2.0 main flow, kept for backward compat |
| `ui/app.js` | Status panel render, tray click handler | `lib.rs` via `invoke()`, backend via `listen("batch-updated")` |

---

## Architectural Patterns

### Pattern 1: AppHandle Clone for Background Threads

**What:** Clone `AppHandle` before moving into a `std::thread::spawn` closure. `AppHandle` is `Clone + Send`, designed for exactly this use case.

**When to use:** Any background thread (watcher loop, file processing) that needs to emit events, access managed state, or interact with windows.

**Trade-offs:** `AppHandle` holds a reference to the running app. Cloning is cheap (Arc under the hood). No alternative exists for giving background threads app access.

```rust
let app_handle = app.handle().clone();
std::thread::spawn(move || {
    // app_handle is now owned by this thread
    let state = app_handle.state::<SharedState>();
    let _ = app_handle.emit("event-name", payload);
});
```

### Pattern 2: Lock-Clone-Drop Before Any Async Work

**What:** Acquire `std::sync::Mutex` guard, clone the data needed, drop the guard, then do async work. Re-acquire lock only to write results back.

**When to use:** Every async command that reads from or writes to `SharedState`. Already established in v1.0 `commit_selection`.

**Trade-offs:** Requires cloning data (usually small: Vec<PathBuf>, String). Prevents holding `MutexGuard` across `.await` which is a compile error with `std::sync::Mutex`.

```rust
// In process_batch_inner:
let paths: Vec<PathBuf> = {
    let guard = state.inner.lock().unwrap();
    guard.batch.clone()
};  // guard dropped — no lock held during OCR awaits

for path in &paths {
    let bytes = std::fs::read(path)?;
    let text = ollama::recognize_text(&bytes).await?;
    // ...
}

let mut guard = state.inner.lock().unwrap();
guard.set_batch_result(merged);
guard.clear_batch();
```

### Pattern 3: Single Managed State, Multiple Access Points

**What:** `app.manage(SharedState::default())` once in `run()`. Access via `State<'_, SharedState>` in commands, or `app.state::<SharedState>()` in background threads.

**When to use:** Consistent throughout the app. Both command handlers and background threads (watcher, hotkey handler) need batch state access.

**Trade-offs:** Single `Mutex` means serialized access. Acceptable for this app — OCR is the slow step (async, no lock held), not state mutation.

### Pattern 4: Tray Title as Badge (Linux/macOS)

**What:** Use `tray.set_title(Some("[N]"))` to show a count next to the tray icon. On macOS this appears as text in the menu bar. On Linux it appears if an icon is also set (which it is).

**When to use:** Preferred over OS badge APIs because Tauri 2 has no native badge count API. `set_title` is cross-platform.

**Trade-offs:** On macOS, menu bar text is visible and conventional. On Linux, behavior depends on desktop environment — GNOME may not show it. Clear `set_title(None)` when count returns to zero.

---

## Anti-Patterns

### Anti-Pattern 1: Spawning the Watcher Inside a Tauri Command

**What people do:** Register a `start_watching` Tauri command and call it from the frontend on app load.

**Why it's wrong:** If the frontend window isn't visible (tray-only launch), the command never gets called. The watcher must start unconditionally at app boot, not on frontend request.

**Do this instead:** Start the watcher in the `.setup()` closure in `run()`, which always executes regardless of window visibility.

### Anti-Pattern 2: Dropping the Debouncer

**What people do:** Create the debouncer inside a closure or short-lived function that returns, letting the debouncer drop.

**Why it's wrong:** `notify-debouncer-mini` explicitly documents that dropping the debouncer ends watching. The watcher silently stops with no error.

**Do this instead:** Keep the debouncer binding alive for the lifetime of the background thread (bind it before the `for path in rx` loop, not inside it).

### Anti-Pattern 3: Blocking OCR Inside the Watcher Event Handler

**What people do:** Call `ollama::recognize_text()` inside the file-event callback to "process immediately."

**Why it's wrong:** The watcher callback runs on the notify debouncer's internal thread. Blocking it with an async HTTP call prevents further events from being processed and holds the watcher thread.

**Do this instead:** The watcher only pushes `PathBuf` into `BatchState` and updates the badge. OCR runs separately, triggered by the hotkey or "process now" button, on Tauri's async runtime.

### Anti-Pattern 4: Writing Config Back to TOML on Every State Change

**What people do:** Serialize runtime state (like batch count) back into `config.toml` as a form of persistence.

**Why it's wrong:** Config is user-authored settings. Runtime state belongs in memory. The archive directory is the persistence model for processed files.

**Do this instead:** `config.toml` is read-only at runtime. Batch state lives in `AppState.batch`. Archive is written to disk after processing, not to config.

### Anti-Pattern 5: Using Global Hotkey Plugin Before `.setup()` Completes

**What people do:** Register shortcuts in a spawned thread before the Tauri event loop starts.

**Why it's wrong:** The global shortcut plugin must be registered via `.plugin()` on the builder before `.run()`, and shortcuts must be registered inside `.setup()` after the plugin is initialized.

**Do this instead:** Register the plugin in the builder chain with `.plugin(tauri_plugin_global_shortcut::Builder::new().build())`. Then in `.setup()`, call `app.handle().global_shortcut().on_shortcut(...)`.

---

## File System Layout — New and Modified Files

```
src-tauri/
├── src/
│   ├── config.rs        NEW  — TOML config loader, AppConfig struct
│   ├── watcher.rs       NEW  — notify-debouncer-mini directory watcher
│   ├── lib.rs           MOD  — tray setup, plugin registration, 3 new commands
│   ├── state.rs         MOD  — add batch: Vec<PathBuf>, batch_merged_text, BatchPayload
│   ├── ollama.rs        ---  — unchanged (endpoint becomes config-driven in v2.1)
│   ├── merge.rs         ---  — unchanged
│   └── platform.rs      ---  — unchanged
├── icons/
│   └── tray.png         NEW  — small icon for system tray (22x22 or 32x32 PNG)
├── Cargo.toml           MOD  — add toml, dirs, notify, notify-debouncer-mini,
│                               tauri-plugin-global-shortcut
└── tauri.conf.json      MOD  — window visible: false, skipTaskbar: true, trayIcon config
```

---

## Suggested Build Order

Dependencies between components drive this order. Each step is independently verifiable.

**Step 1: Cargo.toml + tauri.conf.json baseline**
Add all new dependencies. Change `tauri.conf.json` to hide the window on start and add tray icon config. Create placeholder `icons/tray.png`. Verify `cargo build` compiles. This step has zero logic risk — it is pure configuration.

**Step 2: `config.rs` — TOML config**
No Tauri dependencies; pure Rust + `toml` + `dirs`. Can be unit-tested independently. Wire into `lib.rs` via `app.manage(config)` and `State<'_, AppConfig>` in commands. Verifiable: add a `#[tauri::command] fn get_config()` test command and call from frontend.

**Step 3: `state.rs` batch fields**
Extend `AppState` with `batch` and `batch_merged_text`. Add `BatchPayload`, `push_batch_file()`, `clear_batch()`, `set_batch_result()`. All existing fields and methods stay unchanged. Verifiable: `cargo test` — existing merge tests still pass.

**Step 4: Tray icon + window hide/show**
Wire `TrayIconBuilder` in `lib.rs` `.setup()`. Implement `toggle_status_panel()`. Handle `WindowEvent::CloseRequested` to hide instead of close. Verifiable: app launches to tray, left-click shows/hides status panel window, close button hides window instead of quitting.

**Step 5: `watcher.rs` — file detection + badge**
Implement directory watcher using `notify-debouncer-mini`. Push paths to `BatchState`. Update tray title on file arrival. Emit `"batch-updated"` event. Verifiable: drop a PNG into watch directory, tray badge increments, status panel shows file name if open.

**Step 6: Global hotkey + `process_batch_inner()`**
Register hotkey in `.setup()` using `tauri-plugin-global-shortcut`. Implement `process_batch_inner()` as an async function: read files, OCR via `ollama::recognize_text()`, merge via `merge::append_text()`, write clipboard, archive files, reset batch state. Verifiable: hotkey fires, merged text lands in clipboard, badge resets, archived files appear in `.jfc-archive/`.

**Step 7: Status panel UI**
Update `ui/app.js` to listen for `"batch-updated"` and `"batch-processed"` events. Render file list and merged text preview. Add "Process Now" and "Clear" buttons wired to `process_batch` and `clear_batch` commands. Verifiable: status panel shows pending files and merged preview accurately.

**Build order rationale:**
- Step 1 before everything else: no logic compiles without the dependencies
- Step 2 before Step 6: hotkey string comes from config; config must be loadable first
- Step 3 before Steps 5 and 6: both need batch fields on AppState
- Step 4 before Steps 5/6: tray must exist before setting its badge title
- Step 5 before Step 6: watcher populates batch; batch processing needs files to process
- Step 7 last: frontend can be stubbed with direct invoke() calls during backend steps

---

## Integration Points with Existing Modules

| Existing Module | v2.0 Integration | Change Type |
|----------------|-----------------|-------------|
| `ollama.rs` | `process_batch_inner()` calls `ollama::recognize_text()` per file | Caller changes; module unchanged |
| `merge.rs` | `process_batch_inner()` calls `append_text()` for each OCR result | Caller changes; module unchanged |
| `state.rs` | New batch fields added; existing fields stay | Additive extension |
| `lib.rs` | New plugin registrations, tray setup, 3 new commands | Additive extension |
| `platform.rs` | Not called in v2.0 main flow; `capture_snapshot` command still registered | No change needed |
| `ui/app.js` | New event listeners, new invoke() calls for batch commands | Additive extension |

---

## Scaling Considerations

This is a single-user desktop app. Relevant operational limits:

| Concern | Practical Limit | Mitigation |
|---------|----------------|------------|
| Batch size | GLM-OCR is slow (~5-15s per image); large batches block hotkey response | Process sequentially with progress emit per file; user sees progress |
| Watch directory depth | `NonRecursive` mode avoids watching subdirectories (including archive subdir) | Use `RecursiveMode::NonRecursive` in watcher |
| Archive accumulation | Archive dir grows unbounded over time | Out of scope for v2.0; document as known limitation |
| Concurrent hotkey presses | Second hotkey fires while batch processing in progress | Guard with an `Arc<AtomicBool> processing_flag` in SharedState; skip if already running |

---

## Sources

- [Tauri 2 — System Tray](https://v2.tauri.app/learn/system-tray/) — HIGH confidence
- [Tauri 2 — Global Shortcut Plugin](https://v2.tauri.app/plugin/global-shortcut/) — HIGH confidence
- [Tauri 2 — Calling Frontend from Rust (emit)](https://v2.tauri.app/develop/calling-frontend/) — HIGH confidence
- [notify-debouncer-mini docs.rs](https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/) — HIGH confidence
- [notify-rs GitHub](https://github.com/notify-rs/notify) — HIGH confidence
- [tauri-plugin-global-shortcut crates.io](https://crates.io/crates/tauri-plugin-global-shortcut) — HIGH confidence (v2.2.1 confirmed)
- [toml crate docs.rs](https://docs.rs/toml) — HIGH confidence
- [dirs crate crates.io](https://crates.io/crates/dirs) — HIGH confidence
- [Tauri Discussion #11489 — tray-only app, ExitRequested pattern](https://github.com/tauri-apps/tauri/discussions/11489) — HIGH confidence
- [Tauri tray JS API reference](https://v2.tauri.app/reference/javascript/api/namespacetray/) — HIGH confidence (set_title for badge confirmed)
- [Stack Overflow — Tauri CloseRequested hide instead of close](https://stackoverflow.com/questions/77856626/close-tauri-window-without-closing-the-entire-app) — MEDIUM confidence

---

*Architecture research for: JustFuckingCopy v2.0 ambient tray integration*
*Researched: 2026-03-21*
