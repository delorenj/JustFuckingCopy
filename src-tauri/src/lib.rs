mod merge;
mod ollama;
mod platform;
mod state;

use std::thread;
use std::time::Duration;

use tauri::{State, WebviewWindow};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::platform::{capture_snapshot as platform_capture_snapshot, crop_png};
use crate::state::{AppStatePayload, SelectionRect, SharedState, SnapshotPayload};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitSelectionRequest {
    snapshot_id: u64,
    selection: SelectionRect,
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
fn commit_selection(
    request: CommitSelectionRequest,
    state: State<'_, SharedState>,
) -> Result<AppStatePayload, String> {
    let mut guard = state
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
    let recognized_text = recognize_text_from_png(&crop)?;

    if recognized_text.trim().is_empty() {
        return Err("OCR returned no text. Try a tighter marquee or a clearer zoom level.".into());
    }

    guard.push_segment(snapshot.id, request.selection, recognized_text);
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
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(SharedState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            reset_session,
            capture_snapshot,
            commit_selection,
            undo_last_segment,
            copy_merged_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
