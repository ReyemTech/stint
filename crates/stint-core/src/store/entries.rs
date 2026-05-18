use crate::{ids, store::Store, time, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct NewTimeEntry {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub billable: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TimeEntryRow {
    pub local_uuid: String,
    pub solidtime_id: Option<String>,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub billable: i64,
    pub source: String,
    pub source_event_id: Option<String>,
    pub sync_state: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Entries {
    store: Store,
}

impl Entries {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn create(&self, new: NewTimeEntry) -> Result<String> {
        let local_uuid = ids::new_local_uuid();
        let now = time::now_utc();
        sqlx::query(
            r#"INSERT INTO time_entries
               (local_uuid, description, project_id, task_id, start_at, billable,
                source, sync_state, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, 'pending_create', ?, ?)"#,
        )
        .bind(&local_uuid)
        .bind(new.description)
        .bind(new.project_id)
        .bind(new.task_id)
        .bind(new.start_at)
        .bind(if new.billable { 1 } else { 0 })
        .bind(new.source)
        .bind(&now)
        .bind(&now)
        .execute(self.store.pool())
        .await?;
        Ok(local_uuid)
    }

    pub async fn get(&self, local_uuid: &str) -> Result<Option<TimeEntryRow>> {
        let row = sqlx::query_as::<_, TimeEntryRow>(
            "SELECT * FROM time_entries WHERE local_uuid = ?",
        )
        .bind(local_uuid)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row)
    }

    pub async fn list_between(&self, from: &str, to: &str) -> Result<Vec<TimeEntryRow>> {
        let rows = sqlx::query_as::<_, TimeEntryRow>(
            "SELECT * FROM time_entries WHERE start_at >= ? AND start_at <= ? ORDER BY start_at",
        )
        .bind(from)
        .bind(to)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }
}
