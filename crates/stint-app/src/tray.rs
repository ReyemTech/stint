use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::windows;

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
        .unwrap_or_else(|_| app.default_window_icon().cloned().unwrap());

    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open Stint", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
        ],
    )?;

    let tray = TrayIconBuilder::with_id("stint-tray")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                let _ = windows::show_main(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button, .. } = event {
                if matches!(button, MouseButton::Left) {
                    let app = tray.app_handle();
                    if let Some(win) = app.get_webview_window("popover") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = windows::hide_popover(app);
                        } else {
                            let _ = windows::show_popover(app);
                            let _ = tauri_plugin_positioner::WindowExt::move_window(
                                &win,
                                tauri_plugin_positioner::Position::TrayCenter,
                            );
                        }
                    }
                }
            }
        })
        .build(app)?;
    Ok(tray)
}
