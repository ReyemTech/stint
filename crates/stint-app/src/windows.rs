use tauri::{AppHandle, Manager};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

pub fn show_popover(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("popover") {
        win.show()?;
        win.set_focus()?;
    }
    Ok(())
}

pub fn hide_popover(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("popover") {
        win.hide()?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn hide_dock(app: &AppHandle) {
    let _ = app.set_activation_policy(ActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
pub fn hide_dock(_app: &AppHandle) {}
