//! Background calendar refresher. Polls every 15 min while the GUI runs,
//! mirroring `sync_worker.rs`. Emits `calendar:changed` after any tick
//! that upserted at least one event.

use std::sync::Arc;
use std::time::Duration;
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::config::secrets::Secrets;
use stint_core::store::Store;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub const EVENT_CALENDAR_CHANGED: &str = "calendar:changed";
const TICK: Duration = Duration::from_secs(15 * 60);

pub fn spawn(app: AppHandle, store: Arc<Store>) {
    tokio::spawn(async move {
        info!("calendar worker started (tick = {:?})", TICK);
        loop {
            match tick(&store).await {
                Ok(n) if n > 0 => {
                    let _ = app.emit(EVENT_CALENDAR_CHANGED, n);
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "calendar tick failed"),
            }
            sleep(TICK).await;
        }
    });
}

async fn tick(store: &Store) -> stint_core::Result<usize> {
    let cs = CalendarStore::new((*store).clone());
    let secrets = Secrets::default();
    let accounts = cs.list_accounts().await?;
    if accounts.is_empty() {
        debug!("calendar worker: no accounts; skipping tick");
        return Ok(0);
    }

    let mut total = 0usize;
    for account in accounts {
        if !account.enabled {
            continue;
        }
        match stint_core::calendar::google::build_provider_from_blob(&secrets, &account.id) {
            Ok(provider) => {
                match refresh_account(
                    &cs,
                    &account.id,
                    provider.as_ref(),
                    Ranges::background_poll(),
                )
                .await
                {
                    Ok(n) => total += n,
                    Err(e) => {
                        warn!(account = %account.id, error = %e, "calendar refresh failed")
                    }
                }
            }
            Err(e) => warn!(account = %account.id, error = %e, "could not build provider"),
        }
    }
    if total > 0 {
        info!(events = total, "calendar worker refreshed events");
    }
    Ok(total)
}
