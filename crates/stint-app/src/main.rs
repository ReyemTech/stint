mod app_state;
mod commands;
mod tray;
mod windows;

use anyhow::Result;
use app_state::AppState;
use tauri::Manager;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STINT_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let app_state = AppState::init().await?;

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(RwLock::new(app_state))
        .invoke_handler(tauri::generate_handler![
            commands::timer::get_running_timer,
            commands::timer::start_timer,
            commands::timer::stop_timer,
            commands::timer::delete_entry,
            commands::timer::update_description,
            commands::entries::list_today,
            commands::entries::list_between,
            commands::projects::list_projects,
            commands::projects::refresh_projects,
            commands::config::config_show,
            commands::config::config_set,
            commands::config::config_test,
            commands::sync::sync_now,
        ])
        .setup(|app| {
            tray::build(app.handle())?;

            // Hide dock icon on startup (menu-bar app behavior).
            windows::hide_dock(app.handle());

            // Intercept main-window close: hide instead of quit, and return to
            // accessory mode (no dock icon) until the user reopens it.
            if let Some(main) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                let main_clone = main.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_clone.hide();
                        windows::hide_dock(&app_handle);
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
