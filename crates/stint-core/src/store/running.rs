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
        let now = time::now_utc();
        sqlx::query(
            "INSERT INTO running_timer (id, local_uuid, heartbeat_at) VALUES (1, ?, ?)
             ON CONFLICT(id) DO UPDATE SET local_uuid = excluded.local_uuid, heartbeat_at = excluded.heartbeat_at",
        )
        .bind(local_uuid)
        .bind(now)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn get(&self) -> Result<Option<RunningRow>> {
        let row = sqlx::query_as::<_, RunningRow>(
            "SELECT local_uuid, heartbeat_at FROM running_timer WHERE id = 1",
        )
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row)
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query("DELETE FROM running_timer WHERE id = 1")
            .execute(self.store.pool())
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
