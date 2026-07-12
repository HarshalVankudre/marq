#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod assoc;
mod render;
mod watch;
#[cfg(windows)]
mod webview2;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use tauri::menu::{CheckMenuItem, MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt as _;
use tauri_plugin_opener::OpenerExt as _;

#[derive(Default)]
pub struct AppState {
    pub doc: Mutex<Option<render::RenderResult>>,
    watcher: Mutex<Option<watch::DocWatcher>>,
    /// When set, the next `mark_rendered` triggers a silent PDF export: (out_path, exit_after).
    pdf_auto: Mutex<Option<(PathBuf, bool)>>,
    /// Bumped on every show/hide; lets the idle-destroy timer detect staleness.
    hide_epoch: AtomicU64,
}

/// How long the window may sit hidden before the webview is torn down to
/// shrink the tray footprint to just the Rust process. Reopening rebuilds it.
const IDLE_DESTROY_SECS: u64 = 300;

const MD_EXTS: [&str; 5] = ["md", "markdown", "mdown", "mkd", "mdtxt"];

fn is_markdown(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| MD_EXTS.iter().any(|m| e.eq_ignore_ascii_case(m)))
        .unwrap_or(false)
}

fn is_dev_exe() -> bool {
    std::env::current_exe()
        .map(|p| p.components().any(|c| c.as_os_str() == "target"))
        .unwrap_or(false)
}

// ---------------------------------------------------------------- commands

#[tauri::command]
fn get_current_doc(state: tauri::State<'_, AppState>) -> Option<render::RenderResult> {
    state.doc.lock().unwrap().clone()
}

#[tauri::command]
fn get_highlight_css() -> render::HighlightCss {
    render::highlight_css()
}

/// The frontend calls this after the document has painted; used to drive `--pdf` exports.
#[tauri::command]
fn mark_rendered(app: AppHandle, window: WebviewWindow) {
    let pending = app.state::<AppState>().pdf_auto.lock().unwrap().take();
    let Some((out, exit_after)) = pending else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        #[cfg(windows)]
        let res = webview2::print_to_pdf(window, out.clone()).await;
        #[cfg(not(windows))]
        let res: Result<(), String> = {
            let _ = window;
            Err("PDF export is Windows-only".into())
        };
        match &res {
            Ok(()) => eprintln!("pdf: wrote {}", out.display()),
            Err(e) => eprintln!("pdf: failed: {e}"),
        }
        if exit_after {
            app.exit(if res.is_ok() { 0 } else { 1 });
        } else {
            let msg = match res {
                Ok(()) => format!("Exported {}", out.display()),
                Err(e) => format!("PDF export failed: {e}"),
            };
            let _ = app.emit("toast", msg);
        }
    });
}

#[tauri::command]
async fn export_pdf(app: AppHandle, window: WebviewWindow) -> Result<Option<String>, String> {
    let (start_dir, file_name) = {
        let state = app.state::<AppState>();
        let doc = state.doc.lock().unwrap();
        match doc.as_ref() {
            Some(d) => {
                let p = PathBuf::from(&d.path);
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("document")
                    .to_string();
                (p.parent().map(|d| d.to_path_buf()), format!("{stem}.pdf"))
            }
            None => return Err("Open a Markdown file first.".into()),
        }
    };

    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dlg = app
        .dialog()
        .file()
        .add_filter("PDF document", &["pdf"])
        .set_file_name(&file_name);
    if let Some(dir) = start_dir {
        dlg = dlg.set_directory(dir);
    }
    dlg.save_file(move |fp| {
        let _ = tx.send(fp);
    });
    let picked = rx.await.map_err(|_| "dialog closed unexpectedly".to_string())?;
    let Some(fp) = picked else { return Ok(None) };
    let out = fp.into_path().map_err(|e| e.to_string())?;

    #[cfg(windows)]
    {
        webview2::print_to_pdf(window, out.clone()).await?;
        Ok(Some(out.display().to_string()))
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        Err("PDF export is Windows-only".into())
    }
}

#[tauri::command]
fn open_path(app: AppHandle, path: String, base: Option<String>) -> Result<(), String> {
    let mut p = PathBuf::from(&path);
    if p.is_relative() {
        match base {
            Some(b) if !b.is_empty() => p = PathBuf::from(b).join(p),
            _ => return Err(format!("Cannot resolve relative path {path}")),
        }
    }
    let p = dunce::canonicalize(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    if is_markdown(&p) {
        open_doc(&app, &p, true, None, false)
    } else {
        app.opener()
            .open_path(p.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
        app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
    } else {
        Err("Only http(s) and mailto links open externally".into())
    }
}

#[tauri::command]
fn open_file_dialog(app: AppHandle) {
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("Markdown", &MD_EXTS)
        .pick_file(move |fp| {
            if let Some(fp) = fp {
                if let Ok(p) = fp.into_path() {
                    if let Err(e) = open_doc(&handle, &p, true, None, false) {
                        let _ = handle.emit("toast", e);
                    }
                }
            }
        });
}

#[tauri::command]
fn set_default_app(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        assoc::register_current_exe().map_err(|e| e.to_string())?;
        assoc::open_default_apps_settings(&app);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("Windows-only".into())
    }
}

// ---------------------------------------------------------------- core flow

fn open_doc(
    app: &AppHandle,
    path: &Path,
    show: bool,
    pdf_out: Option<PathBuf>,
    exit_after_pdf: bool,
) -> Result<(), String> {
    let path = dunce::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = render::render_path(&path)?;

    let state = app.state::<AppState>();
    *state.doc.lock().unwrap() = Some(doc.clone());
    if let Some(out) = pdf_out {
        *state.pdf_auto.lock().unwrap() = Some((out, exit_after_pdf));
    }
    *state.watcher.lock().unwrap() = watch::watch(app.clone(), path.clone()).ok();

    if show {
        show_main_window(app);
    }
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_title(&format!("{} — Marq", doc.title));
    }
    let _ = app.emit("doc", &doc);
    Ok(())
}

fn attach_close_handler(app: &AppHandle, win: &WebviewWindow) {
    let h = app.clone();
    win.on_window_event(move |ev| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = ev {
            // Close = park in tray; quitting is explicit via the tray menu.
            api.prevent_close();
            hide_main_window(&h);
        }
    });
}

/// The main window is destroyed after sitting hidden a while; recreate it
/// from the config on demand.
fn ensure_main_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(win) = app.get_webview_window("main") {
        return Some(win);
    }
    let cfg = app.config().app.windows.first()?.clone();
    match tauri::WebviewWindowBuilder::from_config(app, &cfg) {
        Ok(builder) => match builder.build() {
            Ok(win) => {
                attach_close_handler(app, &win);
                Some(win)
            }
            Err(e) => {
                eprintln!("marq: failed to rebuild window: {e}");
                None
            }
        },
        Err(e) => {
            eprintln!("marq: bad window config: {e}");
            None
        }
    }
}

fn show_main_window(app: &AppHandle) {
    app.state::<AppState>().hide_epoch.fetch_add(1, Ordering::SeqCst);
    let Some(win) = ensure_main_window(app) else { return };
    #[cfg(windows)]
    webview2::set_webview_visible(&win, true);
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
}

fn hide_main_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    let _ = win.hide();
    // Drop most of the renderer's memory while parked in the tray...
    #[cfg(windows)]
    webview2::set_webview_visible(&win, false);
    // ...and after a while, drop the webview entirely.
    let epoch = app.state::<AppState>().hide_epoch.fetch_add(1, Ordering::SeqCst) + 1;
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(IDLE_DESTROY_SECS)).await;
        if handle.state::<AppState>().hide_epoch.load(Ordering::SeqCst) != epoch {
            return; // shown or re-hidden since
        }
        if let Some(win) = handle.get_webview_window("main") {
            if win.is_visible().unwrap_or(true) {
                return;
            }
            let h2 = handle.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(win) = h2.get_webview_window("main") {
                    let _ = win.destroy();
                }
            });
        }
    });
}

fn resolve_arg(cwd: Option<&Path>, raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_relative() {
        if let Some(c) = cwd {
            return c.join(p);
        }
    }
    p
}

/// Shared between the initial launch and single-instance forwarded launches.
fn handle_launch(app: &AppHandle, args: &[String], cwd: Option<&Path>, initial: bool) {
    let mut hidden = false;
    let mut pdf_out: Option<PathBuf> = None;
    let mut file: Option<PathBuf> = None;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hidden" => hidden = true,
            "--pdf" => {
                if let Some(v) = it.next() {
                    pdf_out = Some(resolve_arg(cwd, v));
                }
            }
            s if s.starts_with('-') => {}
            s => {
                if file.is_none() {
                    file = Some(resolve_arg(cwd, s));
                }
            }
        }
    }

    let headless_pdf = pdf_out.is_some();
    match file {
        Some(f) if f.exists() => {
            let show = !hidden && !headless_pdf;
            let exit_after = headless_pdf && initial;
            if let Err(e) = open_doc(app, &f, show, pdf_out, exit_after) {
                eprintln!("marq: {e}");
                if headless_pdf && initial {
                    app.exit(1);
                } else if !hidden {
                    show_main_window(app);
                    let _ = app.emit("toast", e);
                }
            }
        }
        Some(f) => {
            eprintln!("marq: file not found: {}", f.display());
            if headless_pdf && initial {
                app.exit(1);
            } else if !hidden {
                show_main_window(app);
                let _ = app.emit("toast", format!("File not found: {}", f.display()));
            }
        }
        None => {
            if !hidden {
                show_main_window(app);
            }
        }
    }

    // Launched for the tray only (autostart): park the renderer immediately.
    if hidden {
        hide_main_window(app);
    }
}

fn first_run_init(app: &AppHandle) {
    if is_dev_exe() {
        return;
    }
    let Ok(dir) = app.path().app_config_dir() else { return };
    let marker = dir.join(".initialized");
    if marker.exists() {
        // Keep the autostart entry pointing at the current exe location.
        if app.autolaunch().is_enabled().unwrap_or(false) {
            let _ = app.autolaunch().enable();
        }
        return;
    }
    let _ = std::fs::create_dir_all(&dir);
    // The whole point of Marq is being resident for instant opens.
    let _ = app.autolaunch().enable();
    let _ = std::fs::write(&marker, b"1");
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let open = MenuItem::with_id(app, "open", "Open Marq", true, None::<&str>)?;
    let open_file = MenuItem::with_id(app, "open_file", "Open file…", true, None::<&str>)?;
    let autostart =
        CheckMenuItem::with_id(app, "autostart", "Start with Windows", true, autostart_on, None::<&str>)?;
    let set_default = MenuItem::with_id(app, "set_default", "Make default for .md files", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Marq", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &open_file])
        .separator()
        .items(&[&autostart, &set_default])
        .separator()
        .items(&[&quit])
        .build()?;

    let autostart_item = autostart.clone();
    TrayIconBuilder::with_id("marq-tray")
        .icon(tauri::include_image!("icons/32x32.png"))
        .tooltip("Marq — Markdown viewer")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(move |app, ev| match ev.id().as_ref() {
            "open" => show_main_window(app),
            "open_file" => {
                let handle = app.clone();
                app.dialog()
                    .file()
                    .add_filter("Markdown", &MD_EXTS)
                    .pick_file(move |fp| {
                        if let Some(fp) = fp {
                            if let Ok(p) = fp.into_path() {
                                if let Err(e) = open_doc(&handle, &p, true, None, false) {
                                    let _ = handle.emit("toast", e);
                                }
                            }
                        }
                    });
            }
            "autostart" => {
                let al = app.autolaunch();
                let was_on = al.is_enabled().unwrap_or(false);
                let _ = if was_on { al.disable() } else { al.enable() };
                let _ = autostart_item.set_checked(al.is_enabled().unwrap_or(!was_on));
            }
            "set_default" => {
                #[cfg(windows)]
                {
                    let _ = assoc::register_current_exe();
                    assoc::open_default_apps_settings(app);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn run_app() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // A second marq.exe (e.g. double-clicked .md file) forwards its args here.
            let args: Vec<String> = argv.into_iter().skip(1).collect();
            handle_launch(app, &args, Some(Path::new(&cwd)), false);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_current_doc,
            get_highlight_css,
            mark_rendered,
            export_pdf,
            open_path,
            open_external,
            open_file_dialog,
            set_default_app
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            build_tray(app)?;

            if let Some(win) = app.get_webview_window("main") {
                attach_close_handler(&handle, &win);
            }

            #[cfg(windows)]
            if !is_dev_exe() {
                let _ = assoc::register_current_exe();
            }
            first_run_init(&handle);

            let args: Vec<String> = std::env::args().skip(1).collect();
            let cwd = std::env::current_dir().ok();
            handle_launch(&handle, &args, cwd.as_deref(), true);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error building Marq")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // Stay resident in the tray unless an explicit app.exit() happened.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}

fn main() {
    // Headless pipeline check: marq --render-html <in.md> <out.html>
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--render-html" {
        match render::render_path(Path::new(&args[2])) {
            Ok(doc) => {
                std::fs::write(&args[3], &doc.html).expect("write output");
                println!("ok: {} ({} bytes)", args[3], doc.html.len());
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    run_app();
}
