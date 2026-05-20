pub mod pull;
pub mod push;
pub mod refresh;

use crate::{
    solidtime::SolidtimeClient,
    store::{queue::Queue, Store},
    Error, Result,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Drains the sync queue once. Returns the number of items processed.
pub async fn drain_once(store: &Store, client: &SolidtimeClient) -> Result<usize> {
    let queue = Queue::new(store.clone());
    let due = queue.take_due(50).await?;
    let count = due.len();
    for row in due {
        if let Err(e) = push::push_one(store, client, &row).await {
            warn!(error = %e, op = %row.op, "queue item failed");
            if matches!(e, Error::SolidtimeAuth) {
                return Err(e);
            }
        }
    }
    Ok(count)
}

/// Runs forever, draining the queue and pulling reference data.
/// Returns only on a fatal error (auth failure).
pub async fn run_loop(store: Store, client: SolidtimeClient) -> Result<()> {
    let mut tick = 0u64;
    loop {
        drain_once(&store, &client).await?;
        if tick % 15 == 0 {
            if let Err(e) = refresh::refresh_reference_data(&store, &client).await {
                warn!(error = %e, "reference refresh failed");
            }
        }
        info!(tick, "sync loop tick");
        sleep(Duration::from_secs(60)).await;
        tick += 1;
    }
}
