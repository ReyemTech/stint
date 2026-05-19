mod common;

use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::solidtime::auth::{build_token_provider, AuthMode};

#[tokio::test]
async fn returns_api_token_provider_when_mode_is_api_token() {
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.auth_mode", "api_token")
        .await
        .unwrap();
    let secrets =
        Secrets::with_service_prefix(format!("tech.reyem.stint.test-{}", uuid::Uuid::new_v4()));
    secrets.set("solidtime", "the-pat-token").unwrap();

    let (provider, _client) = build_token_provider(
        &Settings::new(env.store.clone()),
        &secrets,
        "https://time.example.com",
    )
    .await
    .unwrap();
    assert_eq!(provider.access_token().await.unwrap(), "the-pat-token");
    secrets.delete("solidtime").unwrap();
}

#[tokio::test]
async fn returns_missing_config_when_oauth_mode_but_no_blob() {
    let env = common::setup().await;
    Settings::new(env.store.clone())
        .set("solidtime.auth_mode", "oauth")
        .await
        .unwrap();
    let secrets =
        Secrets::with_service_prefix(format!("tech.reyem.stint.test-{}", uuid::Uuid::new_v4()));

    let result = build_token_provider(
        &Settings::new(env.store.clone()),
        &secrets,
        "https://time.example.com",
    )
    .await;
    assert!(result.is_err(), "expected Err but got Ok");
    let err = result.err().unwrap();
    match err {
        stint_core::Error::MissingConfig(k) => {
            assert_eq!(k, "solidtime.oauth");
        }
        e => panic!("expected MissingConfig, got {e:?}"),
    }
}

#[test]
fn auth_mode_defaults_to_api_token() {
    assert_eq!(AuthMode::from_str_or_default(None), AuthMode::ApiToken);
    assert_eq!(
        AuthMode::from_str_or_default(Some("api_token")),
        AuthMode::ApiToken
    );
    assert_eq!(
        AuthMode::from_str_or_default(Some("oauth")),
        AuthMode::OAuth
    );
    // Unknown values default safely to ApiToken (prevent locking the user out).
    assert_eq!(
        AuthMode::from_str_or_default(Some("garbage")),
        AuthMode::ApiToken
    );
}
