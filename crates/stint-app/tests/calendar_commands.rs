//! Integration tests for `commands/calendar.rs`.
//!
//! Skipped (exempt):
//!   - `calendar_add_google`: launches the system browser for OAuth.
//!   - `calendar_refresh_account` with a real account: requires an OAuth
//!     blob in the Keychain; only the no-blob error path is tested.

mod common;

use stint_app::commands::calendar::{
    calendar_ignore_event, calendar_list_accounts, calendar_list_calendars,
    calendar_list_events_in_range, calendar_log_event, calendar_oauth_status,
    calendar_refresh_account, calendar_remove_account, calendar_set_calendar_included,
    calendar_set_default_project,
};
use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{
    AttendeeStatus, Calendar, CalendarAccount, CalendarEvent, ProviderKind,
};
use stint_core::store::entries::Entries;
use stint_core::store::reference::{ProjectRow, Reference};
use stint_core::time;
use tauri::Manager;

async fn seed_account(store: &std::sync::Arc<stint_core::store::Store>, id: &str) {
    let cs = CalendarStore::new((**store).clone());
    cs.add_account(&CalendarAccount {
        id: id.into(),
        provider: ProviderKind::Google,
        display_name: "Me".into(),
        identifier: "me@example.com".into(),
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    })
    .await
    .unwrap();
}

async fn seed_calendars(
    store: &std::sync::Arc<stint_core::store::Store>,
    account_id: &str,
    cals: &[(&str, &str)],
) {
    let cs = CalendarStore::new((**store).clone());
    let rows: Vec<Calendar> = cals
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
    cs.upsert_calendars(account_id, &rows).await.unwrap();
}

async fn seed_event(
    store: &std::sync::Arc<stint_core::store::Store>,
    account_id: &str,
    calendar_id: &str,
    event_id: &str,
    start_at: &str,
    end_at: &str,
    title: &str,
) {
    let cs = CalendarStore::new((**store).clone());
    cs.upsert_events(&[CalendarEvent {
        id: event_id.into(),
        account_id: account_id.into(),
        calendar_id: calendar_id.into(),
        title: title.into(),
        start_at: start_at.into(),
        end_at: end_at.into(),
        is_all_day: false,
        attendee_status: Some(AttendeeStatus::Accepted),
        recurring_root: None,
        fetched_at: time::now_utc(),
    }])
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_accounts_returns_empty_then_seeded() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();

    let accounts = calendar_list_accounts(handle.state()).await.unwrap();
    assert!(accounts.is_empty());

    seed_account(&ctx.store, "acc-1").await;

    let accounts = calendar_list_accounts(handle.state()).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "acc-1");
    assert_eq!(accounts[0].identifier, "me@example.com");
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_oauth_status_is_signed_out_when_blob_missing() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;

    let status = calendar_oauth_status("acc-1".into()).await.unwrap();
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["signed_in"], false);
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_calendars_returns_seeded_rows() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(
        &ctx.store,
        "acc-1",
        &[("cal-1", "Personal"), ("cal-2", "Work")],
    )
    .await;

    let handle = ctx.handle();
    let cals = calendar_list_calendars(handle.state(), "acc-1".into())
        .await
        .unwrap();
    assert_eq!(cals.len(), 2);
    assert!(cals.iter().all(|c| c.included));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_set_calendar_included_toggles_persistence() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;

    let handle = ctx.handle();
    calendar_set_calendar_included(handle.state(), handle.clone(), "cal-1".into(), false)
        .await
        .unwrap();

    let cals = calendar_list_calendars(handle.state(), "acc-1".into())
        .await
        .unwrap();
    assert!(!cals[0].included);

    calendar_set_calendar_included(handle.state(), handle.clone(), "cal-1".into(), true)
        .await
        .unwrap();
    let cals = calendar_list_calendars(handle.state(), "acc-1".into())
        .await
        .unwrap();
    assert!(cals[0].included);
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_list_events_in_range_returns_events_within_window() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;
    seed_event(
        &ctx.store,
        "acc-1",
        "cal-1",
        "evt-1",
        "2026-05-20T09:00:00Z",
        "2026-05-20T09:30:00Z",
        "Standup",
    )
    .await;

    let handle = ctx.handle();
    let events = calendar_list_events_in_range(
        handle.state(),
        "acc-1".into(),
        "2026-05-20T00:00:00Z".into(),
        "2026-05-21T00:00:00Z".into(),
    )
    .await
    .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.title, "Standup");
    assert!(events[0].decision.is_none(), "no decision recorded yet");
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_log_event_creates_time_entry_and_marks_logged() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;
    seed_event(
        &ctx.store,
        "acc-1",
        "cal-1",
        "evt-1",
        "2026-05-20T09:00:00Z",
        "2026-05-20T09:30:00Z",
        "Standup",
    )
    .await;

    let handle = ctx.handle();
    let local_uuid = calendar_log_event(
        handle.state(),
        handle.clone(),
        "acc-1".into(),
        "evt-1".into(),
        "2026-05-20T09:00:00Z".into(),
    )
    .await
    .unwrap();
    assert!(!local_uuid.is_empty());

    // Entry persisted.
    let row = Entries::new((*ctx.store).clone())
        .get(&local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.description, "Standup");
    assert_eq!(row.source, "calendar");

    // Subsequent list shows the decision.
    let events = calendar_list_events_in_range(
        handle.state(),
        "acc-1".into(),
        "2026-05-20T00:00:00Z".into(),
        "2026-05-21T00:00:00Z".into(),
    )
    .await
    .unwrap();
    assert_eq!(events[0].decision.as_deref(), Some("logged_manual"));
    assert_eq!(
        events[0].linked_local_uuid.as_deref(),
        Some(local_uuid.as_str())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_set_default_project_round_trips_and_logs_prefill() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;
    seed_event(
        &ctx.store,
        "acc-1",
        "cal-1",
        "evt-1",
        "2026-05-20T09:00:00Z",
        "2026-05-20T09:30:00Z",
        "Standup",
    )
    .await;

    let handle = ctx.handle();
    calendar_set_default_project(
        handle.state(),
        handle.clone(),
        "cal-1".into(),
        Some("p-42".into()),
    )
    .await
    .unwrap();
    let cals = calendar_list_calendars(handle.state(), "acc-1".into())
        .await
        .unwrap();
    assert_eq!(cals[0].default_project_id.as_deref(), Some("p-42"));

    // Logging the event now prefills project_id from the calendar default.
    let local_uuid = calendar_log_event(
        handle.state(),
        handle.clone(),
        "acc-1".into(),
        "evt-1".into(),
        "2026-05-20T09:00:00Z".into(),
    )
    .await
    .unwrap();
    let row = Entries::new((*ctx.store).clone())
        .get(&local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id.as_deref(), Some("p-42"));

    // Clearing the default removes it from the calendar (no retroactive effect
    // on the already-logged entry).
    calendar_set_default_project(handle.state(), handle.clone(), "cal-1".into(), None)
        .await
        .unwrap();
    let cals = calendar_list_calendars(handle.state(), "acc-1".into())
        .await
        .unwrap();
    assert_eq!(cals[0].default_project_id, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_log_event_inherits_default_project_billable() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;

    // Reference table caches Solidtime project is_billable as billable_default.
    Reference::new((*ctx.store).clone())
        .upsert_projects(&[ProjectRow {
            id: "p-bill".into(),
            name: "Billed".into(),
            color: None,
            client_id: None,
            client_name: None,
            archived: 0,
            billable_default: 1,
        }])
        .await
        .unwrap();

    seed_event(
        &ctx.store,
        "acc-1",
        "cal-1",
        "evt-1",
        "2026-05-20T09:00:00Z",
        "2026-05-20T09:30:00Z",
        "Standup",
    )
    .await;

    let handle = ctx.handle();
    calendar_set_default_project(
        handle.state(),
        handle.clone(),
        "cal-1".into(),
        Some("p-bill".into()),
    )
    .await
    .unwrap();

    let local_uuid = calendar_log_event(
        handle.state(),
        handle.clone(),
        "acc-1".into(),
        "evt-1".into(),
        "2026-05-20T09:00:00Z".into(),
    )
    .await
    .unwrap();

    let row = Entries::new((*ctx.store).clone())
        .get(&local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.project_id.as_deref(), Some("p-bill"));
    assert_eq!(row.billable, 1, "billable_default=1 should flow through");
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_ignore_event_records_ignored_decision() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;
    seed_event(
        &ctx.store,
        "acc-1",
        "cal-1",
        "evt-2",
        "2026-05-20T11:00:00Z",
        "2026-05-20T11:15:00Z",
        "Other",
    )
    .await;

    let handle = ctx.handle();
    calendar_ignore_event(
        handle.state(),
        handle.clone(),
        "acc-1".into(),
        "evt-2".into(),
        "2026-05-20T11:00:00Z".into(),
    )
    .await
    .unwrap();

    let events = calendar_list_events_in_range(
        handle.state(),
        "acc-1".into(),
        "2026-05-20T00:00:00Z".into(),
        "2026-05-21T00:00:00Z".into(),
    )
    .await
    .unwrap();
    assert_eq!(events[0].decision.as_deref(), Some("ignored"));
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_remove_account_deletes_account_and_calendars() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-1").await;
    seed_calendars(&ctx.store, "acc-1", &[("cal-1", "Personal")]).await;

    let handle = ctx.handle();
    calendar_remove_account(handle.state(), handle.clone(), "acc-1".into())
        .await
        .unwrap();

    let accounts = calendar_list_accounts(handle.state()).await.unwrap();
    assert!(accounts.is_empty(), "account row deleted");
    // calendars cascade-delete via FK; confirm via store-level query.
    let cs = CalendarStore::new((*ctx.store).clone());
    let cals = cs.list_calendars("acc-1").await.unwrap();
    assert!(cals.is_empty(), "calendars cascade-deleted");
}

#[tokio::test(flavor = "multi_thread")]
async fn calendar_refresh_account_errors_when_no_oauth_blob_present() {
    let ctx = common::make_app().await;
    seed_account(&ctx.store, "acc-no-blob").await;

    let handle = ctx.handle();
    let err = calendar_refresh_account(handle.state(), handle.clone(), "acc-no-blob".into())
        .await
        .unwrap_err();
    // The exact error wording comes from stint-core; just confirm we got
    // an error (not Ok). build_provider_from_blob bubbles up as keyring/
    // OAuth-style error.
    assert!(
        !err.message.is_empty(),
        "expected non-empty error message; got: {:?}",
        err
    );
}
