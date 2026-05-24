//! Input and output DTOs shared by all transports.

use serde::{Deserialize, Serialize};

/// Inputs ----------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct StartParams {
    pub description: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub billable: bool,
    /// ISO 8601 UTC. None = "now". Caller is responsible for `at`-style
    /// parsing (relative/HH:MM); this layer accepts only normalized UTC.
    #[serde(default)]
    pub start_at: Option<String>,
    /// Provenance tag stored on the entry (e.g., "cli", "gui", "mcp", "http").
    pub source: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntryFilter {
    /// ISO 8601 UTC; entries with start_at >= this are included.
    #[serde(default)]
    pub since: Option<String>,
    /// ISO 8601 UTC; entries with start_at < this are included.
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    /// Max rows to return. None = unlimited (use with care).
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntryPatch {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_id: Option<Option<String>>, // None = no change, Some(None) = clear
    #[serde(default)]
    pub task_id: Option<Option<String>>,
    #[serde(default)]
    pub billable: Option<bool>,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default)]
    pub end_at: Option<Option<String>>,
}

/// Outputs ---------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct EntryView {
    pub local_uuid: String,
    pub solidtime_id: Option<String>,
    pub description: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub billable: bool,
    pub start_at: String,
    pub end_at: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectView {
    pub solidtime_id: String,
    pub name: String,
    pub color: Option<String>,
    pub client_id: Option<String>,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub solidtime_id: String,
    pub project_id: String,
    pub name: String,
    pub done: bool,
}
