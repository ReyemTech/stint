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
