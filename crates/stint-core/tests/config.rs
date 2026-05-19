mod common;

use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;

// NOTE: These tests touch the real macOS Keychain. We use a unique service
// suffix per test run to avoid collisions and clean up after ourselves.
fn unique_secrets() -> (Secrets, String) {
    let suffix = format!("test-{}", uuid::Uuid::new_v4());
    let secrets = Secrets::with_service_prefix(format!("tech.reyem.stint.{suffix}"));
    (secrets, suffix)
}

#[test]
fn set_get_delete_round_trip() {
    // CI does not have a usable macOS Keychain (the login keychain on
    // GitHub-hosted runners is locked by default and prompts differ from
    // end-user behaviour). Local developers run this test; CI sets
    // STINT_SKIP_KEYCHAIN_TESTS=1 so the suite still passes without it.
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }

    let (secrets, _suffix) = unique_secrets();

    assert!(secrets.get("k").unwrap().is_none());
    secrets.set("k", "hunter2").unwrap();
    assert_eq!(secrets.get("k").unwrap().as_deref(), Some("hunter2"));
    secrets.delete("k").unwrap();
    assert!(secrets.get("k").unwrap().is_none());
}

#[tokio::test]
async fn get_returns_none_for_unknown_key() {
    let env = common::setup().await;
    let s = Settings::new(env.store.clone());
    assert_eq!(s.get("nope").await.unwrap(), None);
}

#[tokio::test]
async fn set_then_get_round_trips() {
    let env = common::setup().await;
    let s = Settings::new(env.store.clone());
    s.set("solidtime.url", "https://time.reyem.ca")
        .await
        .unwrap();
    assert_eq!(
        s.get("solidtime.url").await.unwrap().as_deref(),
        Some("https://time.reyem.ca")
    );
}

#[tokio::test]
async fn set_twice_updates_value() {
    let env = common::setup().await;
    let s = Settings::new(env.store.clone());
    s.set("key", "v1").await.unwrap();
    s.set("key", "v2").await.unwrap();
    assert_eq!(s.get("key").await.unwrap().as_deref(), Some("v2"));
}

#[tokio::test]
async fn delete_removes_key() {
    let env = common::setup().await;
    let s = Settings::new(env.store.clone());
    s.set("key", "v").await.unwrap();
    s.delete("key").await.unwrap();
    assert_eq!(s.get("key").await.unwrap(), None);
}
