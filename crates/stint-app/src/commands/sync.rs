use crate::app_state::AppState;
use crate::commands::{store, AppError};
use crate::sync_worker::EVENT_ENTRIES_CHANGED;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::drain_once;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::RwLock;

#[tauri::command]
pub async fn sync_now<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, RwLock<AppState>>,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let secrets = Secrets::default();
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.org"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider).with_org(org);
    let n = drain_once(&store, &client).await?;
    if n > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, n);
    }
    Ok(n)
}
