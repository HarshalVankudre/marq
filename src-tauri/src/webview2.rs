//! Direct WebView2 COM access for the two things Tauri doesn't expose:
//! silent print-to-PDF and renderer suspension while parked in the tray.

use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Environment6, ICoreWebView2PrintSettings, ICoreWebView2_19, ICoreWebView2_2,
    ICoreWebView2_3, ICoreWebView2_7, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT,
};
use webview2_com::{PrintToPdfCompletedHandler, TrySuspendCompletedHandler};
use windows_core::Interface;

type PdfSender = Arc<Mutex<Option<tokio::sync::oneshot::Sender<Result<(), String>>>>>;

fn send(tx: &PdfSender, r: Result<(), String>) {
    if let Some(t) = tx.lock().unwrap().take() {
        let _ = t.send(r);
    }
}

/// Prints the window's current document to `out` as an A4 PDF with backgrounds
/// and no browser headers/footers. Resolves when WebView2 reports completion.
pub async fn print_to_pdf(window: WebviewWindow, out: PathBuf) -> Result<(), String> {
    if let Some(dir) = out.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let wide: Vec<u16> = out
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
    let tx: PdfSender = Arc::new(Mutex::new(Some(tx)));
    let tx_setup = tx.clone();

    window
        .with_webview(move |pw| {
            let tx_handler = tx_setup.clone();
            let result = (|| -> windows_core::Result<()> {
                unsafe {
                    let controller = pw.controller();
                    let core = controller.CoreWebView2()?;
                    let wv7: ICoreWebView2_7 = core.cast()?;
                    let core2: ICoreWebView2_2 = core.cast()?;
                    let env: ICoreWebView2Environment6 = core2.Environment()?.cast()?;
                    let settings: ICoreWebView2PrintSettings = env.CreatePrintSettings()?;
                    settings.SetOrientation(COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT)?;
                    settings.SetShouldPrintBackgrounds(true)?;
                    settings.SetShouldPrintHeaderAndFooter(false)?;
                    settings.SetPageWidth(8.27)?; // A4, inches
                    settings.SetPageHeight(11.69)?;
                    settings.SetMarginTop(0.6)?;
                    settings.SetMarginBottom(0.6)?;
                    settings.SetMarginLeft(0.6)?;
                    settings.SetMarginRight(0.6)?;

                    let handler = PrintToPdfCompletedHandler::create(Box::new(move |ec, ok: bool| {
                        let r = match ec {
                            Ok(()) if ok => Ok(()),
                            Ok(()) => Err("WebView2 reported print failure".to_string()),
                            Err(e) => Err(format!("print error: {e}")),
                        };
                        send(&tx_handler, r);
                        Ok(())
                    }));
                    wv7.PrintToPdf(
                        windows_core::PCWSTR(wide.as_ptr()),
                        &settings,
                        &handler,
                    )?;
                }
                Ok(())
            })();
            if let Err(e) = result {
                send(&tx_setup, Err(format!("print setup failed: {e}")));
            }
        })
        .map_err(|e| e.to_string())?;

    match tokio::time::timeout(Duration::from_secs(90), rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => Err("print channel dropped".into()),
        Err(_) => Err("PDF export timed out".into()),
    }
}

/// Hidden → low memory target + suspend the renderer (frees most of the
/// WebView's working set). Visible → normal target; resume is automatic.
pub fn set_webview_visible(window: &WebviewWindow, visible: bool) {
    let _ = window.with_webview(move |pw| unsafe {
        let controller = pw.controller();
        let _ = controller.SetIsVisible(visible);
        if let Ok(core) = controller.CoreWebView2() {
            if let Ok(wv19) = core.cast::<ICoreWebView2_19>() {
                let _ = wv19.SetMemoryUsageTargetLevel(if visible {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                } else {
                    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                });
            }
            if !visible {
                if let Ok(wv3) = core.cast::<ICoreWebView2_3>() {
                    let handler = TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(())));
                    let _ = wv3.TrySuspend(&handler);
                }
            }
        }
    });
}
