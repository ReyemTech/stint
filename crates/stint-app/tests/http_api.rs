//! In-process axum test using `tower::ServiceExt::oneshot` — no real socket bind.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

mod common;

#[tokio::test]
async fn start_stop_current_via_http() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    // start
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/start")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"description":"http test","source":"http"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // current
    let resp = app
        .clone()
        .oneshot(Request::get("/v1/current").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["description"], "http test");

    // stop
    let resp = app
        .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
