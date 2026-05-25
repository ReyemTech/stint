use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn stint_update_check_prints_current_version() {
    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_UPDATE_SKIP_NETWORK", "1")
        .args(["update", "--check"])
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn stint_update_help_explains_flags() {
    Command::cargo_bin("stint")
        .unwrap()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(contains("--check"))
        .stdout(contains("--force"));
}

#[test]
fn stint_update_check_json_emits_skipped_ack_when_offline() {
    let out = Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_UPDATE_SKIP_NETWORK", "1")
        .args(["--json", "update", "--check"])
        .output()
        .expect("run update --check --json");
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert_eq!(v["checked"], false);
    assert_eq!(v["skipped"], "network");
    assert_eq!(v["current"], env!("CARGO_PKG_VERSION"));
}
