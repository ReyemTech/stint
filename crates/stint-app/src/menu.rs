//! macOS application menu bar.
//!
//! Standard macOS menus: Stint (with About + Settings), Edit, Window.
//! Menu items emit `navigate` events that the frontend listens to.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Emitter, Wry,
};

use crate::windows;

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let app_menu = Submenu::with_items(
        app,
        "Stint",
        true,
        &[
            &MenuItem::with_id(app, "menu-about", "About Stint", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "menu-check-updates",
                "Check for Updates…",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "menu-settings", "Settings…", true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])
}

pub fn handle(app: &AppHandle, id: &str) {
    match id {
        "menu-about" => {
            let _ = windows::show_main(app);
            let _ = app.emit("navigate", "/about");
        }
        "menu-settings" => {
            let _ = windows::show_main(app);
            let _ = app.emit("navigate", "/settings");
        }
        "menu-check-updates" => {
            let _ = windows::show_main(app);
            let _ = app.emit("navigate", "/settings");
            // Second event the UpdatesPanel listens for. App.tsx's navigate
            // listener fires synchronously, so by the time this lands the
            // panel is on its way to mounting — UpdatesPanel registers a
            // global signal handler that survives mount/unmount cycles.
            let _ = app.emit("check-for-updates", ());
        }
        _ => {}
    }
}
