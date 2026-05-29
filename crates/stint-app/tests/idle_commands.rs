//! Integration test for idle_discard / idle_split. Exercises the verb
//! layer the way the Tauri commands would — same store + arguments.

mod common;

use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;

#[tokio::test]
async fn idle_discard_stops_entry_at_idle_started() {
    let ctx = common::make_app().await;

    let start_at = "2026-05-27T10:00:00Z";
    let view = stint_core::verbs::start(
        &ctx.store,
        stint_core::verbs::StartParams {
            description: "deep work".into(),
            project_id: None,
            task_id: None,
            billable: false,
            start_at: Some(start_at.into()),
            source: "test".into(),
        },
    )
    .await
    .unwrap();

    let idle_started = "2026-05-27T10:18:00Z";

    stint_app::commands::idle::discard_impl(&ctx.store, idle_started)
        .await
        .unwrap();

    let row = Entries::new((*ctx.store).clone())
        .get(&view.local_uuid)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.end_at.as_deref(), Some(idle_started));

    let running = RunningTimer::new((*ctx.store).clone()).get().await.unwrap();
    assert!(running.is_none());
}

#[tokio::test]
async fn idle_discard_errors_when_no_running_timer() {
    let ctx = common::make_app().await;
    let result = stint_app::commands::idle::discard_impl(&ctx.store, "2026-05-27T10:00:00Z").await;
    assert!(matches!(result, Err(stint_core::Error::Invariant(_))));
}
