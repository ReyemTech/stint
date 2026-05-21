use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn logout_with_no_oauth_blob_completes_cleanly() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    // STINT_SECRET_PREFIX routes any keychain touch to a synthetic test
    // prefix so the test doesn't fail in macOS dark wake (when the real
    // keychain refuses UI prompts). Mirrors the pattern in cli_e2e.rs.
    let prefix = format!(
        "tech.reyem.stint.test.{}",
        stint_core::ids::new_local_uuid()
    );

    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", &db)
        .env("STINT_SECRET_PREFIX", &prefix)
        .args(["config", "logout"])
        .assert()
        .success()
        .stdout(contains("OAuth tokens cleared"));
}
