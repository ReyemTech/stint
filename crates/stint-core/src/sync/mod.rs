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

/// One iteration of the sync loop: always drain the queue; on every 15th
/// tick, also refresh reference data. Auth failures from drain propagate;
/// reference-refresh errors are logged and swallowed so a transient
/// projects/tasks/tags hiccup doesn't break the queue worker.
///
/// Extracted from `run_loop` so the one-tick behaviour is unit-testable
/// without driving an infinite loop. Mirrors `stint-app::pull_worker::tick`.
pub async fn tick(store: &Store, client: &SolidtimeClient, count: u64) -> Result<()> {
    drain_once(store, client).await?;
    if count % 15 == 0 {
        if let Err(e) = refresh::refresh_reference_data(store, client).await {
            warn!(error = %e, "reference refresh failed");
        }
    }
    Ok(())
}

/// Runs forever, ticking once per minute. Returns only on a fatal error
/// (auth failure from drain_once).
pub async fn run_loop(store: Store, client: SolidtimeClient) -> Result<()> {
    let mut count = 0u64;
    loop {
        tick(&store, &client, count).await?;
        info!(tick = count, "sync loop tick");
        sleep(Duration::from_secs(60)).await;
        count += 1;
    }
}
