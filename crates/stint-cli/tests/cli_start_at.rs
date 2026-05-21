use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::entries::Entries;
use stint_core::store::Store;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
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
async fn start_with_at_15min_ago_backdates_entry() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "deep work", "--at", "15min ago"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started: deep work"));

    let store = Store::connect(&db).await.unwrap();
    let entries = Entries::new(store);
    let rows = entries
        .list_between("2000-01-01T00:00:00Z", "2099-01-01T00:00:00Z")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let start = chrono::DateTime::parse_from_rfc3339(&rows[0].start_at).unwrap();
    let diff = chrono::Utc::now()
        .signed_duration_since(start.with_timezone(&chrono::Utc))
        .num_seconds();
    assert!(
        (885..=915).contains(&diff),
        "expected ~900s (15min) ago, got {diff}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_with_at_future_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let future = (chrono::Utc::now() + chrono::Duration::hours(1))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    cmd(&db)
        .args(["start", "future deep work", "--at", &future])
        .assert()
        .failure()
        .stderr(predicate::str::contains("future"));
}

#[tokio::test(flavor = "multi_thread")]
async fn start_with_at_garbage_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "x", "--at", "yesterday"])
        .assert()
        .failure();
}
