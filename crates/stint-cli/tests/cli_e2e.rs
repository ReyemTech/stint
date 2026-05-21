use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::config::secrets::Secrets;
use stint_core::store::{entries::Entries, Store};
use std::sync::{LazyLock, Mutex};
use tempfile::TempDir;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

static KEYCHAIN_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct SecretRestoreGuard {
    secrets: Secrets,
    key: &'static str,
    prior: Option<String>,
}

impl SecretRestoreGuard {
    fn capture(key: &'static str) -> Self {
        let secrets = Secrets::default();
        let prior = secrets.get(key).expect("read prior secret");
        Self { secrets, key, prior }
    }
}

impl Drop for SecretRestoreGuard {
    fn drop(&mut self) {
        match &self.prior {
            Some(value) => self
                .secrets
                .set(self.key, value)
                .expect("restore prior secret value"),
            None => self
                .secrets
                .delete(self.key)
                .expect("delete test secret value"),
        }
    }
}

async fn first_entry_id(db: &std::path::Path) -> String {
    let store = Store::connect(db).await.expect("connect temp store");
    let entries = Entries::new(store);
    let [from, to] = wide_range();
    let rows = entries
        .list_between(from, to)
        .await
        .expect("list entries in wide range");
    rows.first()
        .expect("entry row present")
        .local_uuid
        .clone()
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

    cmd(&db)
        .args(["config", "set", "solidtime.url", "https://time.example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved solidtime.url."));
    cmd(&db)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved solidtime.org."));

    cmd(&db)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("solidtime.url = https://time.example.com"))
        .stdout(predicate::str::contains("solidtime.org = org-1"));
}

#[test]
fn config_set_requires_value_for_non_secret_keys() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["config", "set", "solidtime.url"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "value required for solidtime.url",
        ));
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
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let _lock = KEYCHAIN_LOCK.lock().unwrap();
    let _restore = SecretRestoreGuard::capture("solidtime.token");

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

    cmd(&db)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd(&db)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Saved solidtime.token to Keychain.",
        ));

    cmd(&db)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains(&format!(
            "solidtime.url = {}",
            server.uri()
        )))
        .stdout(predicate::str::contains("solidtime.token = ••••"));

    cmd(&db)
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
        .stdout(predicate::str::contains(&format!(
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

    cmd(&db)
        .args(["start", "delete me"])
        .assert()
        .success();
    cmd(&db).args(["stop"]).assert().success();

    let entry_id = first_entry_id(&db).await;

    cmd(&db)
        .args(["delete", &entry_id])
        .assert()
        .success()
        .stdout(predicate::str::contains(&format!("Deleted {entry_id}.")));

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
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }

    let _lock = KEYCHAIN_LOCK.lock().unwrap();
    let _restore = SecretRestoreGuard::capture("solidtime.token");

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

    cmd(&db)
        .args(["config", "set", "solidtime.url", &server.uri()])
        .assert()
        .success();
    cmd(&db)
        .args(["config", "set", "solidtime.org", "org-1"])
        .assert()
        .success();
    cmd(&db)
        .args(["config", "set", "solidtime.token", "test-token"])
        .assert()
        .success();

    cmd(&db)
        .args(["projects", "refresh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("refreshed"));

    cmd(&db)
        .args(["projects", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("p1  Tet"));
}
