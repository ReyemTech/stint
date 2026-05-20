use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_empty_returns_no_accounts() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["calendar", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No calendar accounts"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_help_lists_subcommands() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["calendar", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("calendars"))
        .stdout(predicate::str::contains("refresh"));
}
