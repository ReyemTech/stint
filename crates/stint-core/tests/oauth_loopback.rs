use std::time::Duration;
use stint_core::oauth::loopback::{listen_for_callback, CapturedCallback};

#[tokio::test]
async fn captures_code_and_state_from_callback() {
    let server = listen_for_callback(Duration::from_secs(5))
        .await
        .expect("bind loopback");
    let port = server.port();
    let url = format!("http://127.0.0.1:{port}/callback?code=test-code&state=test-state");

    // In a real flow the browser hits this URL. We simulate it with reqwest.
    tokio::spawn(async move {
        let _ = reqwest::get(&url).await;
    });

    let captured: CapturedCallback = server.await_callback().await.expect("capture");
    assert_eq!(captured.code, "test-code");
    assert_eq!(captured.state, "test-state");
}

#[tokio::test]
async fn returns_oauth_server_error_when_callback_carries_error_param() {
    let server = listen_for_callback(Duration::from_secs(5))
        .await
        .expect("bind loopback");
    let port = server.port();
    let url = format!(
        "http://127.0.0.1:{port}/callback?error=access_denied&error_description=User+rejected"
    );
    tokio::spawn(async move {
        let _ = reqwest::get(&url).await;
    });
    let err = server.await_callback().await.unwrap_err();
    match err {
        stint_core::Error::OAuthServer(msg) => {
            assert!(msg.contains("access_denied"), "got: {msg}");
        }
        e => panic!("expected OAuthServer, got {e:?}"),
    }
}

#[tokio::test]
async fn times_out_with_oauth_cancelled() {
    let server = listen_for_callback(Duration::from_millis(200))
        .await
        .expect("bind loopback");
    // Don't hit the callback URL — let it time out.
    let err = server.await_callback().await.unwrap_err();
    assert!(
        matches!(err, stint_core::Error::OAuthCancelled),
        "got {err:?}"
    );
}
