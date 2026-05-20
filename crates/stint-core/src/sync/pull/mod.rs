//! Down-sync from Solidtime: running-timer adoption, history & delete
//! reconciliation. Each trigger calls `pull(...)` which runs the
//! reconciliation sub-functions and returns a summary. Task 6 wires up
//! running-timer adoption; history and deletes land in subsequent tasks.

pub mod running;
pub mod window;

pub use window::{Trigger, Window};

use crate::{
    config::Settings,
    solidtime::SolidtimeClient,
    store::Store,
    Error, Result,
};
use chrono::Utc;

#[derive(Debug, Default, Clone)]
pub struct PullReport {
    pub adopted: Option<String>,
    pub conflict: Option<ConflictInfo>,
    pub inserted: usize,
    pub updated: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub remote_id: String,
    pub remote_description: String,
    pub remote_start_at: String,
    pub local_local_uuid: String,
    pub local_description: String,
}

pub async fn pull(
    store: &Store,
    client: &SolidtimeClient,
    trigger: Trigger,
) -> Result<PullReport> {
    let settings = Settings::new(store.clone());
    let member_id = settings
        .get("solidtime.member_id")
        .await?
        .ok_or(Error::MissingConfig("solidtime.member_id"))?;

    let window = Window::for_trigger(trigger, Utc::now());
    let from = window.from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let to = window.to.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let remote_entries = client.list_time_entries(&member_id, &from, &to).await?;

    let running_outcome = running::reconcile_running(store, client, &remote_entries).await?;

    Ok(PullReport {
        adopted: running_outcome.adopted,
        conflict: running_outcome.conflict,
        inserted: 0,
        updated: 0,
        deleted: 0,
    })
}
