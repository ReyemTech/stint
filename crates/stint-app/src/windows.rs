use tauri::{AppHandle, Manager, WebviewWindow};

pub fn show_main(app: &AppHandle) -> tauri::Result<()> {
    if let Some(win) = app.get_webview_window("main") {
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

pub fn focus_or_show_main(app: &AppHandle) -> tauri::Result<()> {
    show_main(app)
}

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}
