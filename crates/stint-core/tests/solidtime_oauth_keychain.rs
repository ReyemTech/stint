use chrono::Utc;
use stint_core::config::secrets::Secrets;
use stint_core::oauth::tokens::TokenSet;
use stint_core::solidtime::auth::{oauth_blob_delete, oauth_blob_load, oauth_blob_save, OAuthBlob};

fn unique_secrets() -> Secrets {
    Secrets::with_service_prefix(format!("tech.reyem.stint.test-{}", uuid::Uuid::new_v4()))
}

#[test]
fn round_trips_blob_through_keychain() {
    if std::env::var("STINT_SKIP_KEYCHAIN_TESTS").is_ok() {
        eprintln!("skipping: STINT_SKIP_KEYCHAIN_TESTS is set");
        return;
    }
    let secrets = unique_secrets();

    assert!(oauth_blob_load(&secrets).unwrap().is_none());
    let tokens = TokenSet::from_response(
        "a".into(),
        Some("r".into()),
        3600,
        Some("read".into()),
        Utc::now(),
    );
    let blob = OAuthBlob {
        client_id: "stint-desktop".into(),
        tokens,
    };
    oauth_blob_save(&secrets, &blob).unwrap();

    let loaded = oauth_blob_load(&secrets).unwrap().expect("present");
    assert_eq!(loaded.client_id, "stint-desktop");
    assert_eq!(loaded.tokens.access_token, "a");

    oauth_blob_delete(&secrets).unwrap();
    assert!(oauth_blob_load(&secrets).unwrap().is_none());
}
