mod common;

use assert_cmd::Command;
use serde_json::Value;

#[test]
fn start_json_emits_entry_view_shape() {
    let env = common::TestEnv::new();
    let out = Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", env.db_path())
        .args(["--json", "start", "hello"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["description"], "hello");
    assert_eq!(v["source"], "cli");
    assert!(v["local_uuid"].is_string());
    assert!(v["end_at"].is_null());
}

#[test]
fn list_json_emits_array() {
    let env = common::TestEnv::new();
    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", env.db_path())
        .args(["start", "a"])
        .assert()
        .success();
    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", env.db_path())
        .args(["stop"])
        .assert()
        .success();

    let out = Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", env.db_path())
        .args([
            "--json",
            "list",
            "2000-01-01T00:00:00Z",
            "2100-01-01T00:00:00Z",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v.is_array());
    assert_eq!(v.as_array().unwrap().len(), 1);
    assert_eq!(v[0]["description"], "a");
}
