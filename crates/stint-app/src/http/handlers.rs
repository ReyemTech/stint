//! HTTP handlers. Each delegates to the corresponding `stint_core::verbs::*`
//! function — no business logic here. Public visibility so integration tests
//! can construct the router and call handlers via `tower::oneshot`.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;
use stint_core::store::Store;
use stint_core::verbs::{self, EntryFilter, EntryPatch, EntryView, ProjectView, StartParams, TaskView};

use super::error::ApiError;

pub type ApiState = Arc<Store>;

pub async fn start(
    State(store): State<ApiState>,
    Json(params): Json<StartParams>,
) -> Result<Json<EntryView>, ApiError> {
    Ok(Json(verbs::start(&store, params).await?))
}

pub async fn stop(State(store): State<ApiState>) -> Result<Json<EntryView>, ApiError> {
    Ok(Json(verbs::stop(&store).await?))
}

pub async fn current(
    State(store): State<ApiState>,
) -> Result<Json<Option<EntryView>>, ApiError> {
    Ok(Json(verbs::current(&store).await?))
}

pub async fn list_entries(
    State(store): State<ApiState>,
    Query(filter): Query<EntryFilter>,
) -> Result<Json<Vec<EntryView>>, ApiError> {
    Ok(Json(verbs::list_entries(&store, filter).await?))
}

pub async fn list_projects(
    State(store): State<ApiState>,
) -> Result<Json<Vec<ProjectView>>, ApiError> {
    Ok(Json(verbs::list_projects(&store).await?))
}

#[derive(Deserialize)]
pub struct ListTasksQuery {
    pub project_id: Option<String>,
}

pub async fn list_tasks(
    State(store): State<ApiState>,
    Query(q): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskView>>, ApiError> {
    Ok(Json(verbs::list_tasks(&store, q.project_id).await?))
}

pub async fn update_entry(
    State(store): State<ApiState>,
    Path(id): Path<String>,
    Json(patch): Json<EntryPatch>,
) -> Result<Json<EntryView>, ApiError> {
    Ok(Json(verbs::update_entry(&store, &id, patch).await?))
}

pub async fn delete_entry(
    State(store): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<()>, ApiError> {
    Ok(Json(verbs::delete_entry(&store, &id).await?))
}
