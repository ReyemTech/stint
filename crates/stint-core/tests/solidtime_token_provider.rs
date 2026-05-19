use std::sync::Arc;
use stint_core::solidtime::auth::{ApiTokenProvider, TokenProvider};

#[tokio::test]
async fn api_token_provider_returns_configured_token() {
    let p: Arc<dyn TokenProvider> = Arc::new(ApiTokenProvider::new("static-token-1".into()));
    let t1 = p.access_token().await.unwrap();
    let t2 = p.access_token().await.unwrap();
    assert_eq!(t1, "static-token-1");
    assert_eq!(t2, "static-token-1");
}
