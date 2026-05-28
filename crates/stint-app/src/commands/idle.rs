//! Tauri commands backing the IdleBanner.tsx buttons. The user gets:
//!   Keep    — banner dismisses; entry untouched.
//!   Discard — end the entry at idle_started; subtract the idle period.
//!   Split   — same storage behavior as Discard; UI distinguishes by
//!             pre-filling the start form for one-click resume.

use stint_core::store::entries::Entries;
use stint_core::store::running::RunningTimer;
use stint_core::store::Store;
use stint_core::{Error, Result};
use tauri::State;
use tokio::sync::RwLock;

/// Pure backend helper — exposed so tests can exercise without going through
/// Tauri's runtime.
pub async fn discard_impl(store: &Store, idle_started: &str) -> Result<()> {
    let running = RunningTimer::new(store.clone())
        .get()
        .await?
        .ok_or_else(|| Error::Invariant("no running timer".into()))?;
    let entries = Entries::new(store.clone());
    entries.set_end(&running.local_uuid, idle_started).await?;
    RunningTimer::new(store.clone()).clear().await?;
    Ok(())
}

#[tauri::command]
pub async fn idle_keep() -> std::result::Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn idle_discard(
    idle_started: String,
    state: State<'_, RwLock<crate::app_state::AppState>>,
) -> std::result::Result<(), String> {
    let store = state.read().await.store.clone();
    discard_impl(&store, &idle_started)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn idle_split(
    idle_started: String,
    state: State<'_, RwLock<crate::app_state::AppState>>,
) -> std::result::Result<(), String> {
    let store = state.read().await.store.clone();
    discard_impl(&store, &idle_started)
        .await
        .map_err(|e| e.to_string())
}
