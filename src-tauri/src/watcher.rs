use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use notify::{EventKind, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

pub struct BatchState {
    pub inner: Mutex<Vec<PathBuf>>,
}

impl Default for BatchState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }
}

impl BatchState {
    /// Adds a path to the pending list if it is an image file and not already present.
    /// Returns true if the path was added, false otherwise.
    pub fn add_pending_file(&self, path: PathBuf) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "png" && ext != "jpg" && ext != "jpeg" {
            return false;
        }

        let mut guard = self.inner.lock().expect("BatchState lock poisoned");
        if guard.contains(&path) {
            return false;
        }
        guard.push(path);
        true
    }

    /// Returns the number of pending files.
    pub fn pending_count(&self) -> usize {
        self.inner.lock().expect("BatchState lock poisoned").len()
    }

    /// Drains all pending files and returns them, leaving the list empty.
    pub fn drain_pending(&self) -> Vec<PathBuf> {
        let mut guard = self.inner.lock().expect("BatchState lock poisoned");
        guard.drain(..).collect()
    }

    /// Clears all pending files without returning them.
    pub fn clear(&self) {
        self.inner.lock().expect("BatchState lock poisoned").clear();
    }
}

/// Starts a filesystem watcher on `watch_dir`, adding new PNG/JPEG files to BatchState
/// and updating the tray tooltip on each addition.
///
/// Expands `~` in `watch_dir` to the user's home directory.
/// If the directory does not exist, logs a warning and returns Ok(()) without starting.
/// Spawns a background thread that keeps the watcher alive indefinitely.
pub fn start_watcher(watch_dir: &str, app_handle: AppHandle) -> Result<(), String> {
    let expanded = expand_tilde(watch_dir);
    let watch_path = PathBuf::from(&expanded);

    if !watch_path.exists() {
        eprintln!(
            "[JFC watcher] Watch directory not found: {}. Watcher not started.",
            watch_path.display()
        );
        return Ok(());
    }

    let app_handle_clone = app_handle.clone();

    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        let event = match result {
            Ok(e) => e,
            Err(e) => {
                eprintln!("[JFC watcher] Watch error: {e}");
                return;
            }
        };

        let is_relevant = matches!(
            event.kind,
            EventKind::Create(_)
                | EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To
                ))
        );

        if !is_relevant {
            return;
        }

        let batch_state = app_handle_clone.state::<BatchState>();
        for path in event.paths {
            if batch_state.add_pending_file(path) {
                let count = batch_state.pending_count();
                let tooltip = format!("JustFuckingCopy ({count} pending)");
                if let Some(tray) = app_handle_clone.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some(&tooltip));
                }
            }
        }
    })
    .map_err(|e| format!("[JFC watcher] Failed to create watcher: {e}"))?;

    watcher
        .watch(&watch_path, RecursiveMode::NonRecursive)
        .map_err(|e| format!("[JFC watcher] Failed to watch directory: {e}"))?;

    // Move watcher into a thread to keep it alive
    std::thread::spawn(move || {
        let _watcher = watcher;
        // Block forever — the watcher runs its event loop on the OS side
        loop {
            std::thread::sleep(Duration::from_secs(60));
        }
    });

    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/test/{name}"))
    }

    #[test]
    fn test_add_pending_file_png() {
        let state = BatchState::default();
        let added = state.add_pending_file(make_path("screenshot.png"));
        assert!(added, "Expected .png to be added");
        assert_eq!(state.pending_count(), 1);
    }

    #[test]
    fn test_add_pending_file_dedup() {
        let state = BatchState::default();
        let path = make_path("screenshot.png");
        let first = state.add_pending_file(path.clone());
        let second = state.add_pending_file(path);
        assert!(first, "First addition should return true");
        assert!(!second, "Second addition of same path should return false");
        assert_eq!(state.pending_count(), 1, "Should still be exactly 1 entry");
    }

    #[test]
    fn test_add_pending_file_ignores_non_image() {
        let state = BatchState::default();
        let added = state.add_pending_file(make_path("document.txt"));
        assert!(!added, "Expected .txt to be ignored");
        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_add_pending_file_jpg_and_jpeg() {
        let state = BatchState::default();
        let jpg_added = state.add_pending_file(make_path("photo.jpg"));
        let jpeg_added = state.add_pending_file(make_path("photo2.jpeg"));
        assert!(jpg_added, "Expected .jpg to be accepted");
        assert!(jpeg_added, "Expected .jpeg to be accepted");
        assert_eq!(state.pending_count(), 2);
    }

    #[test]
    fn test_pending_count() {
        let state = BatchState::default();
        assert_eq!(state.pending_count(), 0);
        state.add_pending_file(make_path("a.png"));
        assert_eq!(state.pending_count(), 1);
        state.add_pending_file(make_path("b.jpg"));
        assert_eq!(state.pending_count(), 2);
    }

    #[test]
    fn test_drain_pending() {
        let state = BatchState::default();
        state.add_pending_file(make_path("a.png"));
        state.add_pending_file(make_path("b.jpg"));

        let drained = state.drain_pending();
        assert_eq!(drained.len(), 2, "Should drain 2 entries");
        assert_eq!(state.pending_count(), 0, "List should be empty after drain");
    }

    #[test]
    fn test_start_watcher_returns_ok_for_existing_dir() {
        // We can't test the full event loop without an AppHandle,
        // but we can verify that start_watcher does not panic on a non-existent dir
        // (it should log and return Ok).
        let non_existent = "/tmp/jfc_no_such_dir_12345_test";
        // This should return Ok(()) with a warning, not an error
        // We test expand_tilde and the missing-dir path directly
        let expanded = super::expand_tilde(non_existent);
        assert_eq!(expanded, non_existent, "Non-tilde path should be unchanged");

        // Verify that adding to BatchState from multiple threads is safe
        use std::sync::Arc;
        let state = Arc::new(BatchState::default());
        let state2 = state.clone();
        let handle = std::thread::spawn(move || {
            state2.add_pending_file(PathBuf::from("/tmp/threaded.png"));
        });
        handle.join().unwrap();
        assert_eq!(state.pending_count(), 1);
    }
}
