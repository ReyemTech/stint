use std::time::Duration;
use stint_core::oauth::loopback::{listen_for_callback, CapturedCallback};

#[tokio::test]
async fn captures_code_and_state_from_callback() {
    let server = listen_for_callback(Duration::from_secs(5), "Solidtime")
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
    let server = listen_for_callback(Duration::from_secs(5), "Solidtime")
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
    let server = listen_for_callback(Duration::from_millis(200), "Solidtime")
        .await
        .expect("bind loopback");
    // Don't hit the callback URL — let it time out.
    let err = server.await_callback().await.unwrap_err();
    assert!(
        matches!(err, stint_core::Error::OAuthCancelled),
        "got {err:?}"
    );
}

#[tokio::test]
async fn success_html_includes_provider_label() {
    use std::time::Duration;
    use stint_core::oauth::loopback::listen_for_callback;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let server = listen_for_callback(Duration::from_secs(5), "Google")
        .await
        .expect("bind");
    let port = server.port();

    // Hit the callback with a fake code+state pair so the success branch fires.
    tokio::spawn(async move {
        let mut sock = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        sock.write_all(b"GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        sock.read_to_end(&mut buf).await.unwrap();
        let body = String::from_utf8_lossy(&buf);
        assert!(body.contains("Signed in to Google"), "got: {body}");
    });

    let cap = server.await_callback().await.unwrap();
    assert_eq!(cap.code, "abc");
    assert_eq!(cap.state, "xyz");
}
