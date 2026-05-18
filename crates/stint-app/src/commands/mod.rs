pub mod config;
pub mod entries;
pub mod projects;
pub mod sync;
pub mod timer;

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
        };
        Self {
            kind: kind.into(),
            message: e.to_string(),
        }
    }
}
