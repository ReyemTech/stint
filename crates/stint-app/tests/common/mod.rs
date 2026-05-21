//! Shared test harness for stint-app integration tests.
//!
//! Builds a `tauri::App<MockRuntime>` per test using `tauri::test::mock_builder()`
//! and manages a fresh tempdir-backed `AppState`. Each test gets its own
//! tempdir + store, and the tempdir is kept alive for the test's scope via
//! the returned `AppContext`.

#![allow(dead_code)]

use std::sync::Arc;
use stint_app::app_state::AppState;
use stint_core::store::Store;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, AppHandle, Manager};
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Owns the live test app, the tempdir backing its database, and a
/// direct `Arc<Store>` handle so tests can poke the database without
/// going through Tauri state.
pub struct AppContext {
    pub app: App<MockRuntime>,
    pub store: Arc<Store>,
    pub _tempdir: TempDir,
}

impl AppContext {
    pub fn handle(&self) -> AppHandle<MockRuntime> {
        self.app.handle().clone()
    }
}

/// Build a fresh tempdir-backed store, then a mock Tauri app with the
/// `RwLock<AppState>` managed on its handle. Mirrors what
/// `crates/stint-app/src/main.rs::setup` does in production.
pub async fn make_app() -> AppContext {
    let tempdir = TempDir::new().expect("create tempdir");
    let db_path = tempdir.path().join("stint.db");
    let store = Arc::new(Store::connect(&db_path).await.expect("connect store"));

    let app = mock_builder()
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    app.manage(RwLock::new(AppState {
        store: store.clone(),
    }));

    AppContext {
        app,
        store,
        _tempdir: tempdir,
    }
}
