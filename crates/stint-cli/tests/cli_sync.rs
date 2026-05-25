//! Coverage for `stint sync` subcommands that don't need a live Solidtime:
//! `force-adopt` (purely local) and `retry-abandoned` with an empty queue.
//! The drain happy-path is already covered by cli_e2e::sync_drains_one_pending_create.

use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::Entries;
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn force_adopt_links_local_entry_to_remote_id_and_clears_queue() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db).args(["start", "adopt-me"]).assert().success();
    cmd(&db).args(["stop"]).assert().success();

    // Find the freshly-created entry's local_uuid.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store.clone());
    let id = entries
        .list_between("2000-01-01T00:00:00Z", "2100-01-01T00:00:00Z")
        .await
        .unwrap()
        .first()
        .unwrap()
        .local_uuid
        .clone();
    drop(store);

    cmd(&db)
        .args(["sync", "force-adopt", &id, "remote-fixed-id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked"))
        .stdout(predicate::str::contains("remote-fixed-id"))
        .stdout(predicate::str::contains("Cleared 1"));

    // Confirm the local row is now linked to the remote id.
    let store = Store::connect(&db).await.unwrap();
    let row = Entries::new(store).get(&id).await.unwrap().unwrap();
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-fixed-id"));
    assert_eq!(row.sync_state, "synced");
}

#[tokio::test(flavor = "multi_thread")]
async fn force_adopt_errors_when_local_uuid_does_not_exist() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db)
        .args(["sync", "force-adopt", "missing-uuid", "remote-x"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found locally"));
}

#[test]
fn retry_abandoned_reports_zero_on_empty_queue() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db)
        .args(["sync", "retry-abandoned"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reset 0 abandoned"));
}

#[test]
fn retry_abandoned_json_emits_ack_shape() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let out = cmd(&db)
        .args(["--json", "sync", "retry-abandoned"])
        .output()
        .expect("retry-abandoned --json");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(v["reset"], 0);
    assert_eq!(v["drained"], 0);
}
