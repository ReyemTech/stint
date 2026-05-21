use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::config::Settings;
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_errors_when_solidtime_url_missing() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["pull"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("solidtime.url not set"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_errors_when_solidtime_org_missing() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    // URL set, org missing — exercises the second early-return.
    Settings::new(Store::connect(&db).await.unwrap())
        .set("solidtime.url", "https://time.example.com")
        .await
        .unwrap();

    cmd(&db)
        .args(["pull"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("solidtime.org not set"));
}
