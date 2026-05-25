//! Miscellaneous CLI surface tests: `stint current`, `stint api info`,
//! `stint generate-man`. These commands are thin wrappers over Settings /
//! verbs::current / clap_mangen but each carries a JSON + human render
//! pair, so we exercise both arms.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

// ---- current ----------------------------------------------------------

#[test]
fn current_human_reports_no_timer_running_on_fresh_store() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db)
        .args(["current"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No timer running."));
}

#[test]
fn current_json_emits_null_on_fresh_store() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let out = cmd(&db)
        .args(["--json", "current"])
        .output()
        .expect("run current --json");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert!(v.is_null(), "expected null when idle, got {v}");
}

#[test]
fn current_human_reports_running_entry_after_start() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db).args(["start", "current-test"]).assert().success();
    cmd(&db)
        .args(["current"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Running: current-test"));
}

#[test]
fn current_json_emits_entry_view_after_start() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db).args(["start", "json-current"]).assert().success();
    let out = cmd(&db)
        .args(["--json", "current"])
        .output()
        .expect("run current --json");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["description"], "json-current");
    assert!(v["end_at"].is_null());
}

// ---- api info ---------------------------------------------------------

#[test]
fn api_info_human_reports_disabled_on_fresh_store() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    cmd(&db)
        .args(["api", "info"])
        .assert()
        .success()
        .stdout(predicate::str::contains("enabled: false"))
        .stdout(predicate::str::contains("host:    127.0.0.1"))
        .stdout(predicate::str::contains("port:    -"))
        .stdout(predicate::str::contains("url:     -"));
}

#[test]
fn api_info_json_emits_disabled_shape_on_fresh_store() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let out = cmd(&db)
        .args(["--json", "api", "info"])
        .output()
        .expect("run api info --json");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["enabled"], false);
    assert_eq!(v["host"], "127.0.0.1");
    assert!(v["port"].is_null());
    assert!(v["base_url"].is_null());
}

#[test]
fn api_info_reflects_persisted_enabled_and_port() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    // `config set api.enabled true` / `config set api.port 47921` —
    // generic non-secret keys go through the same Settings table.
    cmd(&db)
        .args(["config", "set", "api.enabled", "true"])
        .assert()
        .success();
    cmd(&db)
        .args(["config", "set", "api.port", "47921"])
        .assert()
        .success();

    let out = cmd(&db)
        .args(["--json", "api", "info"])
        .output()
        .expect("run api info --json");
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["enabled"], true);
    assert_eq!(v["port"], 47921);
    assert_eq!(v["base_url"], "http://127.0.0.1:47921");
}

// ---- generate-man -----------------------------------------------------

#[test]
fn generate_man_writes_stint_1_into_target_dir() {
    let tmp = TempDir::new().unwrap();
    let out_dir = tmp.path().join("man1");
    // generate-man doesn't open the store, but the binary still wants a
    // STINT_DB env so other ambient commands don't blow up. It's safe to
    // point at a non-existent file — the command never touches it.
    let mut c = Command::cargo_bin("stint").unwrap();
    c.env("STINT_DB", tmp.path().join("unused.db"));
    let out = c
        .args(["generate-man", out_dir.to_str().unwrap()])
        .output()
        .expect("run generate-man");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let man_path = out_dir.join("stint.1");
    assert!(man_path.exists(), "stint.1 should be written");
    let body = std::fs::read_to_string(&man_path).expect("read man page");
    let lower = body.to_lowercase();
    assert!(
        lower.contains("stint"),
        "man page should reference stint, got first 200 chars: {:?}",
        &body[..body.len().min(200)]
    );
    // The clap_mangen output also includes the binary name in a `.TH` macro
    // and the description string we set on Cli; sanity-check we got both.
    assert!(
        body.contains("Time tracker") || body.contains("time tracker"),
        "man page should include the description"
    );
}
