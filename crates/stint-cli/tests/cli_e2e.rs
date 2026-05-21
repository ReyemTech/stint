use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::store::{entries::Entries, Store};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

/// Per-test secret namespace. Threaded through the spawned `stint`
/// subprocess (via STINT_SECRET_PREFIX) so tests never touch the
/// developer's real `tech.reyem.stint.*` keychain entries. The synthetic
/// entries leak into the test-prefix namespace — harmless, and swept by
/// scripts/clean-test-keychain.sh.
///
/// We intentionally do NOT install a Drop guard that deletes the
/// synthetic entry: the test process has a different cdhash from the
/// subprocess that created the entry, so a cross-process delete triggers
/// a macOS keychain prompt. The leakage is bounded (one entry per test
/// per `cargo test` invocation, all under tech.reyem.stint.test.*).
fn unique_test_prefix() -> String {
    format!(
        "tech.reyem.stint.test.{}",
        stint_core::ids::new_local_uuid()
    )
}

fn cmd_with_prefix(db: &std::path::Path, prefix: &str) -> Command {
    let mut c = cmd(db);
    c.env("STINT_SECRET_PREFIX", prefix);
    c
}

async fn first_entry_id(db: &std::path::Path) -> String {
    let store = Store::connect(db).await.expect("connect temp store");
    let entries = Entries::new(store);
    let [from, to] = wide_range();
    let rows = entries
        .list_between(from, to)
        .await
        .expect("list entries in wide range");
    rows.first().expect("entry row present").local_uuid.clone()
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

#[test]
fn config_show_round_trips_non_secret_settings() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = unique_test_prefix();

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url", "https://time.example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved solidtime.url."));
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved solidtime.org."));

    cmd_with_prefix(&db, &prefix)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "solidtime.url = https://time.example.com",
        ))
        .stdout(predicate::str::contains("solidtime.org = org-1"));
}

#[test]
fn config_set_requires_value_for_non_secret_keys() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = unique_test_prefix();

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("value required for solidtime.url"));
}

#[tokio::test(flavor = "multi_thread")]
async fn config_test_fails_when_solidtime_url_is_missing() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["config", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("solidtime.url not set"));
}

#[tokio::test(flavor = "multi_thread")]
async fn config_test_succeeds_against_solidtime_and_masks_secret_in_show() {
    let prefix = unique_test_prefix();

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/users/me"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "id": "user-1", "email": "me@example.com" }
        })))
        .mount(&server)
        .await;

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Saved solidtime.token to Keychain.",
        ));

    cmd_with_prefix(&db, &prefix)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "solidtime.url = {}",
            server.uri()
        )))
        .stdout(predicate::str::contains("solidtime.token = ••••"));

    cmd_with_prefix(&db, &prefix)
        .args(["config", "test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("connected as me@example.com"));
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

    cmd(&db).args(["start", "finished task"]).assert().success();
    cmd(&db).args(["stop"]).assert().success();

    cmd(&db).args(["start", "running task"]).assert().success();

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

    cmd(&db).args(["start", "listed task"]).assert().success();
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

#[tokio::test(flavor = "multi_thread")]
async fn edit_without_flags_is_a_no_op() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["edit", "deadbeef"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Nothing to update. Pass --description to change something.",
        ));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_description_updates_the_entry() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["start", "original description"])
        .assert()
        .success();
    cmd(&db).args(["stop"]).assert().success();

    let entry_id = first_entry_id(&db).await;

    cmd(&db)
        .args(["edit", &entry_id, "--description", "edited description"])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Updated description for {entry_id}."
        )));

    let [from, to] = wide_range();
    cmd(&db)
        .args(["list", from, to])
        .assert()
        .success()
        .stdout(predicate::str::contains("edited description"))
        .stdout(predicate::str::contains("[pending_create]"));
}

#[tokio::test(flavor = "multi_thread")]
async fn edit_description_reports_missing_entries() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["edit", "deadbeef", "--description", "edited description"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_removes_existing_entries_and_reports_missing_ones() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db).args(["start", "delete me"]).assert().success();
    cmd(&db).args(["stop"]).assert().success();

    let entry_id = first_entry_id(&db).await;

    cmd(&db)
        .args(["delete", &entry_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!("Deleted {entry_id}.")));

    let [from, to] = wide_range();
    cmd(&db)
        .args(["list", from, to])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    cmd(&db)
        .args(["delete", "deadbeef"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[tokio::test(flavor = "multi_thread")]
async fn projects_list_is_empty_before_refresh() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn projects_refresh_populates_cached_projects() {
    let prefix = unique_test_prefix();

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/projects"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "p1", "name": "Tet", "color": null, "client_id": null, "archived": false }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tasks"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "t1", "project_id": "p1", "name": "Ship it", "done": false }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/tags"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "g1", "name": "billable" }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/clients"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "id": "c1", "name": "Acme", "archived": false }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success();

    cmd_with_prefix(&db, &prefix)
        .args(["projects", "refresh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("refreshed"));

    cmd_with_prefix(&db, &prefix)
        .args(["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("p1  Tet"));
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_drains_one_pending_create() {
    // Unique synthetic Keychain prefix per test. No SecretRestoreGuard:
    // with a fresh-per-test prefix there's no prior value to restore, and
    // having the test process attempt a `delete` on an entry created by
    // the `stint` subprocess (different cdhash) triggers a macOS prompt.
    // The synthetic entry leaks into the test prefix — harmless and
    // periodically swept by scripts/clean-test-keychain.sh.
    let prefix = unique_test_prefix();

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": { "id": "remote-1", "description": "x", "start": "2026-05-20T09:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.member_id", "m-1"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success();

    // Start a timer to enqueue a create_entry op.
    cmd_with_prefix(&db, &prefix)
        .args(["start", "deep work"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["stop"])
        .assert()
        .success();

    cmd_with_prefix(&db, &prefix)
        .args(["sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Drained 1 item"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_with_empty_remote_reports_zero_changes() {
    // See sync_drains_one_pending_create above for the no-Drop rationale.
    let prefix = unique_test_prefix();

    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/organizations/org-1/time-entries"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": []
        })))
        .mount(&server)
        .await;

    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.member_id", "m-1"])
        .assert()
        .success();
    cmd_with_prefix(&db, &prefix)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success();

    cmd_with_prefix(&db, &prefix)
        .args(["pull"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "+0 entries, ~0 updates, -0 deletes",
        ));
}
