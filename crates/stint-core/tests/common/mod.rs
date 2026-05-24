//! Shared test helpers. Each test file gets its own TempDir + Store.

#![allow(dead_code)]

use stint_core::store::reference::{ProjectRow, Reference};
use stint_core::store::Store;
use tempfile::TempDir;

pub struct TestEnv {
    pub _tempdir: TempDir,
    pub store: Store,
}

pub async fn setup() -> TestEnv {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("stint.db");
    let store = Store::connect(&db_path).await.expect("connect store");
    TestEnv {
        _tempdir: tempdir,
        store,
    }
}

/// Seed the projects reference table with `(id, name)` pairs.
pub async fn seed_projects(store: &Store, projects: &[(&str, &str)]) {
    let reference = Reference::new(store.clone());
    let rows: Vec<ProjectRow> = projects
        .iter()
        .map(|(id, name)| ProjectRow {
            id: (*id).to_string(),
            name: (*name).to_string(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 0,
        })
        .collect();
    reference
        .upsert_projects(&rows)
        .await
        .expect("seed projects");
}
