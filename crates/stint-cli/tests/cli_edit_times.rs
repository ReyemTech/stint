use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::{Entries, NewTimeEntry};
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    // STINT_SECRET_PREFIX is unused for `edit` (no keychain touch) but added
    // defensively in case the CLI initializes a Secrets handle on startup.
    c.env(
        "STINT_SECRET_PREFIX",
        format!(
            "tech.reyem.stint.test.{}",
            stint_core::ids::new_local_uuid()
        ),
    );
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_start_and_end_updates_times_keeping_date() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    // Seed a completed entry on 2026-05-20.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store.clone());
    let id = entries
        .create(NewTimeEntry {
            description: "seed".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    entries.set_end(&id, "2026-05-20T10:00:00Z").await.unwrap();

    cmd(&db)
        .args(["edit", &id, "--start", "09:15", "--end", "10:45"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated times"));

    // Re-read directly to assert. We're storing UTC, the CLI accepts HH:MM
    // as local time and converts. The stored value depends on the test
    // machine's TZ; the invariant we check is that the date portion of the
    // stored UTC string is plausible (either 2026-05-20 or, if local TZ is
    // far east of UTC, 2026-05-19) and that end > start.
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let row = entries.get(&id).await.unwrap().unwrap();
    let start_date = &row.start_at[..10];
    let end_date = &row.end_at.as_ref().unwrap()[..10];
    assert!(
        matches!(start_date, "2026-05-19" | "2026-05-20"),
        "unexpected start date: {start_date}"
    );
    assert!(
        matches!(end_date, "2026-05-19" | "2026-05-20" | "2026-05-21"),
        "unexpected end date: {end_date}"
    );
    assert!(row.end_at.as_ref().unwrap().as_str() > row.start_at.as_str());
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_without_flags_is_a_no_op() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let id = entries
        .create(NewTimeEntry {
            description: "seed".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();

    cmd(&db)
        .args(["edit", &id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Nothing to update"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_times_fails_on_running_entry() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let id = entries
        .create(NewTimeEntry {
            description: "running".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    // No set_end → entry stays running.

    cmd(&db)
        .args(["edit", &id, "--start", "09:30"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("running entry"));
}
