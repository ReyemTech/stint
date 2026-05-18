pub mod secrets;

use crate::{store::Store, time, Result};

pub struct Settings {
    store: Store,
}

impl Settings {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(self.store.pool())
            .await?;
        Ok(row.map(|(v,)| v))
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        let now = time::now_utc();
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(key)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    pub async fn list_prefixed(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT key, value FROM settings WHERE key LIKE ?")
                .bind(format!("{prefix}%"))
                .fetch_all(self.store.pool())
                .await?;
        Ok(rows)
    }
}
