mod common;

use stint_core::recovery::{recover_on_startup, RecoveryDecision, RecoveryOutcome};
use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::timer::{StartArgs, TimerService};

#[tokio::test]
async fn no_running_timer_returns_nothing_to_do() {
    let env = common::setup().await;
    let outcome = recover_on_startup(&env.store, |_| RecoveryDecision::Discard)
        .await
        .unwrap();
    assert!(matches!(outcome, RecoveryOutcome::NothingToDo));
}

#[tokio::test]
async fn fresh_heartbeat_returns_attach_in_place() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();

    let outcome = recover_on_startup(&env.store, |_| RecoveryDecision::Discard)
        .await
        .unwrap();
    assert!(matches!(outcome, RecoveryOutcome::AttachInPlace { .. }));
}

#[tokio::test]
async fn very_stale_heartbeat_prompts_decision_and_keep_continues_timer() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();

    sqlx::query("UPDATE running_timer SET heartbeat_at = ? WHERE id = 1")
        .bind("2020-01-01T00:00:00Z")
        .execute(env.store.pool())
        .await
        .unwrap();

    let outcome = recover_on_startup(&env.store, |_| RecoveryDecision::KeepRunning)
        .await
        .unwrap();
    assert!(matches!(outcome, RecoveryOutcome::Recovered { .. }));

    let r = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.local_uuid, id);
    assert_ne!(r.heartbeat_at, "2020-01-01T00:00:00Z");

    let row = Entries::new(env.store.clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.end_at.is_none());
}

#[tokio::test]
async fn very_stale_heartbeat_stop_at_last_heartbeat_sets_end_and_clears() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();
    sqlx::query("UPDATE running_timer SET heartbeat_at = ? WHERE id = 1")
        .bind("2020-01-01T00:00:00Z")
        .execute(env.store.pool())
        .await
        .unwrap();

    recover_on_startup(&env.store, |_| RecoveryDecision::StopAtLastHeartbeat)
        .await
        .unwrap();

    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());
    let row = Entries::new(env.store.clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.end_at.as_deref(), Some("2020-01-01T00:00:00Z"));
}

#[tokio::test]
async fn moderately_stale_heartbeat_auto_recovers_without_prompting() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();

    // 2 minutes ago: falls in the (60s, 600s] auto-recover band — no prompt.
    let stale_hb = (chrono::Utc::now() - chrono::Duration::seconds(120))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("UPDATE running_timer SET heartbeat_at = ? WHERE id = 1")
        .bind(&stale_hb)
        .execute(env.store.pool())
        .await
        .unwrap();

    let prompt_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pc = prompt_called.clone();
    let outcome = recover_on_startup(&env.store, move |_| {
        pc.store(true, std::sync::atomic::Ordering::SeqCst);
        RecoveryDecision::Discard
    })
    .await
    .unwrap();

    assert!(matches!(outcome, RecoveryOutcome::Recovered { .. }));
    assert!(
        !prompt_called.load(std::sync::atomic::Ordering::SeqCst),
        "prompt should not be invoked when heartbeat is auto-recoverable",
    );

    // Heartbeat was refreshed; entry stays running (no end_at).
    let r = RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.local_uuid, id);
    assert_ne!(r.heartbeat_at, stale_hb);
    let row = Entries::new(env.store.clone())
        .get(&id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.end_at.is_none());
}

#[tokio::test]
async fn very_stale_heartbeat_discard_deletes_entry_and_clears_running() {
    let env = common::setup().await;
    let timer = TimerService::new(env.store.clone());
    let id = timer
        .start(StartArgs {
            description: "x".into(),
            project_id: None,
            task_id: None,
            billable: false,
            source: "cli".into(),
            start_at: None,
        })
        .await
        .unwrap();
    sqlx::query("UPDATE running_timer SET heartbeat_at = ? WHERE id = 1")
        .bind("2020-01-01T00:00:00Z")
        .execute(env.store.pool())
        .await
        .unwrap();

    let outcome = recover_on_startup(&env.store, |_| RecoveryDecision::Discard)
        .await
        .unwrap();
    assert!(matches!(outcome, RecoveryOutcome::Discarded { .. }));

    assert!(RunningTimer::new(env.store.clone())
        .get()
        .await
        .unwrap()
        .is_none());
    // The entry was in pending_create (just started), so Entries::delete
    // hard-removes the row. Confirming the row is gone exercises the
    // Discarded → Entries::delete → pending_create-branch path.
    let row = Entries::new(env.store.clone()).get(&id).await.unwrap();
    assert!(row.is_none(), "discarded entry should be hard-deleted");
}
