use std::path::PathBuf;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use tauri::{AppHandle, Emitter, Manager};

/// Keeps the current document live: edits from any editor re-render instantly.
pub struct DocWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
}

pub fn watch(app: AppHandle, file: PathBuf) -> Result<DocWatcher, String> {
    let dir = file
        .parent()
        .ok_or_else(|| "file has no parent directory".to_string())?
        .to_path_buf();
    let fname = file
        .file_name()
        .ok_or_else(|| "path has no file name".to_string())?
        .to_os_string();

    let mut debouncer = new_debouncer(
        Duration::from_millis(200),
        move |res: DebounceEventResult| {
            let Ok(events) = res else { return };
            // Watch the whole directory (non-recursive) so atomic save-via-rename
            // that editors like VS Code use still hits our file name.
            let relevant = events.iter().any(|e| {
                e.path
                    .file_name()
                    .map(|n| n.eq_ignore_ascii_case(&fname))
                    .unwrap_or(false)
            });
            if !relevant {
                return;
            }
            match crate::render::render_path(&file) {
                Ok(doc) => {
                    let state = app.state::<crate::AppState>();
                    *state.doc.lock().unwrap() = Some(doc.clone());
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.set_title(&format!("{} — Marq", doc.title));
                    }
                    let _ = app.emit("doc", &doc);
                }
                Err(_) => {
                    // Transient: editors often delete+recreate on save. Keep showing
                    // the last good render; the next event re-renders.
                }
            }
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    Ok(DocWatcher {
        _debouncer: debouncer,
    })
}
