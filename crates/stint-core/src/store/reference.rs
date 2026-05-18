use crate::{store::Store, time, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub client_id: Option<String>,
    pub archived: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub done: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TagRow {
    pub id: String,
    pub name: String,
}

pub struct Reference {
    store: Store,
}

impl Reference {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub async fn upsert_projects(&self, projects: &[ProjectRow]) -> Result<()> {
        let now = time::now_utc();
        let mut tx = self.store.pool().begin().await?;
        for p in projects {
            sqlx::query(
                r#"INSERT INTO projects (id, name, color, client_id, archived, fetched_at)
                   VALUES (?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     color = excluded.color,
                     client_id = excluded.client_id,
                     archived = excluded.archived,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&p.id)
            .bind(&p.name)
            .bind(&p.color)
            .bind(&p.client_id)
            .bind(p.archived)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectRow>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT id, name, color, client_id, archived FROM projects ORDER BY name",
        )
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }

    pub async fn upsert_tasks(&self, tasks: &[TaskRow]) -> Result<()> {
        let now = time::now_utc();
        let mut tx = self.store.pool().begin().await?;
        for t in tasks {
            sqlx::query(
                r#"INSERT INTO tasks (id, project_id, name, done, fetched_at)
                   VALUES (?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     project_id = excluded.project_id,
                     name = excluded.name,
                     done = excluded.done,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&t.id)
            .bind(&t.project_id)
            .bind(&t.name)
            .bind(t.done)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_tasks(&self, project_id: &str) -> Result<Vec<TaskRow>> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, project_id, name, done FROM tasks WHERE project_id = ? ORDER BY name",
        )
        .bind(project_id)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows)
    }

    pub async fn upsert_tags(&self, tags: &[TagRow]) -> Result<()> {
        let now = time::now_utc();
        let mut tx = self.store.pool().begin().await?;
        for t in tags {
            sqlx::query(
                r#"INSERT INTO tags (id, name, fetched_at)
                   VALUES (?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&t.id)
            .bind(&t.name)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_tags(&self) -> Result<Vec<TagRow>> {
        let rows = sqlx::query_as::<_, TagRow>("SELECT id, name FROM tags ORDER BY name")
            .fetch_all(self.store.pool())
            .await?;
        Ok(rows)
    }
}
