//! Tool surface for the MCP server. Each tool delegates to a single
//! `stint_core::verbs::*` function — no business logic here.
//!
//! Tool descriptions are written for AI agents: terse, action-oriented,
//! and explicit about side effects. Inputs use MCP-local types (with
//! `JsonSchema` derived) and convert to the verb's input DTO at the call
//! boundary, so `stint-core` stays free of `schemars`.
//!
//! Returns are JSON-serialized payloads in a text-content block (rather
//! than `structured_content`) — that's the lowest common denominator across
//! current MCP clients and keeps the wire format the same as the CLI's
//! `--json` output.

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use stint_core::store::Store;
use stint_core::verbs::{self, EntryFilter, EntryPatch, StartParams};

#[derive(Clone)]
pub struct StintServer {
    store: std::sync::Arc<Store>,
}

impl StintServer {
    pub fn new(store: Store) -> Self {
        Self {
            store: std::sync::Arc::new(store),
        }
    }
}

// ---------------------------------------------------------------------------
// Input shapes — duplicated here (rather than on `stint_core::verbs::types`)
// to keep `schemars` out of the core crate's dep graph.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartInput {
    /// What you're working on. Free-form, shown in lists and synced to
    /// Solidtime as the entry description.
    pub description: String,
    /// Optional Solidtime project UUID. Use `list_projects` to discover IDs.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Optional Solidtime task UUID within the project.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Mark this entry as billable. Defaults to false.
    #[serde(default)]
    pub billable: bool,
    /// Backdate the start. ISO 8601 UTC (e.g. "2026-05-24T09:30:00Z"). Omit
    /// for "now".
    #[serde(default)]
    pub start_at: Option<String>,
}

impl From<StartInput> for StartParams {
    fn from(i: StartInput) -> Self {
        StartParams {
            description: i.description,
            project_id: i.project_id,
            task_id: i.task_id,
            billable: i.billable,
            start_at: i.start_at,
            // Provenance tag — overrides whatever the caller might have sent.
            source: "mcp".into(),
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ListEntriesInput {
    /// Lower bound (inclusive) on `start_at`. ISO 8601 UTC.
    #[serde(default)]
    pub since: Option<String>,
    /// Upper bound (exclusive) on `start_at`. ISO 8601 UTC.
    #[serde(default)]
    pub until: Option<String>,
    /// Restrict to one Solidtime project UUID.
    #[serde(default)]
    pub project_id: Option<String>,
    /// Cap the number of rows returned. Omit for "no cap".
    #[serde(default)]
    pub limit: Option<u32>,
}

impl From<ListEntriesInput> for EntryFilter {
    fn from(i: ListEntriesInput) -> Self {
        EntryFilter {
            since: i.since,
            until: i.until,
            project_id: i.project_id,
            limit: i.limit,
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct ListTasksInput {
    /// Restrict to tasks under one project. Omit for all tasks.
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateEntryInput {
    /// Local UUID of the entry to mutate. See `list_entries` for IDs.
    pub local_uuid: String,
    /// New description (omit to leave unchanged).
    #[serde(default)]
    pub description: Option<String>,
    /// Set or clear (null) the project. Omit field to leave unchanged.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[schemars(with = "Option<String>")]
    pub project_id: Option<Option<String>>,
    /// Set or clear (null) the task. Omit field to leave unchanged.
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[schemars(with = "Option<String>")]
    pub task_id: Option<Option<String>>,
    /// Flip the billable flag.
    #[serde(default)]
    pub billable: Option<bool>,
    /// New start_at, ISO 8601 UTC. Requires the entry already has an end_at
    /// or that end_at is also supplied.
    #[serde(default)]
    pub start_at: Option<String>,
    /// New end_at, ISO 8601 UTC. `null` clears it (re-opens the entry).
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[schemars(with = "Option<String>")]
    pub end_at: Option<Option<String>>,
}

impl UpdateEntryInput {
    fn split(self) -> (String, EntryPatch) {
        (
            self.local_uuid,
            EntryPatch {
                description: self.description,
                project_id: self.project_id,
                task_id: self.task_id,
                billable: self.billable,
                start_at: self.start_at,
                end_at: self.end_at,
            },
        )
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteEntryInput {
    /// Local UUID of the entry to delete. Idempotent — silently OK if gone.
    pub local_uuid: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_text<T: serde::Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string(value)
        .map_err(|e| McpError::internal_error(format!("serialize: {e}"), None))
}

fn map_err(e: stint_core::Error) -> McpError {
    // `Error::NotFound` and `Error::Invariant` are caller-induced; surface
    // them as `invalid_params` so the model can correct. Everything else is
    // an internal failure from the agent's perspective.
    match e {
        stint_core::Error::NotFound(msg) => McpError::invalid_params(msg, None),
        stint_core::Error::Invariant(msg) => McpError::invalid_params(msg, None),
        other => McpError::internal_error(other.to_string(), None),
    }
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[tool_router(server_handler)]
impl StintServer {
    #[tool(
        name = "start",
        description = "Start a new running timer entry. Errors if a timer is already running — call `stop` first or use `current` to check. Returns the new EntryView (local_uuid, description, project_id, start_at, …) as JSON."
    )]
    async fn start(
        &self,
        Parameters(input): Parameters<StartInput>,
    ) -> Result<String, McpError> {
        let view = verbs::start(&self.store, input.into())
            .await
            .map_err(map_err)?;
        json_text(&view)
    }

    #[tool(
        name = "stop",
        description = "Stop the currently running timer. Errors if no timer is running. Returns the stopped EntryView as JSON."
    )]
    async fn stop(&self) -> Result<String, McpError> {
        let view = verbs::stop(&self.store).await.map_err(map_err)?;
        json_text(&view)
    }

    #[tool(
        name = "current",
        description = "Return the currently running entry as JSON, or `null` when idle. Cheap — call this before `start` to avoid the 'already running' error."
    )]
    async fn current(&self) -> Result<String, McpError> {
        let view = verbs::current(&self.store).await.map_err(map_err)?;
        json_text(&view)
    }

    #[tool(
        name = "list_entries",
        description = "List time entries matching an optional date window and/or project filter. Returns an array of EntryView as JSON. Results are ordered oldest-first; pass `limit` to cap."
    )]
    async fn list_entries(
        &self,
        Parameters(input): Parameters<ListEntriesInput>,
    ) -> Result<String, McpError> {
        let rows = verbs::list_entries(&self.store, input.into())
            .await
            .map_err(map_err)?;
        json_text(&rows)
    }

    #[tool(
        name = "list_projects",
        description = "List locally-cached Solidtime projects. Use the `solidtime_id` field as `project_id` in `start` or `update_entry`. Returns an array of ProjectView as JSON."
    )]
    async fn list_projects(&self) -> Result<String, McpError> {
        let rows = verbs::list_projects(&self.store).await.map_err(map_err)?;
        json_text(&rows)
    }

    #[tool(
        name = "list_tasks",
        description = "List locally-cached Solidtime tasks, optionally filtered by `project_id`. Use the `solidtime_id` field as `task_id` in `start` or `update_entry`. Returns an array of TaskView as JSON."
    )]
    async fn list_tasks(
        &self,
        Parameters(input): Parameters<ListTasksInput>,
    ) -> Result<String, McpError> {
        let rows = verbs::list_tasks(&self.store, input.project_id)
            .await
            .map_err(map_err)?;
        json_text(&rows)
    }

    #[tool(
        name = "update_entry",
        description = "Patch an existing entry by `local_uuid`. Omit fields to leave them unchanged; pass `null` for nullable fields (project_id/task_id/end_at) to clear them. Returns the updated EntryView as JSON."
    )]
    async fn update_entry(
        &self,
        Parameters(input): Parameters<UpdateEntryInput>,
    ) -> Result<String, McpError> {
        let (local_uuid, patch) = input.split();
        let view = verbs::update_entry(&self.store, &local_uuid, patch)
            .await
            .map_err(map_err)?;
        json_text(&view)
    }

    #[tool(
        name = "delete_entry",
        description = "Delete an entry by `local_uuid`. Idempotent — silently succeeds if the row is already gone. Returns `{\"ok\":true}` on success."
    )]
    async fn delete_entry(
        &self,
        Parameters(input): Parameters<DeleteEntryInput>,
    ) -> Result<String, McpError> {
        verbs::delete_entry(&self.store, &input.local_uuid)
            .await
            .map_err(map_err)?;
        Ok(r#"{"ok":true}"#.into())
    }
}

#[allow(dead_code)]
fn _assert_server_handler()
where
    StintServer: ServerHandler,
{
}
