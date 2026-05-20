mod common;

use stint_core::calendar::store::CalendarStore;
use stint_core::calendar::types::{CalendarAccount, ProviderKind};

fn sample_account(id: &str, email: &str) -> CalendarAccount {
    CalendarAccount {
        id: id.into(),
        provider: ProviderKind::Google,
        display_name: email.into(),
        identifier: email.into(),
        caldav_url: None,
        enabled: true,
        created_at: "2026-05-19T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn add_then_list_returns_one_account() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());

    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    let list = s.list_accounts().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "acc-1");
    assert_eq!(list[0].identifier, "me@example.com");
    assert!(list[0].enabled);
}

#[tokio::test]
async fn get_account_returns_none_for_missing() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    assert!(s.get_account("does-not-exist").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_account_removes_it() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    s.delete_account("acc-1").await.unwrap();
    assert!(s.list_accounts().await.unwrap().is_empty());
}

#[tokio::test]
async fn add_account_with_duplicate_id_returns_error() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    let err = s.add_account(&sample_account("acc-1", "other@example.com")).await.unwrap_err();
    // sqlx returns a UNIQUE-constraint violation; surfaces as Error::Sqlite.
    assert!(matches!(err, stint_core::Error::Sqlite(_)));
}

#[tokio::test]
async fn set_enabled_toggles() {
    let env = common::setup().await;
    let s = CalendarStore::new(env.store.clone());
    s.add_account(&sample_account("acc-1", "me@example.com")).await.unwrap();
    s.set_account_enabled("acc-1", false).await.unwrap();
    let a = s.get_account("acc-1").await.unwrap().unwrap();
    assert!(!a.enabled);
}
