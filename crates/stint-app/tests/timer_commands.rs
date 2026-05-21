//! Proof-of-life integration test for the Task 13 mock-app harness.
//!
//! Subsequent tests (Task 14) flesh out the timer command coverage. This
//! file currently only verifies the harness compiles, the mock app
//! constructs, state is managed correctly, and a no-argument command
//! returns the expected empty result on a fresh database.

mod common;

use stint_app::commands::timer::get_running_timer;
use tauri::Manager;

#[tokio::test(flavor = "multi_thread")]
async fn get_running_timer_returns_none_on_fresh_store() {
    let ctx = common::make_app().await;
    let handle = ctx.handle();
    let state = handle.state();

    let result = get_running_timer(state).await.expect("command succeeds");
    assert!(result.is_none(), "no timer should be running on a fresh DB");
}
