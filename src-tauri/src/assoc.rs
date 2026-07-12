//! Per-user (HKCU, no admin) file-type registration so Marq shows up in
//! "Open with" and Settings → Default apps. Windows 10/11 deliberately keep
//! the final "make it the default" click with the user.

use std::io;
use std::path::Path;

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const PROGID: &str = "Marq.Markdown";
const EXTS: [&str; 4] = [".md", ".markdown", ".mdown", ".mkd"];

pub fn register_current_exe() -> io::Result<()> {
    register(&std::env::current_exe()?)
}

pub fn register(exe: &Path) -> io::Result<()> {
    let exe_s = exe.to_string_lossy();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // ProgID: what "opening a Markdown document with Marq" means.
    let (progid, _) = hkcu.create_subkey(format!("Software\\Classes\\{PROGID}"))?;
    progid.set_value("", &"Markdown document")?;
    let (icon, _) = progid.create_subkey("DefaultIcon")?;
    icon.set_value("", &format!("\"{exe_s}\",0"))?;
    let (open, _) = progid.create_subkey("shell\\open")?;
    open.set_value("", &"Open with Marq")?;
    let (cmd, _) = progid.create_subkey("shell\\open\\command")?;
    cmd.set_value("", &format!("\"{exe_s}\" \"%1\""))?;

    // Offer Marq for each extension's "Open with" list.
    for ext in EXTS {
        let (k, _) = hkcu.create_subkey(format!("Software\\Classes\\{ext}\\OpenWithProgids"))?;
        k.set_value(PROGID, &"")?;
    }

    // Capabilities registration: makes Marq appear in Settings → Default apps.
    let (caps, _) = hkcu.create_subkey("Software\\Marq\\Capabilities")?;
    caps.set_value("ApplicationName", &"Marq")?;
    caps.set_value("ApplicationDescription", &"Instant, beautiful Markdown viewer")?;
    let (fa, _) = caps.create_subkey("FileAssociations")?;
    for ext in EXTS {
        fa.set_value(ext, &PROGID)?;
    }
    let (reg_apps, _) = hkcu.create_subkey("Software\\RegisteredApplications")?;
    reg_apps.set_value("Marq", &"Software\\Marq\\Capabilities")?;

    notify_shell();
    Ok(())
}

/// Tell Explorer the association set changed so icons/menus refresh.
fn notify_shell() {
    use windows::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
}

pub fn open_default_apps_settings(app: &tauri::AppHandle) {
    use tauri_plugin_opener::OpenerExt as _;
    // Windows 11 deep-links straight to Marq's page in Default apps.
    if app
        .opener()
        .open_url("ms-settings:defaultapps?registeredAppUser=Marq", None::<&str>)
        .is_err()
    {
        let _ = app.opener().open_url("ms-settings:defaultapps", None::<&str>);
    }
}
