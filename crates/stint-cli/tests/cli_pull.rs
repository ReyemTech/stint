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
async fn sync_force_adopt_links_local_to_remote_and_clears_queue() {
    use stint_core::store::entries::{Entries, NewTimeEntry};
    use stint_core::store::queue::{Queue, QueueOp};

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    // Seed a stuck pending_create entry + its queue row.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store.clone());
    let local_uuid = entries
        .create(NewTimeEntry {
            description: "Liberty Issue".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-21T13:39:14Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    let queue = Queue::new(store.clone());
    queue
        .enqueue(
            QueueOp::CreateEntry,
            &serde_json::json!({ "local_uuid": local_uuid }).to_string(),
            Some(&local_uuid),
        )
        .await
        .unwrap();
    drop(queue);
    drop(entries);
    drop(store);

    cmd(&db)
        .args(["sync", "force-adopt", &local_uuid, "remote-XYZ"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Linked"))
        .stdout(predicate::str::contains("remote-XYZ"));

    // Verify local state: synced + linked.
    let store = Store::connect(&db).await.unwrap();
    let row = Entries::new(store.clone())
        .get(&local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.sync_state, "synced");
    assert_eq!(row.solidtime_id.as_deref(), Some("remote-XYZ"));
    // Queue rows for this entry are gone.
    let due = Queue::new(store.clone()).take_due(10).await.unwrap();
    assert!(
        due.iter()
            .all(|r| r.entry_uuid.as_deref() != Some(local_uuid.as_str())),
        "force-adopt should leave no queued ops for the entry",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_force_adopt_errors_on_unknown_uuid() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["sync", "force-adopt", "no-such-uuid", "remote-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
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
