//! Tauri commands backing the Settings → Integrations panel.
//!
//! Exposes read/write access to the `api.enabled` flag plus the bound port
//! recorded by `http::maybe_spawn` at startup. Toggling `enabled` from the
//! UI only updates the setting — the HTTP server is bound once at process
//! launch, so changes take effect on the next app restart.

use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::config::{Settings, DEFAULT_API_HOST, KEY_API_ENABLED, KEY_API_HOST, KEY_API_PORT};
use tauri::State;
use tokio::sync::RwLock;

#[derive(Serialize)]
pub struct ApiIntegrationState {
    /// Current value of the `api.enabled` setting.
    pub enabled: bool,
    /// Host the server binds to. Defaults to 127.0.0.1.
    pub host: String,
    /// Last persisted port (from the `api.port` setting). Persisted across
    /// restarts so the GUI can render the address even before this session's
    /// server has finished binding.
    pub port: Option<u16>,
    /// Convenience URL — `http://{host}:{port}` when both are known.
    pub base_url: Option<String>,
    /// True when the loopback server actually bound a socket during this
    /// app session. False when `api.enabled` is on but the GUI hasn't been
    /// restarted yet, or when the bind failed.
    pub bound_this_session: bool,
}

async fn read_state(app_state: &RwLock<AppState>) -> Result<ApiIntegrationState, AppError> {
    // Clone Arc handles out under the read lock so we don't hold it across
    // SQLite awaits.
    let (store_arc, port_slot) = {
        let guard = app_state.read().await;
        (guard.store.clone(), guard.http_api_port.clone())
    };
    let settings = Settings::new((*store_arc).clone());

    let enabled = settings.get(KEY_API_ENABLED).await?.as_deref() == Some("true");
    let host = settings
        .get(KEY_API_HOST)
        .await?
        .unwrap_or_else(|| DEFAULT_API_HOST.to_string());
    let port: Option<u16> = settings
        .get(KEY_API_PORT)
        .await?
        .and_then(|s| s.parse().ok());

    let base_url = port.map(|p| format!("http://{host}:{p}"));
    let bound_this_session = port_slot.read().await.is_some();

    Ok(ApiIntegrationState {
        enabled,
        host,
        port,
        base_url,
        bound_this_session,
    })
}

/// Read the current HTTP API integration state for the Settings panel.
#[tauri::command]
pub async fn get_api_integration_state(
    state: State<'_, RwLock<AppState>>,
) -> Result<ApiIntegrationState, AppError> {
    read_state(&state).await
}

/// Persist a new value for `api.enabled`. The change only takes effect after
/// the next app restart — the loopback server binds once at startup.
#[tauri::command]
pub async fn set_api_enabled(
    state: State<'_, RwLock<AppState>>,
    enabled: bool,
) -> Result<ApiIntegrationState, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    settings
        .set(KEY_API_ENABLED, if enabled { "true" } else { "false" })
        .await?;
    read_state(&state).await
}
