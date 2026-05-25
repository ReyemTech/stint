pub mod calendar;
pub mod config;
pub mod entries;
pub mod integrations;
pub mod projects;
pub mod pull;
pub mod sync;
pub mod timer;
pub mod ui;

use crate::app_state::AppState;
use std::sync::Arc;
use stint_core::store::Store;
use tauri::State;
use tokio::sync::RwLock;

pub(crate) async fn store(state: &State<'_, RwLock<AppState>>) -> Arc<Store> {
    state.read().await.store.clone()
}

#[derive(Debug, serde::Serialize)]
pub struct AppError {
    pub kind: String,
    pub message: String,
}

impl AppError {
    pub fn msg(s: impl Into<String>) -> Self {
        Self {
            kind: "msg".into(),
            message: s.into(),
        }
    }
}

impl From<stint_core::Error> for AppError {
    fn from(e: stint_core::Error) -> Self {
        let kind = match &e {
            stint_core::Error::Sqlite(_) => "sqlite",
            stint_core::Error::Migration(_) => "migration",
            stint_core::Error::Io(_) => "io",
            stint_core::Error::Http(_) => "http",
            stint_core::Error::Serde(_) => "serde",
            stint_core::Error::Keyring(_) => "keyring",
            stint_core::Error::Solidtime { .. } => "solidtime",
            stint_core::Error::SolidtimeAuth => "solidtime_auth",
            stint_core::Error::MissingConfig(_) => "missing_config",
            stint_core::Error::Invariant(_) => "invariant",
            stint_core::Error::NotFound(_) => "not_found",
            stint_core::Error::OAuthCancelled => "oauth_cancelled",
            stint_core::Error::OAuthServer(_) => "oauth_server",
            stint_core::Error::OAuthRefreshFailed => "oauth_refresh_failed",
            stint_core::Error::OAuthStateMismatch => "oauth_state_mismatch",
            stint_core::Error::OAuthLoopback(_) => "oauth_loopback",
        };
        Self {
            kind: kind.into(),
            message: e.to_string(),
        }
    }
}
