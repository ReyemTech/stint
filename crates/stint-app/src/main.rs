mod app_state;
mod calendar_worker;
mod commands;
mod menu;
mod pull_worker;
mod sync_worker;
mod tray;
#[allow(dead_code)]
mod updater_endpoint;
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
        .menu(menu::build)
        .on_menu_event(|app, event| menu::handle(app, event.id.as_ref()))
        .manage(RwLock::new(app_state))
        .invoke_handler(tauri::generate_handler![
            commands::timer::get_running_timer,
            commands::timer::start_timer,
            commands::timer::stop_timer,
            commands::timer::restart_entry,
            commands::timer::delete_entry,
            commands::timer::update_description,
            commands::timer::set_entry_project,
            commands::timer::set_entry_billable,
            commands::timer::update_entry_times,
            commands::entries::list_today,
            commands::entries::list_between,
            commands::projects::list_projects,
            commands::projects::refresh_projects,
            commands::projects::list_organizations,
            commands::pull::pull_now,
            commands::pull::conflict_resolve,
            commands::config::config_show,
            commands::config::config_set,
            commands::config::config_test,
            commands::config::solidtime_url,
            commands::config::oauth_solidtime_status,
            commands::config::oauth_solidtime_start,
            commands::config::oauth_solidtime_logout,
            commands::calendar::calendar_list_accounts,
            commands::calendar::calendar_oauth_status,
            commands::calendar::calendar_add_google,
            commands::calendar::calendar_remove_account,
            commands::calendar::calendar_list_calendars,
            commands::calendar::calendar_set_calendar_included,
            commands::calendar::calendar_set_default_project,
            commands::calendar::calendar_refresh_account,
            commands::calendar::calendar_list_events_in_range,
            commands::calendar::calendar_log_event,
            commands::calendar::calendar_ignore_event,
            commands::calendar::calendar_revert_event,
            commands::sync::sync_now,
            commands::sync::list_sync_errors,
            commands::sync::get_sync_error_overlaps,
            commands::ui::show_main_window,
        ])
        .setup(move |app| {
            tray::build(app.handle())?;

            // Periodic background sync (drains queue every 30s while running).
            sync_worker::spawn(app.handle().clone(), store_for_worker.clone());

            // Periodic Solidtime → stint pull (5-min tick).
            pull_worker::spawn(app.handle().clone(), store_for_worker.clone());

            // One-shot pull on startup: surfaces a remote-side running timer
            // or recent edits within ~1s of launch, without waiting for the
            // 5-min background poll worker.
            {
                let app_handle = app.handle().clone();
                let store_for_pull = store_for_worker.clone();
                tokio::spawn(async move {
                    use stint_core::config::{secrets::Secrets, Settings};
                    use stint_core::solidtime::{auth::build_token_provider, SolidtimeClient};
                    use stint_core::sync::pull::{pull, Trigger};
                    use tauri::Emitter;
                    let settings = Settings::new((*store_for_pull).clone());
                    let Ok(Some(url)) = settings.get("solidtime.url").await else {
                        return;
                    };
                    let Ok(Some(org)) = settings.get("solidtime.org").await else {
                        return;
                    };
                    let secrets = Secrets::default();
                    let Ok((provider, _oauth_client)) =
                        build_token_provider(&settings, &secrets, &url).await
                    else {
                        return;
                    };
                    let client = SolidtimeClient::new(&url, provider).with_org(org);
                    match pull(&store_for_pull, &client, Trigger::OnStartup).await {
                        Ok(report) => {
                            if report.adopted.is_some()
                                || report.inserted + report.updated + report.deleted > 0
                            {
                                let _ = app_handle.emit(sync_worker::EVENT_ENTRIES_CHANGED, 0u32);
                            }
                            if let Some(conflict) = report.conflict {
                                let _ = app_handle.emit(
                                    sync_worker::EVENT_PULL_CONFLICT,
                                    commands::pull::ConflictDto::from(conflict),
                                );
                            }
                        }
                        Err(e) => tracing::warn!(error = %e, "startup pull failed"),
                    }
                });
            }

            // Periodic calendar refresh (polls every 15 min while running).
            calendar_worker::spawn(app.handle().clone(), store_for_worker.clone());

            // Hide dock icon on startup (menu-bar app behavior).
            windows::hide_dock(app.handle());

            // Intercept main-window close: hide instead of quit, and return to
            // accessory mode (no dock icon) until the user reopens it.
            if let Some(main) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                let main_clone = main.clone();
                let app_handle_focus = app.handle().clone();
                let store_for_focus = store_for_worker.clone();
                let last_focus_pull = std::sync::Arc::new(std::sync::Mutex::new(
                    std::time::Instant::now() - std::time::Duration::from_secs(60),
                ));
                main.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = main_clone.hide();
                        windows::hide_dock(&app_handle);
                    }
                    tauri::WindowEvent::Focused(true) => {
                        let mut guard = last_focus_pull.lock().unwrap();
                        if guard.elapsed() < std::time::Duration::from_secs(30) {
                            return;
                        }
                        *guard = std::time::Instant::now();
                        pull_worker::nudge(
                            app_handle_focus.clone(),
                            store_for_focus.clone(),
                            stint_core::sync::pull::Trigger::OnFocus,
                        );
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
