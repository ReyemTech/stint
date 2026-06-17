use crate::{store::Store, time, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub client_id: Option<String>,
    #[sqlx(default)]
    pub client_name: Option<String>,
    pub archived: i64,
    #[sqlx(default)]
    pub billable_default: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClientRow {
    pub id: String,
    pub name: String,
    pub archived: i64,
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
                r#"INSERT INTO projects (id, name, color, client_id, archived, billable_default, fetched_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     color = excluded.color,
                     client_id = excluded.client_id,
                     archived = excluded.archived,
                     billable_default = excluded.billable_default,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&p.id)
            .bind(&p.name)
            .bind(&p.color)
            .bind(&p.client_id)
            .bind(p.archived)
            .bind(p.billable_default)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectRow>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            r#"SELECT p.id, p.name, p.color, p.client_id,
                      c.name AS client_name, p.archived, p.billable_default
               FROM projects p
               LEFT JOIN clients c ON c.id = p.client_id
               ORDER BY p.name"#,
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

    pub async fn list_all_tasks(&self) -> Result<Vec<TaskRow>> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, project_id, name, done FROM tasks ORDER BY name",
        )
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

    pub async fn upsert_clients(&self, clients: &[ClientRow]) -> Result<()> {
        let now = time::now_utc();
        let mut tx = self.store.pool().begin().await?;
        for c in clients {
            sqlx::query(
                r#"INSERT INTO clients (id, name, archived, fetched_at)
                   VALUES (?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     archived = excluded.archived,
                     fetched_at = excluded.fetched_at"#,
            )
            .bind(&c.id)
            .bind(&c.name)
            .bind(c.archived)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_clients(&self) -> Result<Vec<ClientRow>> {
        let rows =
            sqlx::query_as::<_, ClientRow>("SELECT id, name, archived FROM clients ORDER BY name")
                .fetch_all(self.store.pool())
                .await?;
        Ok(rows)
    }

    /// Soft-archive any project whose id isn't in `keep`. Used by the
    /// reference-data refresh path: if a project was deleted on Solidtime
    /// we set `archived = 1` locally so the picker hides it but historical
    /// time entries can still resolve the project name.
    ///
    /// An empty `keep` slice archives every project — that's the correct
    /// semantics for "remote returned zero projects."
    pub async fn archive_projects_not_in(&self, keep: &[&str]) -> Result<()> {
        let now = time::now_utc();
        let keep_json =
            serde_json::to_string(keep).map_err(|e| crate::Error::Invariant(e.to_string()))?;
        sqlx::query(
            r#"UPDATE projects
               SET archived = 1, fetched_at = ?
               WHERE archived = 0
                 AND id NOT IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&now)
        .bind(&keep_json)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    /// Soft-archive any client whose id isn't in `keep`. Same rationale as
    /// `archive_projects_not_in`.
    pub async fn archive_clients_not_in(&self, keep: &[&str]) -> Result<()> {
        let now = time::now_utc();
        let keep_json =
            serde_json::to_string(keep).map_err(|e| crate::Error::Invariant(e.to_string()))?;
        sqlx::query(
            r#"UPDATE clients
               SET archived = 1, fetched_at = ?
               WHERE archived = 0
                 AND id NOT IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&now)
        .bind(&keep_json)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    /// Hard-delete any task whose id isn't in `keep`. Tasks have no
    /// `archived` column so we delete; historical entries that referenced
    /// a deleted task will show no task name (the foreign key on
    /// entries.task_id is unenforced and resolves via JOIN at display time).
    pub async fn delete_tasks_not_in(&self, keep: &[&str]) -> Result<()> {
        let keep_json =
            serde_json::to_string(keep).map_err(|e| crate::Error::Invariant(e.to_string()))?;
        sqlx::query(
            r#"DELETE FROM tasks
               WHERE id NOT IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&keep_json)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    /// Hard-delete any tag whose id isn't in `keep`.
    pub async fn delete_tags_not_in(&self, keep: &[&str]) -> Result<()> {
        let keep_json =
            serde_json::to_string(keep).map_err(|e| crate::Error::Invariant(e.to_string()))?;
        sqlx::query(
            r#"DELETE FROM tags
               WHERE id NOT IN (SELECT value FROM json_each(?))"#,
        )
        .bind(&keep_json)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }
}
