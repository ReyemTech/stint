use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::{ProjectRow, Reference};
use stint_core::sync::refresh::refresh_reference_data;
use stint_core::verbs::{self, TaskView};
use tauri::State;
use tokio::sync::RwLock;

async fn build_client(store: &stint_core::store::Store) -> Result<SolidtimeClient, AppError> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.org"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(SolidtimeClient::new(&url, provider).with_org(org))
}

/// Like build_client but does NOT require an org. Used for endpoints that
/// don't depend on a specific org (e.g. listing memberships).
async fn build_unorg_client(store: &stint_core::store::Store) -> Result<SolidtimeClient, AppError> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(SolidtimeClient::new(&url, provider))
}

#[derive(Serialize)]
pub struct OrgChoice {
    pub id: String,
    pub member_id: String,
    pub name: String,
}

#[tauri::command]
pub async fn list_organizations(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<OrgChoice>, AppError> {
    let store = store(&state).await;
    let client = build_unorg_client(&store).await?;
    let memberships = client.list_memberships().await?;
    Ok(memberships
        .into_iter()
        .map(|m| OrgChoice {
            id: m.organization.id,
            member_id: m.id,
            name: m.organization.name,
        })
        .collect())
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, RwLock<AppState>>,
) -> Result<Vec<ProjectRow>, AppError> {
    let store = store(&state).await;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?)
}

/// Delegates to `stint_core::verbs::list_tasks`. Passing `project_id` scopes
/// the result to one project; omitting it returns every locally-cached task
/// (used sparingly — the picker always passes a project_id).
#[tauri::command]
pub async fn list_tasks(
    state: State<'_, RwLock<AppState>>,
    project_id: Option<String>,
) -> Result<Vec<TaskView>, AppError> {
    let store = store(&state).await;
    Ok(verbs::list_tasks(&store, project_id).await?)
}

#[tauri::command]
pub async fn refresh_projects(state: State<'_, RwLock<AppState>>) -> Result<usize, AppError> {
    let store = store(&state).await;
    let client = build_client(&store).await?;
    refresh_reference_data(&store, &client).await?;
    let r = Reference::new((*store).clone());
    Ok(r.list_projects().await?.len())
}
