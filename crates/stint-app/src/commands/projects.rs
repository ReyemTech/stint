use crate::app_state::AppState;
use crate::commands::{store, AppError};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::{ProjectRow, Reference};
use stint_core::sync::refresh::refresh_reference_data;
use tauri::State;
use tokio::sync::RwLock;

async fn build_client(store: &stint_core::store::Store) -> Result<SolidtimeClient, AppError> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = secrets
        .get("solidtime.token")?
        .ok_or(stint_core::Error::MissingConfig("solidtime.token"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.org"))?;
    Ok(SolidtimeClient::new(&url, &token).with_org(org))
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<ProjectRow>, AppError> {
    let store = store(&state).await;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?)
}

#[tauri::command]
pub async fn refresh_projects(
    state: State<'_, RwLock<AppState>>,
) -> Result<usize, AppError> {
    let store = store(&state).await;
    let client = build_client(&store).await?;
    refresh_reference_data(&store, &client).await?;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?.len())
}
