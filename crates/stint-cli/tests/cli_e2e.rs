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
