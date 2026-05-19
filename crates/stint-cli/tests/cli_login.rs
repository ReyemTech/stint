use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn logout_with_no_oauth_blob_completes_cleanly() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", &db)
        .args(["config", "logout"])
        .assert()
        .success()
        .stdout(contains("OAuth tokens cleared"));
}
