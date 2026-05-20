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
}
