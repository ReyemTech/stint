//! Background sync worker — periodically drains the local queue against
//! Solidtime so users don't have to click "Sync now" manually. Also exposes
//! a fire-and-forget helper for immediate drains after a mutation.
//!
//! Emits `entries:changed` to the frontend after any drain that actually
//! changed local state, so the UI can refresh without polling.

use std::sync::Arc;
use std::time::Duration;
use stint_core::{
    config::{secrets::Secrets, Settings},
    solidtime::auth::build_token_provider,
    solidtime::SolidtimeClient,
    store::Store,
    sync::drain_once,
};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub const EVENT_ENTRIES_CHANGED: &str = "entries:changed";

const TICK: Duration = Duration::from_secs(30);

/// Spawns the periodic background worker on the Tokio runtime.
pub fn spawn(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        info!("background sync worker started (tick = {:?})", TICK);
        loop {
            match tick(&store).await {
                Ok(n) if n > 0 => {
                    let _ = app.emit(EVENT_ENTRIES_CHANGED, n);
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "sync tick failed"),
            }
            sleep(TICK).await;
        }
    });
}

/// Fire-and-forget one-shot drain. Use after a local mutation so the
/// user doesn't have to wait for the next periodic tick.
pub fn nudge(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        match tick(&store).await {
            Ok(n) if n > 0 => {
                let _ = app.emit(EVENT_ENTRIES_CHANGED, n);
            }
            Ok(_) => {}
            Err(e) => debug!(error = %e, "nudge drain failed"),
        }
    });
}

async fn tick(store: &Store) -> stint_core::Result<usize> {
    let Some(client) = build_client(store).await? else {
        debug!("sync worker: config incomplete, skipping tick");
        return Ok(0);
    };
    let drained = drain_once(store, &client).await?;
    if drained > 0 {
        info!(drained, "sync worker drained items");
    }
    Ok(drained)
}

async fn build_client(store: &Store) -> stint_core::Result<Option<SolidtimeClient>> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();

    let Some(url) = settings.get("solidtime.url").await? else {
        return Ok(None);
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Ok(None);
    };
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(Some(SolidtimeClient::new(&url, provider).with_org(org)))
}
