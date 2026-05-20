use crate::{store::Store, time, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RunningRow {
    pub local_uuid: String,
    pub heartbeat_at: String,
}

pub struct RunningTimer {
    store: Store,
}

impl RunningTimer {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn set(&self, local_uuid: &str) -> Result<()> {
        Self::set_with(self.store.pool(), local_uuid).await
    }

    /// Executor-generic variant of [`set`].
    pub async fn set_with<'e, E>(executor: E, local_uuid: &str) -> Result<()>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let now = time::now_utc();
        sqlx::query(
            "INSERT INTO running_timer (id, local_uuid, heartbeat_at) VALUES (1, ?, ?)
             ON CONFLICT(id) DO UPDATE SET local_uuid = excluded.local_uuid, heartbeat_at = excluded.heartbeat_at",
        )
        .bind(local_uuid)
        .bind(now)
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Atomic insert: only sets running_timer if no row currently exists.
    /// Returns true if the insert applied (caller now owns the slot), false
    /// if a row was already present (caller must NOT proceed as the
    /// running timer).
    ///
    /// Use this instead of `get().is_some() / set()` to close the TOCTOU
    /// race where a concurrent pull adoption could land between the check
    /// and the set.
    pub async fn try_claim_with<'e, E>(executor: E, local_uuid: &str) -> Result<bool>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let now = time::now_utc();
        let res = sqlx::query(
            "INSERT INTO running_timer (id, local_uuid, heartbeat_at) VALUES (1, ?, ?)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(local_uuid)
        .bind(now)
        .execute(executor)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    pub async fn get(&self) -> Result<Option<RunningRow>> {
        Self::get_with(self.store.pool()).await
    }

    /// Executor-generic variant of [`get`].
    pub async fn get_with<'e, E>(executor: E) -> Result<Option<RunningRow>>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        let row = sqlx::query_as::<_, RunningRow>(
            "SELECT local_uuid, heartbeat_at FROM running_timer WHERE id = 1",
        )
        .fetch_optional(executor)
        .await?;
        Ok(row)
    }

    pub async fn clear(&self) -> Result<()> {
        Self::clear_with(self.store.pool()).await
    }

    /// Executor-generic variant of [`clear`].
    pub async fn clear_with<'e, E>(executor: E) -> Result<()>
    where
        E: sqlx::SqliteExecutor<'e>,
    {
        sqlx::query("DELETE FROM running_timer WHERE id = 1")
            .execute(executor)
            .await?;
        Ok(())
    }

    pub async fn heartbeat(&self) -> Result<()> {
        sqlx::query("UPDATE running_timer SET heartbeat_at = ? WHERE id = 1")
            .bind(time::now_utc())
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
}
