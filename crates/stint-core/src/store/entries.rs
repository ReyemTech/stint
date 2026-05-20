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

#[derive(Debug, Clone)]
pub struct NewCompletedEntry {
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: String,
    pub billable: bool,
    pub source: String,
    pub source_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RemoteEntryUpsert {
    pub solidtime_id: String,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub start_at: String,
    pub end_at: Option<String>,
    pub billable: bool,
    pub updated_at: String,
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
        Self::create_with(self.store.pool(), new).await
    }

    /// Executor-generic variant of [`create`].
    pub async fn create_with<'e, E>(executor: E, new: NewTimeEntry) -> Result<String>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
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
        .execute(executor)
        .await?;
        Ok(local_uuid)
    }

    /// Insert a finalised time entry (both start_at and end_at set), used by
    /// the calendar "Log this" path and any future bulk-import flow. The
    /// entry begins in `pending_create` so the regular sync queue picks it
    /// up exactly like a CLI/GUI-created entry.
    pub async fn create_completed(&self, new: NewCompletedEntry) -> Result<String> {
        let local_uuid = ids::new_local_uuid();
        let now = time::now_utc();
        sqlx::query(
            r#"INSERT INTO time_entries
               (local_uuid, description, project_id, task_id, start_at, end_at,
                billable, source, source_event_id, sync_state, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending_create', ?, ?)"#,
        )
        .bind(&local_uuid)
        .bind(new.description)
        .bind(new.project_id)
        .bind(new.task_id)
        .bind(new.start_at)
        .bind(new.end_at)
        .bind(if new.billable { 1 } else { 0 })
        .bind(new.source)
        .bind(new.source_event_id)
        .bind(&now)
        .bind(&now)
        .execute(self.store.pool())
        .await?;
        Ok(local_uuid)
    }

    pub async fn get(&self, local_uuid: &str) -> Result<Option<TimeEntryRow>> {
        Self::get_with(self.store.pool(), local_uuid).await
    }

    /// Executor-generic variant of [`get`], used inside transactional
    /// reconcile pipelines so reads and writes share one connection.
    pub async fn get_with<'e, E>(executor: E, local_uuid: &str) -> Result<Option<TimeEntryRow>>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let row =
            sqlx::query_as::<_, TimeEntryRow>("SELECT * FROM time_entries WHERE local_uuid = ?")
                .bind(local_uuid)
                .fetch_optional(executor)
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

    pub async fn mark_synced(&self, local_uuid: &str, solidtime_id: &str) -> Result<()> {
        let now = time::now_utc();
        sqlx::query(
            "UPDATE time_entries
             SET solidtime_id = ?, sync_state = 'synced', updated_at = ?
             WHERE local_uuid = ?",
        )
        .bind(solidtime_id)
        .bind(&now)
        .bind(local_uuid)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn set_end(&self, local_uuid: &str, end_at: &str) -> Result<()> {
        self.update_one(local_uuid, |s| {
            sqlx::query(
                "UPDATE time_entries
                 SET end_at = ?, sync_state = ?, updated_at = ?
                 WHERE local_uuid = ?",
            )
            .bind(end_at)
            .bind(s.next_state())
            .bind(time::now_utc())
            .bind(local_uuid)
        })
        .await
    }

    pub async fn update_description(&self, local_uuid: &str, description: &str) -> Result<()> {
        self.update_one(local_uuid, |s| {
            sqlx::query(
                "UPDATE time_entries
                 SET description = ?, sync_state = ?, updated_at = ?
                 WHERE local_uuid = ?",
            )
            .bind(description)
            .bind(s.next_state())
            .bind(time::now_utc())
            .bind(local_uuid)
        })
        .await
    }

    pub async fn set_project(&self, local_uuid: &str, project_id: Option<&str>) -> Result<()> {
        self.update_one(local_uuid, |s| {
            sqlx::query(
                "UPDATE time_entries
                 SET project_id = ?, sync_state = ?, updated_at = ?
                 WHERE local_uuid = ?",
            )
            .bind(project_id)
            .bind(s.next_state())
            .bind(time::now_utc())
            .bind(local_uuid)
        })
        .await
    }

    pub async fn set_billable(&self, local_uuid: &str, billable: bool) -> Result<()> {
        self.update_one(local_uuid, |s| {
            sqlx::query(
                "UPDATE time_entries
                 SET billable = ?, sync_state = ?, updated_at = ?
                 WHERE local_uuid = ?",
            )
            .bind(if billable { 1 } else { 0 })
            .bind(s.next_state())
            .bind(time::now_utc())
            .bind(local_uuid)
        })
        .await
    }

    pub async fn delete(&self, local_uuid: &str) -> Result<()> {
        let state = self.current_state(local_uuid).await?;
        match state.as_str() {
            "pending_create" => {
                sqlx::query("DELETE FROM time_entries WHERE local_uuid = ?")
                    .bind(local_uuid)
                    .execute(self.store.pool())
                    .await?;
            }
            _ => {
                sqlx::query(
                    "UPDATE time_entries SET sync_state = 'pending_delete', updated_at = ? WHERE local_uuid = ?",
                )
                .bind(time::now_utc())
                .bind(local_uuid)
                .execute(self.store.pool())
                .await?;
            }
        }
        Ok(())
    }

    pub async fn create_from_remote(&self, e: RemoteEntryUpsert) -> Result<String> {
        Self::create_from_remote_with(self.store.pool(), e).await
    }

    /// Executor-generic variant of [`create_from_remote`].
    pub async fn create_from_remote_with<'e, E>(executor: E, e: RemoteEntryUpsert) -> Result<String>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let local_uuid = ids::new_local_uuid();
        sqlx::query(
            r#"INSERT INTO time_entries
               (local_uuid, solidtime_id, description, project_id, task_id,
                start_at, end_at, billable, source, sync_state,
                created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'solidtime', 'synced', ?, ?)"#,
        )
        .bind(&local_uuid)
        .bind(&e.solidtime_id)
        .bind(&e.description)
        .bind(&e.project_id)
        .bind(&e.task_id)
        .bind(&e.start_at)
        .bind(&e.end_at)
        .bind(if e.billable { 1 } else { 0 })
        .bind(&e.updated_at)
        .bind(&e.updated_at)
        .execute(executor)
        .await?;
        Ok(local_uuid)
    }

    pub async fn get_by_solidtime_id(&self, solidtime_id: &str) -> Result<Option<TimeEntryRow>> {
        Self::get_by_solidtime_id_with(self.store.pool(), solidtime_id).await
    }

    /// Executor-generic variant of [`get_by_solidtime_id`].
    pub async fn get_by_solidtime_id_with<'e, E>(
        executor: E,
        solidtime_id: &str,
    ) -> Result<Option<TimeEntryRow>>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let row =
            sqlx::query_as::<_, TimeEntryRow>("SELECT * FROM time_entries WHERE solidtime_id = ?")
                .bind(solidtime_id)
                .fetch_optional(executor)
                .await?;
        Ok(row)
    }

    pub async fn update_from_remote(
        &self,
        solidtime_id: &str,
        e: RemoteEntryUpsert,
    ) -> Result<bool> {
        Self::update_from_remote_with(self.store.pool(), solidtime_id, e).await
    }

    /// Executor-generic variant of [`update_from_remote`]. Preserves the
    /// `AND sync_state = 'synced'` clause so pending-mutation rows are
    /// never clobbered by a remote update.
    pub async fn update_from_remote_with<'e, E>(
        executor: E,
        solidtime_id: &str,
        e: RemoteEntryUpsert,
    ) -> Result<bool>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let res = sqlx::query(
            r#"UPDATE time_entries
               SET description = ?, project_id = ?, task_id = ?,
                   start_at = ?, end_at = ?, billable = ?, updated_at = ?
               WHERE solidtime_id = ? AND sync_state = 'synced'"#,
        )
        .bind(&e.description)
        .bind(&e.project_id)
        .bind(&e.task_id)
        .bind(&e.start_at)
        .bind(&e.end_at)
        .bind(if e.billable { 1 } else { 0 })
        .bind(&e.updated_at)
        .bind(solidtime_id)
        .execute(executor)
        .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn hard_delete_by_solidtime_id(&self, solidtime_id: &str) -> Result<bool> {
        Self::hard_delete_by_solidtime_id_with(self.store.pool(), solidtime_id).await
    }

    /// Executor-generic variant of [`hard_delete_by_solidtime_id`].
    pub async fn hard_delete_by_solidtime_id_with<'e, E>(
        executor: E,
        solidtime_id: &str,
    ) -> Result<bool>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let res = sqlx::query("DELETE FROM time_entries WHERE solidtime_id = ?")
            .bind(solidtime_id)
            .execute(executor)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn list_synced_in_window(&self, from: &str, to: &str) -> Result<Vec<TimeEntryRow>> {
        Self::list_synced_in_window_with(self.store.pool(), from, to).await
    }

    /// Executor-generic variant of [`list_synced_in_window`].
    pub async fn list_synced_in_window_with<'e, E>(
        executor: E,
        from: &str,
        to: &str,
    ) -> Result<Vec<TimeEntryRow>>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let rows = sqlx::query_as::<_, TimeEntryRow>(
            r#"SELECT * FROM time_entries
               WHERE sync_state = 'synced'
                 AND solidtime_id IS NOT NULL
                 AND start_at >= ? AND start_at <= ?
               ORDER BY start_at"#,
        )
        .bind(from)
        .bind(to)
        .fetch_all(executor)
        .await?;
        Ok(rows)
    }

    async fn current_state(&self, local_uuid: &str) -> Result<String> {
        let s: (String,) =
            sqlx::query_as("SELECT sync_state FROM time_entries WHERE local_uuid = ?")
                .bind(local_uuid)
                .fetch_one(self.store.pool())
                .await?;
        Ok(s.0)
    }

    async fn update_one<'q, F>(&self, local_uuid: &'q str, build: F) -> Result<()>
    where
        F: FnOnce(
            StateForUpdate,
        )
            -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    {
        let state_str = self.current_state(local_uuid).await?;
        let s = StateForUpdate::from(state_str.as_str());
        build(s).execute(self.store.pool()).await?;
        Ok(())
    }
}

#[derive(Copy, Clone)]
enum StateForUpdate {
    PendingCreate,
    Synced,
    Dirty,
    PendingDelete,
}

impl StateForUpdate {
    fn next_state(self) -> &'static str {
        match self {
            // Still unsynced — stay in pending_create regardless of edits.
            StateForUpdate::PendingCreate => "pending_create",
            // Anything that was server-side gets marked dirty after a local edit.
            StateForUpdate::Synced | StateForUpdate::Dirty => "dirty",
            // pending_delete shouldn't be edited; if it is, keep it pending_delete.
            StateForUpdate::PendingDelete => "pending_delete",
        }
    }

    fn from(s: &str) -> Self {
        match s {
            "pending_create" => Self::PendingCreate,
            "synced" => Self::Synced,
            "dirty" => Self::Dirty,
            "pending_delete" => Self::PendingDelete,
            _ => Self::Synced,
        }
    }
}
