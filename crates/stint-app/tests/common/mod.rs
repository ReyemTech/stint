//! Shared test harness for stint-app integration tests.
//!
//! Builds a `tauri::App<MockRuntime>` per test using `tauri::test::mock_builder()`
//! and manages a fresh tempdir-backed `AppState`. Each test gets its own
//! tempdir + store, and the tempdir is kept alive for the test's scope via
//! the returned `AppContext`.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::OnceLock;
use stint_app::app_state::AppState;
use stint_core::store::Store;
use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, AppHandle, Manager};
use tempfile::TempDir;
use tokio::sync::RwLock;

/// Routes any in-process `Secrets::default()` calls — including those
/// inside `#[tauri::command]` bodies — to a synthetic prefix unique to
/// this test binary. The developer's real `tech.reyem.stint.*` Keychain
/// entries are never touched. Entries accumulate under
/// `tech.reyem.stint.test.<uuid>.*` and are swept by
/// `scripts/clean-test-keychain.sh`.
fn ensure_test_secret_prefix() -> &'static str {
    static PREFIX: OnceLock<String> = OnceLock::new();
    let prefix = PREFIX.get_or_init(|| {
        format!("tech.reyem.stint.test.{}", stint_core::ids::new_local_uuid())
    });
    std::env::set_var("STINT_SECRET_PREFIX", prefix);
    prefix
}

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
    ensure_test_secret_prefix();

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
