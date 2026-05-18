use crate::app_state::AppState;
use crate::commands::{store, AppError};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::drain_once;
use tauri::State;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn sync_now(state: State<'_, RwLock<AppState>>) -> Result<usize, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = Secrets::default()
        .get("solidtime.token")?
        .ok_or(stint_core::Error::MissingConfig("solidtime.token"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.org"))?;
    let client = SolidtimeClient::new(&url, &token).with_org(org);
    Ok(drain_once(&store, &client).await?)
}
