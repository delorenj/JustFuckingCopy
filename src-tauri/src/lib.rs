mod config;
mod merge;
mod ollama;
mod platform;
mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Manager, RunEvent, State, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::platform::{capture_snapshot as platform_capture_snapshot, crop_png};
use crate::state::{AppStatePayload, SelectionRect, SharedState, SnapshotPayload};

const MAIN_WINDOW_LABEL: &str = "main";
const TOGGLE_STATUS_PANEL_MENU_ID: &str = "tray-toggle-status-panel";
const QUIT_APP_MENU_ID: &str = "tray-quit-app";
const TRAY_TOOLTIP: &str = "JustFuckingCopy";
const TRAY_ICON: tauri::image::Image<'_> = tauri::include_image!("./icons/icon.png");

#[derive(Default)]
struct LifecycleState {
    allow_exit: AtomicBool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitSelectionRequest {
    snapshot_id: u64,
    selection: SelectionRect,
}

fn ensure_status_panel(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW_LABEL)
        .ok_or_else(|| format!("Missing `{MAIN_WINDOW_LABEL}` window configuration."))?;

    let window = WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|error| format!("Failed to prepare status panel window: {error}"))?
        .build()
        .map_err(|error| format!("Failed to build status panel window: {error}"))?;

    attach_status_panel_handlers(&window);

    Ok(window)
}

fn attach_status_panel_handlers(window: &WebviewWindow) {
    let panel = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = panel.hide();
        }
    });
}

fn show_status_panel(app: &AppHandle) -> Result<(), String> {
    let window = ensure_status_panel(app)?;
    window
        .show()
        .map_err(|error| format!("Failed to show status panel: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("Failed to focus status panel: {error}"))?;
    Ok(())
}

fn hide_status_panel(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window
            .hide()
            .map_err(|error| format!("Failed to hide status panel: {error}"))?;
    }

    Ok(())
}

fn toggle_status_panel(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        if window
            .is_visible()
            .map_err(|error| format!("Failed to inspect status panel visibility: {error}"))?
        {
            hide_status_panel(app)?;
        } else {
            show_status_panel(app)?;
        }
    } else {
        show_status_panel(app)?;
    }

    Ok(())
}

fn request_app_exit(app: &AppHandle) {
    app.state::<LifecycleState>()
        .allow_exit
        .store(true, Ordering::SeqCst);
    app.exit(0);
}

fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id == TOGGLE_STATUS_PANEL_MENU_ID {
        if let Err(error) = toggle_status_panel(app) {
            eprintln!("Failed to toggle status panel from tray menu: {error}");
        }
    } else if event.id == QUIT_APP_MENU_ID {
        request_app_exit(app);
    }
}

fn handle_tray_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        if let Err(error) = toggle_status_panel(app) {
            eprintln!("Failed to toggle status panel from tray click: {error}");
        }
    }
}

fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        app.set_dock_visibility(false);
    }

    let toggle_item = MenuItem::with_id(
        app,
        TOGGLE_STATUS_PANEL_MENU_ID,
        "Toggle Status Panel",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(
        app,
        QUIT_APP_MENU_ID,
        "Quit JustFuckingCopy",
        true,
        None::<&str>,
    )?;
    let tray_menu = Menu::with_items(app, &[&toggle_item, &separator, &quit_item])?;

    TrayIconBuilder::with_id(MAIN_WINDOW_LABEL)
        .icon(TRAY_ICON)
        .tooltip(TRAY_TOOLTIP)
        .icon_as_template(true)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_tray_menu_event)
        .on_tray_icon_event(|tray, event| handle_tray_icon_event(tray.app_handle(), event))
        .build(app)
        .map_err(|error| {
            let message = if cfg!(target_os = "linux") {
                format!(
                    "Failed to initialize the tray icon. On Linux, ensure an AppIndicator-compatible library is installed: {error}"
                )
            } else {
                format!("Failed to initialize the tray icon: {error}")
            };

            std::io::Error::other(message)
        })?;

    Ok(())
}

#[tauri::command]
fn get_app_state(state: State<'_, SharedState>) -> Result<AppStatePayload, String> {
    let guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;
    Ok(guard.to_payload())
}

#[tauri::command]
fn reset_session(state: State<'_, SharedState>) -> Result<AppStatePayload, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;
    guard.clear();
    Ok(guard.to_payload())
}

#[tauri::command]
fn capture_snapshot(
    window: WebviewWindow,
    state: State<'_, SharedState>,
) -> Result<SnapshotPayload, String> {
    window
        .hide()
        .map_err(|error| format!("Failed to hide window: {error}"))?;
    thread::sleep(Duration::from_millis(250));

    let capture_result = platform_capture_snapshot();

    window
        .show()
        .map_err(|error| format!("Failed to show window: {error}"))?;
    let _ = window.set_focus();

    let (png_bytes, width, height) = capture_result?;

    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;

    Ok(guard.store_snapshot(png_bytes, width, height))
}

#[tauri::command]
async fn commit_selection(
    request: CommitSelectionRequest,
    state: State<'_, SharedState>,
) -> Result<AppStatePayload, String> {
    // Lock scope 1: extract data needed before the await, then drop the guard
    let (snapshot_id, crop) = {
        let guard = state
            .inner
            .lock()
            .map_err(|_| "State lock was poisoned.".to_string())?;

        let snapshot = guard
            .current_snapshot
            .clone()
            .ok_or_else(|| "Capture a snapshot before committing a selection.".to_string())?;

        if snapshot.id != request.snapshot_id {
            return Err("The snapshot changed before this selection was committed.".into());
        }

        let crop = crop_png(
            &snapshot.png_bytes,
            request.selection.x,
            request.selection.y,
            request.selection.width,
            request.selection.height,
        )?;

        (snapshot.id, crop)
        // guard drops here — MutexGuard is NOT held across .await
    };

    // Await outside the lock scope
    let recognized_text = ollama::recognize_text(&crop).await?;

    if recognized_text.trim().is_empty() {
        return Err("OCR returned no text. Try a tighter marquee or a clearer zoom level.".into());
    }

    // Lock scope 2: write results back into state
    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;
    guard.push_segment(snapshot_id, request.selection, recognized_text);
    Ok(guard.to_payload())
}

#[tauri::command]
fn undo_last_segment(state: State<'_, SharedState>) -> Result<AppStatePayload, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "State lock was poisoned.".to_string())?;

    guard.undo_last_segment();
    Ok(guard.to_payload())
}

#[tauri::command]
fn copy_merged_text(
    app: tauri::AppHandle,
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let merged_text = {
        let guard = state
            .inner
            .lock()
            .map_err(|_| "State lock was poisoned.".to_string())?;
        guard.merged_text.clone()
    };

    if merged_text.trim().is_empty() {
        return Err("There is no merged text to copy yet.".into());
    }

    app.clipboard()
        .write_text(merged_text.clone())
        .map_err(|error| format!("Failed to write merged text to clipboard: {error}"))?;

    Ok(merged_text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_config = config::load_or_create();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(SharedState::default())
        .manage(LifecycleState::default())
        .manage(app_config)
        .setup(|app| setup_tray(app))
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            reset_session,
            capture_snapshot,
            commit_selection,
            undo_last_segment,
            copy_merged_text
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app, event| {
        if let RunEvent::ExitRequested { api, .. } = event {
            let lifecycle_state = app.state::<LifecycleState>();
            if !lifecycle_state.allow_exit.load(Ordering::SeqCst) {
                api.prevent_exit();
            }
        }
    });
}
