use crate::{store::Store, time, Result};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QueueOp {
    CreateEntry,
    UpdateEntry,
    DeleteEntry,
}

impl QueueOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CreateEntry => "create_entry",
            Self::UpdateEntry => "update_entry",
            Self::DeleteEntry => "delete_entry",
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct QueueRow {
    pub id: i64,
    pub op: String,
    pub payload: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub enqueued_at: String,
    pub next_try_at: String,
    pub entry_uuid: Option<String>,
}

/// Queue row + joined time_entries metadata for UI surfacing.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct FailedQueueRow {
    pub queue_id: i64,
    pub local_uuid: Option<String>,
    pub op: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub next_try_at: String,
    pub description: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
}

pub struct Queue {
    store: Store,
}

impl Queue {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn enqueue(
        &self,
        op: QueueOp,
        payload: &str,
        entry_uuid: Option<&str>,
    ) -> Result<i64> {
        Self::enqueue_with(self.store.pool(), op, payload, entry_uuid).await
    }

    /// Executor-generic variant of [`enqueue`].
    pub async fn enqueue_with<'e, E>(
        executor: E,
        op: QueueOp,
        payload: &str,
        entry_uuid: Option<&str>,
    ) -> Result<i64>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let now = time::now_utc();
        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO sync_queue (op, payload, attempts, enqueued_at, next_try_at, entry_uuid)
               VALUES (?, ?, 0, ?, ?, ?) RETURNING id"#,
        )
        .bind(op.as_str())
        .bind(payload)
        .bind(&now)
        .bind(&now)
        .bind(entry_uuid)
        .fetch_one(executor)
        .await?;
        Ok(id)
    }

    pub async fn take_due(&self, limit: i64) -> Result<Vec<QueueRow>> {
        let now = time::now_utc();
        let rows = sqlx::query_as::<_, QueueRow>(
            "SELECT * FROM sync_queue WHERE next_try_at <= ? ORDER BY id LIMIT ?",
        )
        .bind(now)
        .bind(limit)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }

    pub async fn mark_succeeded(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM sync_queue WHERE id = ?")
            .bind(id)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn mark_failed(&self, id: i64, err: &str) -> Result<()> {
        // Read attempts to compute backoff.
        let (attempts,): (i64,) = sqlx::query_as("SELECT attempts FROM sync_queue WHERE id = ?")
            .bind(id)
            .fetch_one(self.store.pool())
            .await?;
        let next_attempt = attempts + 1;
        let backoff_secs = (1u64 << next_attempt.min(8)) as i64; // 2,4,8,...,256
        let backoff_secs = backoff_secs.min(300); // cap at 5 min
        let next_try = Utc::now() + Duration::seconds(backoff_secs);
        let next_try_str = time::format(&next_try);

        sqlx::query(
            "UPDATE sync_queue
             SET attempts = ?, last_error = ?, next_try_at = ?
             WHERE id = ?",
        )
        .bind(next_attempt)
        .bind(err)
        .bind(next_try_str)
        .bind(id)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    /// Reset `next_try_at` to now on any rows parked >30 days in the
    /// future — i.e. rows previously parked by `mark_abandoned`. Used by
    /// `stint sync retry-abandoned` after the user fixes whatever made
    /// the abandonment necessary (e.g. they stopped the remote timer
    /// that was causing overlap). Returns the number of rows revived.
    pub async fn resurrect_abandoned(&self) -> Result<u64> {
        let cutoff_str = time::format(&(Utc::now() + Duration::days(30)));
        let now = time::now_utc();
        let result = sqlx::query(
            "UPDATE sync_queue SET next_try_at = ?, attempts = 0
             WHERE next_try_at > ?",
        )
        .bind(now)
        .bind(cutoff_str)
        .execute(self.store.pool())
        .await?;
        Ok(result.rows_affected())
    }

    /// Return queue rows that have failed at least `min_attempts` times,
    /// joined with their time_entries row (description/start/end) so the
    /// UI can render a meaningful "sync failed" banner without further
    /// lookups. Ordered by `next_try_at` descending (abandoned rows
    /// first, since they sit further in the future).
    pub async fn list_failed_with_entry(&self, min_attempts: i64) -> Result<Vec<FailedQueueRow>> {
        let rows: Vec<FailedQueueRow> = sqlx::query_as(
            r#"SELECT q.id            AS queue_id,
                      q.entry_uuid    AS local_uuid,
                      q.op            AS op,
                      q.attempts      AS attempts,
                      q.last_error    AS last_error,
                      q.next_try_at   AS next_try_at,
                      e.description   AS description,
                      e.start_at      AS start_at,
                      e.end_at        AS end_at
               FROM sync_queue q
               LEFT JOIN time_entries e ON e.local_uuid = q.entry_uuid
               WHERE q.attempts >= ?
               ORDER BY q.next_try_at DESC"#,
        )
        .bind(min_attempts)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }

    /// Delete every queued op tied to a specific local entry. Used by
    /// `stint sync force-adopt` after manually linking a pending_create
    /// row to a remote id — leaving the create_entry op in place would
    /// re-POST a duplicate next drain.
    pub async fn delete_for_entry(&self, local_uuid: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM sync_queue WHERE entry_uuid = ?")
            .bind(local_uuid)
            .execute(self.store.pool())
            .await?;
        Ok(result.rows_affected())
    }

    /// Mark a queue item as abandoned: parks `next_try_at` ~1 year out so
    /// the worker stops picking it up. The row is preserved (with the
    /// error message) so the user can see why sync gave up. Use this for
    /// permanent 4xx rejections from Solidtime — retrying validation /
    /// business-rule errors just floods logs without resolving anything.
    pub async fn mark_abandoned(&self, id: i64, err: &str) -> Result<()> {
        let (attempts,): (i64,) = sqlx::query_as("SELECT attempts FROM sync_queue WHERE id = ?")
            .bind(id)
            .fetch_one(self.store.pool())
            .await?;
        let next_try = Utc::now() + Duration::days(365);
        let next_try_str = time::format(&next_try);

        sqlx::query(
            "UPDATE sync_queue
             SET attempts = ?, last_error = ?, next_try_at = ?
             WHERE id = ?",
        )
        .bind(attempts + 1)
        .bind(err)
        .bind(next_try_str)
        .bind(id)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }
}
