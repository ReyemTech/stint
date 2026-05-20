use crate::app_state::AppState;
use crate::commands::{store, AppError};
use serde::Serialize;
use std::time::Duration;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::{
    build_token_provider, login_interactive, oauth_blob_delete, oauth_blob_load, oauth_blob_save,
    AuthMode, OAuthBlob,
};
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
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or(stint_core::Error::MissingConfig("solidtime.url"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    let client = SolidtimeClient::new(&url, provider);
    let me = client.test_connection().await?;
    Ok(me.email.unwrap_or(me.id))
}

#[derive(Serialize)]
pub struct SolidtimeAuthStatus {
    mode: &'static str,
    signed_in: bool,
    scope: Option<String>,
}

#[tauri::command]
pub async fn oauth_solidtime_status(
    state: State<'_, RwLock<AppState>>,
) -> Result<SolidtimeAuthStatus, AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let mode = AuthMode::from_str_or_default(settings.get("solidtime.auth_mode").await?.as_deref());
    let secrets = Secrets::default();
    let (signed_in, scope) = match mode {
        AuthMode::ApiToken => (secrets.get("solidtime.token")?.is_some(), None),
        AuthMode::OAuth => {
            let blob = oauth_blob_load(&secrets)?;
            (blob.is_some(), blob.and_then(|b| b.tokens.scope))
        }
    };
    Ok(SolidtimeAuthStatus {
        mode: match mode {
            AuthMode::ApiToken => "api_token",
            AuthMode::OAuth => "oauth",
        },
        signed_in,
        scope,
    })
}

#[tauri::command]
pub async fn oauth_solidtime_start(state: State<'_, RwLock<AppState>>) -> Result<(), AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let base_url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| AppError::msg("solidtime.url is not set"))?;
    let client_id = settings
        .get("solidtime.oauth.client_id")
        .await?
        .ok_or_else(|| AppError::msg("solidtime.oauth.client_id is not set"))?;

    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        client_secret: None,
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        // Empty by default — see DEFAULT_SCOPES doc in solidtime/auth.rs.
        scopes: vec![],
        extra_authorize_params: vec![],
    });

    let tokens = login_interactive(&client, Duration::from_secs(300), "Solidtime", |url| {
        if let Err(e) = open_url(&url) {
            tracing::warn!("could not open browser: {e}; user must paste URL manually: {url}");
        }
    })
    .await?;

    oauth_blob_save(&Secrets::default(), &OAuthBlob { client_id, tokens })?;
    settings.set("solidtime.auth_mode", "oauth").await?;
    Ok(())
}

#[tauri::command]
pub async fn oauth_solidtime_logout(state: State<'_, RwLock<AppState>>) -> Result<(), AppError> {
    let store = store(&state).await;
    let settings = Settings::new((*store).clone());
    let secrets = Secrets::default();
    oauth_blob_delete(&secrets)?;
    if secrets.get("solidtime.token")?.is_some() {
        settings.set("solidtime.auth_mode", "api_token").await?;
    }
    Ok(())
}

/// Open a URL in the system browser on macOS using `/usr/bin/open`.
fn open_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("/usr/bin/open")
        .arg(url)
        .status()
        .map(|_| ())
}
