//! Convert stint-core errors into HTTP responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use stint_core::Error;

pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            Error::NotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
