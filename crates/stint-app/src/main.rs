mod app_state;
mod commands;
mod sync_worker;
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
    let store_for_worker = app_state.store.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .manage(RwLock::new(app_state))
        .invoke_handler(tauri::generate_handler![
            commands::timer::get_running_timer,
            commands::timer::start_timer,
            commands::timer::stop_timer,
            commands::timer::delete_entry,
            commands::timer::update_description,
            commands::timer::set_entry_project,
            commands::timer::set_entry_billable,
            commands::entries::list_today,
            commands::entries::list_between,
            commands::projects::list_projects,
            commands::projects::refresh_projects,
            commands::projects::list_organizations,
            commands::config::config_show,
            commands::config::config_set,
            commands::config::config_test,
            commands::config::solidtime_url,
            commands::sync::sync_now,
            commands::ui::show_main_window,
        ])
        .setup(move |app| {
            tray::build(app.handle())?;

            // Periodic background sync (drains queue every 30s while running).
            sync_worker::spawn(app.handle().clone(), store_for_worker.clone());

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
