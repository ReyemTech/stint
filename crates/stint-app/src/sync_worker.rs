//! Background sync worker — periodically drains the local queue against
//! Solidtime so users don't have to click "Sync now" manually. Also exposes
//! a fire-and-forget helper for immediate drains after a mutation.

use std::sync::Arc;
use std::time::Duration;
use stint_core::{
    config::{secrets::Secrets, Settings},
    solidtime::SolidtimeClient,
    store::Store,
    sync::drain_once,
};
use tokio::time::sleep;
use tracing::{debug, info, warn};

const TICK: Duration = Duration::from_secs(30);

/// Spawns the periodic background worker on the Tokio runtime. Returns immediately.
pub fn spawn(store: Arc<Store>) {
    tokio::spawn(async move {
        info!("background sync worker started (tick = {:?})", TICK);
        loop {
            if let Err(e) = tick(&store).await {
                warn!(error = %e, "sync tick failed");
            }
            sleep(TICK).await;
        }
    });
}

/// Fire-and-forget one-shot drain. Used after a local mutation so the user
/// doesn't have to wait for the next periodic tick.
pub fn nudge(store: Arc<Store>) {
    tokio::spawn(async move {
        if let Err(e) = tick(&store).await {
            debug!(error = %e, "nudge drain failed");
        }
    });
}

async fn tick(store: &Store) -> stint_core::Result<()> {
    let Some(client) = build_client(store).await? else {
        debug!("sync worker: config incomplete, skipping tick");
        return Ok(());
    };
    let drained = drain_once(store, &client).await?;
    if drained > 0 {
        info!(drained, "sync worker drained items");
    }
    Ok(())
}

async fn build_client(store: &Store) -> stint_core::Result<Option<SolidtimeClient>> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();

    let Some(url) = settings.get("solidtime.url").await? else {
        return Ok(None);
    };
    let Some(token) = secrets.get("solidtime.token")? else {
        return Ok(None);
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Ok(None);
    };
    Ok(Some(SolidtimeClient::new(&url, &token).with_org(org)))
}
