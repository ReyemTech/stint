//! Tests for the focus-default fallback in verbs::start.
//!
//! The fallback reads `focus.default_project` (a "<focus_id>\t<project_id>"
//! tuple written by Swift's ProjectFocusFilter) and applies it ONLY when
//! the stored focus_id matches the currently-active focus, so stale
//! defaults from previous focus modes don't leak.

mod common;

use stint_core::{config::Settings, verbs};

struct FocusGuard;
impl FocusGuard {
    fn set(value: &str) {
        std::env::set_var("STINT_TEST_FOCUS_ID", value);
    }
    fn clear() {
        std::env::remove_var("STINT_TEST_FOCUS_ID");
    }
}
impl Drop for FocusGuard {
    fn drop(&mut self) {
        Self::clear();
    }
}

fn start_params(desc: &str, project_id: Option<&str>) -> verbs::StartParams {
    verbs::StartParams {
        description: desc.into(),
        project_id: project_id.map(str::to_string),
        task_id: None,
        billable: false,
        start_at: None,
        source: "focus-test".into(),
    }
}

#[tokio::test]
async fn start_picks_up_focus_default_when_project_missing() {
    let env = common::setup().await;
    common::seed_projects(&env.store, &[("proj-uuid-1", "Acme")]).await;

    Settings::new(env.store.clone())
        .set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .await
        .unwrap();

    FocusGuard::set("fake-focus-id");
    let _guard = FocusGuard;

    let view = verbs::start(&env.store, start_params("no project given", None))
        .await
        .unwrap();

    assert_eq!(view.project_id.as_deref(), Some("proj-uuid-1"));
}

#[tokio::test]
async fn start_ignores_focus_default_when_focus_id_mismatches() {
    let env = common::setup().await;
    common::seed_projects(&env.store, &[("proj-uuid-1", "Acme")]).await;

    Settings::new(env.store.clone())
        .set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .await
        .unwrap();

    FocusGuard::set("different-focus-id");
    let _guard = FocusGuard;

    let view = verbs::start(&env.store, start_params("no project given", None))
        .await
        .unwrap();

    assert_eq!(view.project_id, None);
}

#[tokio::test]
async fn start_explicit_project_overrides_focus_default() {
    let env = common::setup().await;
    common::seed_projects(
        &env.store,
        &[("proj-uuid-1", "Acme"), ("proj-uuid-2", "Other")],
    )
    .await;

    Settings::new(env.store.clone())
        .set("focus.default_project", "fake-focus-id\tproj-uuid-1")
        .await
        .unwrap();

    FocusGuard::set("fake-focus-id");
    let _guard = FocusGuard;

    let view = verbs::start(
        &env.store,
        start_params("explicit project", Some("proj-uuid-2")),
    )
    .await
    .unwrap();

    assert_eq!(view.project_id.as_deref(), Some("proj-uuid-2"));
}

#[tokio::test]
async fn start_no_focus_default_no_project_applied() {
    let env = common::setup().await;
    // No focus.default_project key set.

    // No STINT_TEST_FOCUS_ID set either.
    FocusGuard::clear();

    let view = verbs::start(&env.store, start_params("vanilla", None))
        .await
        .unwrap();

    assert_eq!(view.project_id, None);
}

#[tokio::test]
async fn start_focus_default_with_no_active_focus_is_ignored() {
    let env = common::setup().await;
    common::seed_projects(&env.store, &[("proj-uuid-1", "Acme")]).await;

    Settings::new(env.store.clone())
        .set("focus.default_project", "stored-focus\tproj-uuid-1")
        .await
        .unwrap();

    // No active focus.
    FocusGuard::clear();

    let view = verbs::start(&env.store, start_params("v", None))
        .await
        .unwrap();

    assert_eq!(view.project_id, None);
}

#[tokio::test]
async fn start_focus_default_with_malformed_tuple_is_ignored() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("focus.default_project", "no-tab-separator-here")
        .await
        .unwrap();

    FocusGuard::set("any-focus");
    let _guard = FocusGuard;

    let view = verbs::start(&env.store, start_params("v", None))
        .await
        .unwrap();

    assert_eq!(view.project_id, None);
}
