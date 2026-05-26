mod app_state;
mod calendar_worker;
mod commands;
mod http;
mod logging;
mod menu;
mod pull_worker;
mod sync_worker;
mod tray;
mod updater;
#[cfg_attr(not(feature = "updater"), allow(dead_code))]
mod updater_endpoint;
mod windows;

use anyhow::Result;
use app_state::AppState;
use tauri::Manager;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    // Hold the non-blocking log writer guard for the program lifetime; dropping
    // it would flush + close the file appender mid-run.
    let _log_guard = logging::init();

    let app_state = AppState::init().await?;
    let store_for_worker = app_state.store.clone();
    let http_port_slot = app_state.http_api_port.clone();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init());

    #[cfg(feature = "updater")]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());

    builder
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
            commands::config::settings_get,
            commands::config::settings_set,
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
            commands::integrations::get_api_integration_state,
            commands::integrations::set_api_enabled,
            commands::ui::show_main_window,
            updater::check_for_updates,
            updater::install_update,
            updater::restart_app,
        ])
        .setup(move |app| {
            tray::build(app.handle())?;

            // Initialize the StintIntents Swift framework if it's loaded into
            // the app bundle. The framework exports stint_intents_init as an
            // @_cdecl symbol; we look it up via dlsym so this path no-ops on
            // builds where the framework is absent (raw dev binaries from
            // scripts/dev-app.sh, missing build artifacts, etc).
            init_stint_intents();

            // Register stint:// URL scheme handler. Each incoming URL is parsed
            // by stint_core::url_scheme and dispatched to the verbs façade.
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let handle = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let url_str = url.to_string();
                        let h = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = handle_stint_url(&h, &url_str).await {
                                tracing::warn!("stint:// dispatch failed for {url_str}: {e}");
                            }
                        });
                    }
                });
            }

            // Loopback HTTP API (opt-in via `api.enabled` setting). Spawned on
            // the Tokio runtime so it lives for the GUI process lifetime. The
            // bound port is recorded in `http_port_slot` so the Settings
            // → Integrations panel can show "live this session" vs "pending
            // restart".
            {
                let store_for_http = store_for_worker.clone();
                let slot = http_port_slot.clone();
                tokio::spawn(async move {
                    match http::maybe_spawn(store_for_http, slot).await {
                        Ok(Some(port)) => tracing::info!(port, "http api listening"),
                        Ok(None) => {}
                        Err(e) => tracing::error!("http api failed to start: {e}"),
                    }
                });
            }

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

async fn handle_stint_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    url: &str,
) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use stint_core::url_scheme::{parse, Action};
    let action = parse(url)?;

    let state: tauri::State<'_, RwLock<AppState>> = app.state();
    let store = {
        let guard = state.read().await;
        guard.store.clone()
    };

    match action {
        Action::Start {
            description,
            project_id,
            task_id,
            billable,
        } => {
            stint_core::verbs::start(
                &store,
                stint_core::verbs::StartParams {
                    description,
                    project_id,
                    task_id,
                    billable,
                    start_at: None,
                    source: "url".into(),
                },
            )
            .await?;
        }
        Action::Stop => {
            stint_core::verbs::stop(&store).await?;
        }
        Action::OpenEntry { local_uuid } => {
            // Look up the entry's start_at so we can navigate to the day
            // it belongs to (Today only shows today; a stint:// link from
            // Spotlight may point at an older entry).
            let route = match stint_core::verbs::list_entries(
                &store,
                stint_core::verbs::EntryFilter {
                    limit: Some(1000),
                    ..Default::default()
                },
            )
            .await
            {
                Ok(entries) => entries
                    .into_iter()
                    .find(|e| e.local_uuid == local_uuid)
                    .map(|e| {
                        // Pass entry+date so Today (or future routes) can
                        // scroll to / highlight the row.
                        let date = e.start_at.split('T').next().unwrap_or("").to_string();
                        format!("/today?entry={local_uuid}&date={date}")
                    })
                    .unwrap_or_else(|| format!("/today?entry={local_uuid}")),
                Err(_) => format!("/today?entry={local_uuid}"),
            };
            focus_main_window_at_route(app, &route);
        }
        Action::Current => {
            focus_main_window_at_route(app, "/today");
        }
        Action::OpenProject { project_id } => {
            focus_main_window_at_route(app, &format!("/today?project={project_id}"));
        }
        Action::OpenTask { task_id } => {
            // Resolve task → parent project so the Today view can filter by both.
            let route = match stint_core::verbs::list_tasks(&store, None).await {
                Ok(tasks) => tasks
                    .into_iter()
                    .find(|t| t.solidtime_id == task_id)
                    .map(|t| format!("/today?project={}&task={}", t.project_id, task_id))
                    .unwrap_or_else(|| "/today".into()),
                Err(_) => "/today".into(),
            };
            focus_main_window_at_route(app, &route);
        }
    }
    Ok(())
}

/// Bring the main window forward and emit a navigate event so the SolidJS
/// router can land on the requested route. Payload is a bare string to
/// match the existing `navigate` listener in `ui/src/App.tsx` (set by the
/// tray menu and Settings shortcuts).
fn focus_main_window_at_route<R: tauri::Runtime>(app: &tauri::AppHandle<R>, route: &str) {
    use tauri::Emitter;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    let _ = app.emit("navigate", route);
}

/// Best-effort init of the StintIntents Swift framework via dlsym lookup
/// of `stint_intents_init`. No-op when the symbol isn't present (the
/// framework isn't bundled into the running binary).
/// Best-effort init of the StintIntents Swift framework via dlsym lookup
/// of `stint_intents_init`. The framework loads dynamically at app launch
/// (build.rs emits -needed_framework so LC_LOAD_DYLIB references it). At
/// the first call, the framework's @_cdecl symbol is resolvable via the
/// flat dyld namespace.
fn init_stint_intents() {
    use std::ffi::CString;
    type InitFn = unsafe extern "C" fn() -> i32;
    let name = CString::new("stint_intents_init").unwrap();
    let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
    if sym.is_null() {
        tracing::debug!(
            "stint_intents_init not present; Spotlight/App Intents integration disabled"
        );
        return;
    }
    let f: InitFn = unsafe { std::mem::transmute(sym) };
    let rc = unsafe { f() };
    if rc != 0 {
        tracing::warn!(rc, "stint_intents_init returned non-zero");
    } else {
        tracing::info!("StintIntents framework initialized");
    }
}
