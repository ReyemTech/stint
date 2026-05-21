use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tokio::sync::RwLock;

use crate::app_state::AppState;
use crate::sync_worker;
use crate::windows;

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))
        .unwrap_or_else(|_| app.default_window_icon().cloned().unwrap());

    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", "Open Stint", true, None::<&str>)?,
            &MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "about", "About Stint", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
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
            "about" => {
                let _ = windows::show_main(app);
                let _ = app.emit("navigate", "/about");
            }
            "sync" => {
                let app_handle = app.clone();
                tokio::spawn(async move {
                    let state = app_handle.state::<RwLock<AppState>>();
                    let store = state.read().await.store.clone();
                    sync_worker::nudge(app_handle.clone(), store);
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Feed every tray event to the positioner so it records the icon's
            // screen rect — required before move_window(TrayCenter) works.
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            // TrayIconEvent::Click fires for BOTH mouse-down and mouse-up.
            // Only act on the release (Up) so we don't toggle twice per click.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("popover") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = windows::hide_popover(app);
                    } else {
                        // tauri-plugin-positioner 2.3.1 does
                        // `window.current_monitor()?.unwrap()` inside
                        // `move_window`, which panics when the popover
                        // hasn't been associated with a monitor yet (the
                        // window is hidden on first launch). Park it on the
                        // primary monitor first so the unwrap succeeds.
                        if matches!(win.current_monitor(), Ok(None)) {
                            let _ = win.set_position(tauri::PhysicalPosition::new(0i32, 0i32));
                        }
                        let _ = tauri_plugin_positioner::WindowExt::move_window(
                            &win,
                            tauri_plugin_positioner::Position::TrayCenter,
                        );
                        let _ = windows::show_popover(app);
                    }
                }
            }
        })
        .build(app)?;
    Ok(tray)
}
