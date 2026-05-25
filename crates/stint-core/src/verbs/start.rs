use crate::store::entries::Entries;
use crate::store::Store;
use crate::timer::{StartArgs, TimerService};
use crate::verbs::types::{EntryView, StartParams};
use crate::Result;

/// Start a new running entry. Stopping any in-progress timer first is the
/// caller's responsibility — this verb is strict and returns an error if
/// a timer is already running. (Restart-style behavior lives in a separate
/// helper at the transport layer.)
pub async fn start(store: &Store, params: StartParams) -> Result<EntryView> {
    let timer = TimerService::new(store.clone());
    let id = timer
        .start(StartArgs {
            description: params.description,
            project_id: params.project_id,
            task_id: params.task_id,
            billable: params.billable,
            source: params.source,
            start_at: params.start_at,
        })
        .await?;

    let entries = Entries::new(store.clone());
    let row = entries
        .get(&id)
        .await?
        .expect("just-inserted entry must exist");

    Ok(row.into())
}
