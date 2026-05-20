//! Down-sync from Solidtime: running-timer adoption, history & delete
//! reconciliation. Each trigger calls `pull(...)` which runs the three
//! reconciliation sub-functions and returns a summary. The sub-functions
//! land in subsequent tasks; this file currently exposes the stub entry
//! point so callers can be wired in parallel.

pub mod window;

pub use window::{Trigger, Window};

use crate::{solidtime::SolidtimeClient, store::Store, Result};

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

/// Run a full pull cycle. Stub for Task 5; real work lands in Tasks 6+.
pub async fn pull(
    _store: &Store,
    _client: &SolidtimeClient,
    _trigger: Trigger,
) -> Result<PullReport> {
    Ok(PullReport::default())
}
