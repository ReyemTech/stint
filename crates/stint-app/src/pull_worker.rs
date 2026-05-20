//! Periodic Solidtime → stint pull. Runs every 5 minutes while the app is
//! open; also exposed as a nudge for explicit refreshes (window focus).

use crate::sync_worker::{EVENT_ENTRIES_CHANGED, EVENT_PULL_CONFLICT};
use std::sync::Arc;
use std::time::Duration;
use stint_core::{
    config::{secrets::Secrets, Settings},
    solidtime::{auth::build_token_provider, SolidtimeClient},
    store::Store,
    sync::pull::{pull, Trigger},
};
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tracing::{debug, info, warn};

const TICK: Duration = Duration::from_secs(300);

/// Spawn the 5-minute background pull worker.
pub fn spawn(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        info!("background pull worker started (tick = {:?})", TICK);
        loop {
            sleep(TICK).await;
            if let Err(e) = tick(&app, &store, Trigger::BackgroundPoll).await {
                warn!(error = %e, "pull tick failed");
            }
        }
    });
}

/// Fire a one-shot pull (window focus, manual refresh, etc.).
pub fn nudge(app: AppHandle, store: Arc<Store>, trigger: Trigger) {
    tokio::spawn(async move {
        if let Err(e) = tick(&app, &store, trigger).await {
            debug!(error = %e, "pull nudge failed");
        }
    });
}

async fn tick(
    app: &AppHandle,
    store: &Store,
    trigger: Trigger,
) -> stint_core::Result<()> {
    let Some(client) = build_client(store).await? else {
        debug!("pull worker: config incomplete, skipping tick");
        return Ok(());
    };
    let report = pull(store, &client, trigger).await?;
    if report.adopted.is_some() || report.inserted + report.updated + report.deleted > 0 {
        let _ = app.emit(EVENT_ENTRIES_CHANGED, 0u32);
    }
    if let Some(conflict) = report.conflict {
        use crate::commands::pull::ConflictDto;
        let _ = app.emit(EVENT_PULL_CONFLICT, ConflictDto::from(conflict));
    }
    Ok(())
}

async fn build_client(store: &Store) -> stint_core::Result<Option<SolidtimeClient>> {
    let settings = Settings::new(store.clone());
    let Some(url) = settings.get("solidtime.url").await? else {
        return Ok(None);
    };
    let Some(org) = settings.get("solidtime.org").await? else {
        return Ok(None);
    };
    let secrets = Secrets::default();
    let (provider, _) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(Some(SolidtimeClient::new(&url, provider).with_org(org)))
}
