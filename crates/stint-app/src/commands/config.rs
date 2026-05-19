use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use tauri::State;
use tokio::sync::RwLock;

const SECRET_KEYS: &[&str] = &["solidtime.token"];

#[derive(Serialize)]
pub struct ConfigView {
    pub key: String,
    pub value: Option<String>,
    pub is_secret: bool,
    pub present: bool,
}

#[tauri::command]
pub async fn config_show(state: State<'_, RwLock<AppState>>) -> Result<Vec<ConfigView>, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let secrets = Secrets::default();

    let mut out: Vec<ConfigView> = settings
        .list_prefixed("")
        .await?
        .into_iter()
        .map(|(k, v)| ConfigView {
            key: k,
            value: Some(v),
            is_secret: false,
            present: true,
        })
        .collect();

    for k in SECRET_KEYS {
        let present = secrets.get(k)?.is_some();
        out.push(ConfigView {
            key: (*k).to_string(),
            value: None,
            is_secret: true,
            present,
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn config_set(
    state: State<'_, RwLock<AppState>>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    let store = store(&state).await;
    if SECRET_KEYS.contains(&key.as_str()) {
        Secrets::default().set(&key, &value)?;
    } else {
        Settings::new((*store).clone()).set(&key, &value).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn solidtime_url(state: State<'_, RwLock<AppState>>) -> Result<Option<String>, AppError> {
    let store = store(&state).await;
    Ok(Settings::new((*store).clone())
        .get("solidtime.url")
        .await?
        .map(|s| s.trim_end_matches('/').to_string()))
}

#[tauri::command]
pub async fn config_test(state: State<'_, RwLock<AppState>>) -> Result<String, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let token = Secrets::default()
        .get("solidtime.token")?
        .ok_or(stint_core::Error::MissingConfig("solidtime.token"))?;
    let client = SolidtimeClient::new(&url, &token);
    let me = client.test_connection().await?;
    Ok(me.email.unwrap_or(me.id))
}
