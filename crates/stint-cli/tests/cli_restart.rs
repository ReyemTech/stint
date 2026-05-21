use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::store::Store;
use stint_core::timer::{StartArgs, TimerService};
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_clones_metadata_into_a_new_running_timer() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    // Seed a completed template entry by starting + stopping via core.
    let store = Store::connect(&db).await.unwrap();
    let timer = TimerService::new(store.clone());
    let template_id = timer
        .start(StartArgs {
            description: "deep work".into(),
            project_id: Some("proj-x".into()),
            task_id: None,
            billable: true,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();
    timer.stop().await.unwrap();
    drop(store);

    cmd(&db)
        .args(["restart", &template_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Restarted: deep work"));

    // Re-open the store to read the new running entry.
    let store = Store::connect(&db).await.unwrap();
    let running = RunningTimer::new(store.clone())
        .get()
        .await
        .unwrap()
        .expect("a timer is now running");
    let new_row = Entries::new(store.clone())
        .get(&running.local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(new_row.local_uuid, template_id);
    assert_eq!(new_row.description, "deep work");
    assert_eq!(new_row.project_id.as_deref(), Some("proj-x"));
    assert_eq!(new_row.billable, 1);
    assert!(new_row.end_at.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_errors_on_unknown_uuid() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["restart", "not-a-real-uuid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
