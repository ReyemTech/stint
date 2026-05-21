use assert_cmd::Command;
use predicates::prelude::*;
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{Calendar, CalendarAccount, ProviderKind};
use stint_core::store::Store;
use stint_core::time;
use tempfile::TempDir;

fn cmd(db: &std::path::Path) -> Command {
    let mut c = Command::cargo_bin("stint").expect("binary built");
    c.env("STINT_DB", db);
    c
}

/// Threads STINT_SECRET_PREFIX through the binary so any incidental
/// keychain touch (e.g. `calendar remove` deleting a non-existent blob)
/// hits a synthetic test prefix instead of the developer's real entries.
fn cmd_with_prefix(db: &std::path::Path, prefix: &str) -> Command {
    let mut c = cmd(db);
    c.env("STINT_SECRET_PREFIX", prefix);
    c
}

fn unique_test_prefix() -> String {
    format!(
        "tech.reyem.stint.test.{}",
        stint_core::ids::new_local_uuid()
    )
}

async fn seed_account_with_calendars(
    db: &std::path::Path,
    account_id: &str,
    calendars: &[(&str, &str)],
) {
    let store = Store::connect(db).await.unwrap();
    let cs = CalendarStore::new(store);
    cs.add_account(&CalendarAccount {
        id: account_id.into(),
        provider: ProviderKind::Google,
        display_name: "Me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    })
    .await
    .unwrap();
    let cals: Vec<Calendar> = calendars
        .iter()
        .map(|(id, name)| Calendar {
            id: (*id).into(),
            account_id: account_id.into(),
            name: (*name).into(),
            color: None,
            included: true,
            default_project_id: None,
        })
        .collect();
    cs.upsert_calendars(account_id, &cals).await.unwrap();
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

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_prints_seeded_accounts() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    seed_account_with_calendars(&db, "acc-1", &[("cal-1", "Personal")]).await;

    cmd(&db)
        .args(["calendar", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("acc-1"))
        .stdout(predicate::str::contains("google"))
        .stdout(predicate::str::contains("me@example.com"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_calendars_toggles_include_and_exclude() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    seed_account_with_calendars(
        &db,
        "acc-1",
        &[("cal-a", "Calendar A"), ("cal-b", "Calendar B")],
    )
    .await;

    cmd(&db)
        .args(["calendar", "calendars", "acc-1", "--exclude", "cal-b"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Excluded calendar cal-b"))
        .stdout(predicate::str::contains("[ ] cal-b Calendar B"))
        .stdout(predicate::str::contains("[x] cal-a Calendar A"));

    cmd(&db)
        .args(["calendar", "calendars", "acc-1", "--include", "cal-b"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Included calendar cal-b"))
        .stdout(predicate::str::contains("[x] cal-b Calendar B"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_calendars_set_and_clear_default_project_round_trip() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    seed_account_with_calendars(&db, "acc-1", &[("cal-1", "Personal")]).await;

    cmd(&db)
        .args([
            "calendar",
            "calendars",
            "acc-1",
            "--set-default-project",
            "cal-1",
            "p-42",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Set default project p-42 on calendar cal-1",
        ))
        .stdout(predicate::str::contains("(default: p-42)"));

    // Re-open via core to confirm persistence beyond the listing.
    let cs = CalendarStore::new(Store::connect(&db).await.unwrap());
    let cals = cs.list_calendars("acc-1").await.unwrap();
    assert_eq!(cals[0].default_project_id.as_deref(), Some("p-42"));

    cmd(&db)
        .args([
            "calendar",
            "calendars",
            "acc-1",
            "--clear-default-project",
            "cal-1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleared default project on calendar cal-1",
        ));

    let cs = CalendarStore::new(Store::connect(&db).await.unwrap());
    let cals = cs.list_calendars("acc-1").await.unwrap();
    assert_eq!(cals[0].default_project_id, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_add_rejects_unknown_provider() {
    // clap's value_parser is the gate — the inner `unknown provider` arm
    // is only reachable via direct API. Verify the gate.
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");

    cmd(&db)
        .args(["calendar", "add", "outlook"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'outlook'"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_refresh_errors_without_oauth_blob() {
    // Account exists in the DB but no Keychain blob — `build_provider_from_blob`
    // should fail with a clear error rather than a panic.
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = unique_test_prefix();
    seed_account_with_calendars(&db, "acc-1", &[("cal-1", "Personal")]).await;

    cmd_with_prefix(&db, &prefix)
        .args(["calendar", "refresh", "acc-1"])
        .assert()
        .failure();
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_remove_deletes_account_row() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("stint.db");
    let prefix = unique_test_prefix();
    seed_account_with_calendars(&db, "acc-1", &[("cal-1", "Personal")]).await;

    cmd_with_prefix(&db, &prefix)
        .args(["calendar", "remove", "acc-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed account acc-1"));

    // Account gone — list reverts to the empty path.
    cmd_with_prefix(&db, &prefix)
        .args(["calendar", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No calendar accounts"));
}
