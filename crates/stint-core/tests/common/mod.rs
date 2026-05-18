//! Shared test helpers. Each test file gets its own TempDir + Store.

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
    TestEnv { _tempdir: tempdir, store }
}
