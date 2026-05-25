//! Loopback HTTP API. Bound to 127.0.0.1, opt-in via `api.enabled` setting,
//! tied to GUI process lifetime — no separate daemon.

pub mod error;
pub mod handlers;

use axum::routing::{delete, get, patch, post};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use stint_core::config::{Settings, DEFAULT_API_HOST, KEY_API_ENABLED, KEY_API_HOST, KEY_API_PORT};
use stint_core::store::Store;
use stint_core::Result;
use tokio::net::TcpListener;

/// Build the axum router. Exposed so integration tests can drive it via
/// `tower::ServiceExt::oneshot` without binding a real socket.
pub fn build_router(store: Arc<Store>) -> Router {
    Router::new()
        .route("/v1/start", post(handlers::start))
        .route("/v1/stop", post(handlers::stop))
        .route("/v1/current", get(handlers::current))
        .route("/v1/entries", get(handlers::list_entries))
        .route("/v1/entries/:id", patch(handlers::update_entry))
        .route("/v1/entries/:id", delete(handlers::delete_entry))
        .route("/v1/projects", get(handlers::list_projects))
        .route("/v1/tasks", get(handlers::list_tasks))
        .with_state(store)
}

/// Spawn the HTTP server if enabled. Returns the bound port (also persisted
/// to `api.port`) or `None` when disabled.
pub async fn maybe_spawn(store: Arc<Store>) -> Result<Option<u16>> {
    let settings = Settings::new((*store).clone());
    let enabled = settings.get(KEY_API_ENABLED).await?.as_deref() == Some("true");
    if !enabled {
        return Ok(None);
    }

    let host = settings
        .get(KEY_API_HOST)
        .await?
        .unwrap_or_else(|| DEFAULT_API_HOST.to_string());
    let pinned: u16 = settings
        .get(KEY_API_PORT)
        .await?
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let addr: SocketAddr = format!("{host}:{pinned}")
        .parse()
        .map_err(|e| stint_core::Error::Invariant(format!("invalid api addr: {e}")))?;

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| stint_core::Error::Invariant(format!("bind {addr}: {e}")))?;
    let bound = listener.local_addr().unwrap().port();
    settings.set(KEY_API_PORT, &bound.to_string()).await?;

    let app = build_router(store);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("http api server exited: {e}");
        }
    });

    Ok(Some(bound))
}
