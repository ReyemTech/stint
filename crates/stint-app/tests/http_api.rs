//! In-process axum tests using `tower::ServiceExt::oneshot` for handlers, plus
//! one bind-and-listen test for `http::maybe_spawn`. All handlers are
//! exercised here so coverage in `http/handlers.rs` and `http/error.rs`
//! reflects production usage rather than the legacy single-path smoke test.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use stint_core::config::{Settings, KEY_API_ENABLED};
use tokio::sync::RwLock;
use tower::ServiceExt;

mod common;

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    if bytes.is_empty() {
        return serde_json::Value::Null;
    }
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn start_stop_current_via_http() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

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

    let resp = app
        .clone()
        .oneshot(Request::get("/v1/current").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["description"], "http test");

    let resp = app
        .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn list_entries_returns_array_after_starts_and_stops() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    for desc in ["a", "b"] {
        let body = format!(r#"{{"description":"{desc}","source":"http"}}"#);
        let resp = app
            .clone()
            .oneshot(
                Request::post("/v1/start")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let resp = app
            .clone()
            .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    let resp = app
        .oneshot(Request::get("/v1/entries").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v.is_array());
    assert_eq!(v.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn list_projects_and_tasks_return_empty_arrays_on_fresh_store() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    let resp = app
        .clone()
        .oneshot(Request::get("/v1/projects").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!([]));

    let resp = app
        .clone()
        .oneshot(Request::get("/v1/tasks").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!([]));

    // With project_id query string — exercises the Query<ListTasksQuery> arm.
    let resp = app
        .oneshot(
            Request::get("/v1/tasks?project_id=p-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v, serde_json::json!([]));
}

#[tokio::test]
async fn update_entry_via_http_returns_updated_view() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/start")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"description":"before","source":"http"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    let id = v["local_uuid"].as_str().unwrap().to_string();

    let _ = app
        .clone()
        .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::patch(format!("/v1/entries/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"description":"after"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["description"], "after");
}

#[tokio::test]
async fn delete_entry_via_http_removes_row() {
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/start")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"description":"doomed","source":"http"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(resp).await;
    let id = v["local_uuid"].as_str().unwrap().to_string();
    let _ = app
        .clone()
        .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::delete(format!("/v1/entries/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .oneshot(Request::get("/v1/entries").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let v = body_json(resp).await;
    assert!(v.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn update_entry_for_missing_uuid_returns_404_with_json_error() {
    // Exercises the `Error::NotFound` arm in `http::error::ApiError::into_response`.
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    let resp = app
        .oneshot(
            Request::patch("/v1/entries/does-not-exist")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"description":"noop"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let v = body_json(resp).await;
    assert!(v["error"].is_string());
}

#[tokio::test]
async fn stop_with_no_running_timer_returns_500_with_json_error() {
    // Stop on an idle store hits the generic `_ => INTERNAL_SERVER_ERROR`
    // arm of ApiError.
    let ctx = common::make_app().await;
    let app = stint_app::http::build_router(ctx.store.clone());

    let resp = app
        .oneshot(Request::post("/v1/stop").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let v = body_json(resp).await;
    assert!(v["error"].is_string());
}

// ---- maybe_spawn integration ------------------------------------------

#[tokio::test]
async fn maybe_spawn_returns_none_when_api_disabled() {
    let ctx = common::make_app().await;
    let port_slot = Arc::new(RwLock::new(None));

    let bound = stint_app::http::maybe_spawn(ctx.store.clone(), port_slot.clone())
        .await
        .expect("maybe_spawn ok");
    assert!(bound.is_none());
    assert!(port_slot.read().await.is_none());
}

#[tokio::test]
async fn maybe_spawn_binds_when_enabled_and_records_port() {
    let ctx = common::make_app().await;
    let settings = Settings::new((*ctx.store).clone());
    settings
        .set(KEY_API_ENABLED, "true")
        .await
        .expect("enable");

    let port_slot = Arc::new(RwLock::new(None));
    let bound = stint_app::http::maybe_spawn(ctx.store.clone(), port_slot.clone())
        .await
        .expect("maybe_spawn ok");

    let port = bound.expect("server bound to a port");
    assert!(port > 0, "port should be non-zero");
    assert_eq!(*port_slot.read().await, Some(port));
}
