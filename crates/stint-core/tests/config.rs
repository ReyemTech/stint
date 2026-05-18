mod common;

use stint_core::config::Settings;

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
    s.set("solidtime.url", "https://time.reyem.ca").await.unwrap();
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
