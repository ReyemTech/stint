//! Integration tests for `commands/entries.rs`.

mod common;

use stint_app::commands::entries::{list_between, list_today};
use stint_core::store::entries::{Entries, NewCompletedEntry, NewTimeEntry};
use tauri::Manager;

#[tokio::test(flavor = "multi_thread")]
async fn list_today_returns_empty_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let rows = list_today(handle.state()).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn list_between_returns_only_rows_in_range() {
    let ctx = common::make_app().await;
    let entries = Entries::new((*ctx.store).clone());

    // One entry inside the query range, one outside.
    let inside = entries
        .create(NewTimeEntry {
            description: "in".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-05-20T09:00:00Z".into(),
            billable: false,
            source: "cli".into(),
        })
        .await
        .unwrap();
    let _outside = entries
        .create_completed(NewCompletedEntry {
            description: "out".into(),
            project_id: None,
            task_id: None,
            start_at: "2026-04-01T09:00:00Z".into(),
            end_at: "2026-04-01T09:15:00Z".into(),
            billable: false,
            source: "calendar".into(),
            source_event_id: None,
        })
        .await
        .unwrap();

    let handle = ctx.handle();
    let rows = list_between(
        handle.state(),
        "2026-05-20T00:00:00Z".into(),
        "2026-05-21T00:00:00Z".into(),
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].local_uuid, inside);
    assert_eq!(rows[0].description, "in");
}

#[tokio::test(flavor = "multi_thread")]
async fn list_today_maps_billable_int_to_bool() {
    let ctx = common::make_app().await;
    let entries = Entries::new((*ctx.store).clone());

    // Seed a completed billable entry whose start time is today (now-ish UTC),
    // so list_today (which uses Local TZ → UTC) picks it up.
    let now = chrono::Utc::now();
    let start = (now - chrono::Duration::minutes(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let end = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    entries
        .create_completed(NewCompletedEntry {
            description: "billable work".into(),
            project_id: None,
            task_id: None,
            start_at: start,
            end_at: end,
            billable: true,
            source: "cli".into(),
            source_event_id: None,
        })
        .await
        .unwrap();

    let handle = ctx.handle();
    let rows = list_today(handle.state()).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].billable, "billable bool should be true");
}
