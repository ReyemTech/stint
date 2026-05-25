use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn login_without_solidtime_url_returns_clear_error() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = format!(
        "tech.reyem.stint.test.{}",
        stint_core::ids::new_local_uuid()
    );

    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", &db)
        .env("STINT_SECRET_PREFIX", &prefix)
        .args(["config", "login"])
        .assert()
        .failure()
        .stderr(contains("solidtime.url is not set"));
}

#[test]
fn login_without_client_id_explains_how_to_set_it() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = format!(
        "tech.reyem.stint.test.{}",
        stint_core::ids::new_local_uuid()
    );

    // Set URL but not client_id.
    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", &db)
        .env("STINT_SECRET_PREFIX", &prefix)
        .args(["config", "set", "solidtime.url", "https://example.com"])
        .assert()
        .success();

    Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DB", &db)
        .env("STINT_SECRET_PREFIX", &prefix)
        .args(["config", "login"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("solidtime.oauth.client_id is not set")
                .or(predicate::str::contains("missing OAuth client ID")),
        );
}

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
