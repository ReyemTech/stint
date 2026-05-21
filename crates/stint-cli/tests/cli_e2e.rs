use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn full_start_stop_sync_flow() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "e2e", "start": "2026-05-17T09:00:00Z" }
        })))
        .mount(&server)
        .await;

    cmd(&db)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd(&db)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success();

    cmd(&db)
        .args(["start", "e2e"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started: e2e"));
    cmd(&db)
        .args(["stop"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stopped:"));

    cmd(&db)
        .args(["today"])
        .assert()
        .success()
        .stdout(predicate::str::contains("e2e"));
}

fn wide_range() -> [&'static str; 2] {
    ["2000-01-01T00:00:00Z", "2100-01-01T00:00:00Z"]
}

fn empty_range() -> [&'static str; 2] {
    ["1990-01-01T00:00:00Z", "1990-01-02T00:00:00Z"]
}

#[tokio::test(flavor = "multi_thread")]
async fn start_rejects_second_running_timer() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "first task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Started: first task"));

    cmd(&db)
        .args(["start", "second task"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("a timer is already running"));
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_fails_when_no_timer_is_running() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["stop"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no timer running"));
}

#[tokio::test(flavor = "multi_thread")]
async fn today_reports_empty_state_and_lists_completed_and_running_entries() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["today"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No entries today."));

    cmd(&db)
        .args(["start", "finished task"])
        .assert()
        .success();
    cmd(&db).args(["stop"]).assert().success();

    cmd(&db)
        .args(["start", "running task"])
        .assert()
        .success();

    cmd(&db)
        .args(["today"])
        .assert()
        .success()
        .stdout(predicate::str::contains("duration"))
        .stdout(predicate::str::contains("description"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("finished task"))
        .stdout(predicate::str::contains("running task"))
        .stdout(predicate::str::contains("pending_create"))
        .stdout(predicate::str::contains("RUNNING"));
}

#[tokio::test(flavor = "multi_thread")]
async fn list_shows_entries_in_range_and_nothing_outside_it() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "listed task"])
        .assert()
        .success();
    cmd(&db).args(["stop"]).assert().success();

    let [from, to] = wide_range();
    cmd(&db)
        .args(["list", from, to])
        .assert()
        .success()
        .stdout(predicate::str::contains("listed task"))
        .stdout(predicate::str::contains("[pending_create]"));

    let [from, to] = empty_range();
    cmd(&db)
        .args(["list", from, to])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}
