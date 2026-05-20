use chrono::Utc;
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob,
};
use stint_core::config::secrets::Secrets;
use stint_core::oauth::tokens::TokenSet;

fn unique_prefix() -> String {
    format!("tech.reyem.stint-test.{}", uuid::Uuid::new_v4().simple())
}

#[test]
fn save_load_delete_roundtrip() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    let blob = CalendarOAuthBlob {
        client_id: "fake-google-client-id".into(),
        tokens: TokenSet::from_response(
            "access-1".into(),
            Some("refresh-1".into()),
            3600,
            Some("https://www.googleapis.com/auth/calendar.readonly".into()),
            Utc::now(),
        ),
    };
    let account_uuid = "acc-12345";

    assert!(calendar_blob_load(&secrets, account_uuid)
        .unwrap()
        .is_none());

    calendar_blob_save(&secrets, account_uuid, &blob).unwrap();
    let loaded = calendar_blob_load(&secrets, account_uuid).unwrap().unwrap();
    assert_eq!(loaded.tokens.access_token, "access-1");
    assert_eq!(loaded.client_id, "fake-google-client-id");

    calendar_blob_delete(&secrets, account_uuid).unwrap();
    assert!(calendar_blob_load(&secrets, account_uuid)
        .unwrap()
        .is_none());
}

#[test]
fn load_returns_none_for_missing_account() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    assert!(calendar_blob_load(&secrets, "missing-acc")
        .unwrap()
        .is_none());
}

#[test]
fn load_surfaces_oauth_server_on_corrupt_blob() {
    if std::env::var_os("STINT_SKIP_KEYCHAIN_TESTS").is_some() {
        eprintln!("skipped: STINT_SKIP_KEYCHAIN_TESTS=1");
        return;
    }
    let secrets = Secrets::with_service_prefix(unique_prefix());
    let account_uuid = "acc-bad";
    secrets
        .set(&format!("calendar.{account_uuid}"), "this is not JSON")
        .unwrap();

    let err = calendar_blob_load(&secrets, account_uuid).unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => assert!(msg.contains("malformed")),
        e => panic!("expected OAuthServer, got {e:?}"),
    }
    secrets.delete(&format!("calendar.{account_uuid}")).ok();
}
