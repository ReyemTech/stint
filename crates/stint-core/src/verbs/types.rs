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
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub project_id: Option<Option<String>>, // None = no change, Some(None) = clear
    #[serde(default, with = "::serde_with::rust::double_option")]
    pub task_id: Option<Option<String>>,
    #[serde(default)]
    pub billable: Option<bool>,
    #[serde(default)]
    pub start_at: Option<String>,
    #[serde(default, with = "::serde_with::rust::double_option")]
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
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskView {
    pub solidtime_id: String,
    pub project_id: String,
    pub name: String,
    pub done: bool,
}

impl From<crate::store::reference::TaskRow> for TaskView {
    fn from(row: crate::store::reference::TaskRow) -> Self {
        Self {
            solidtime_id: row.id,
            project_id: row.project_id,
            name: row.name,
            done: row.done != 0,
        }
    }
}

impl From<crate::store::reference::ProjectRow> for ProjectView {
    fn from(row: crate::store::reference::ProjectRow) -> Self {
        Self {
            solidtime_id: row.id,
            name: row.name,
            color: row.color,
            client_id: row.client_id,
            archived: row.archived != 0,
        }
    }
}

impl From<crate::store::entries::TimeEntryRow> for EntryView {
    fn from(row: crate::store::entries::TimeEntryRow) -> Self {
        Self {
            local_uuid: row.local_uuid,
            solidtime_id: row.solidtime_id,
            description: row.description,
            project_id: row.project_id,
            task_id: row.task_id,
            billable: row.billable != 0,
            start_at: row.start_at,
            end_at: row.end_at,
            source: row.source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_patch_project_id_three_way_distinction() {
        let absent: EntryPatch = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.project_id, None, "absent field must be None");

        let cleared: EntryPatch =
            serde_json::from_str(r#"{"project_id": null}"#).unwrap();
        assert_eq!(
            cleared.project_id,
            Some(None),
            "explicit null must be Some(None) = clear"
        );

        let set: EntryPatch =
            serde_json::from_str(r#"{"project_id": "abc"}"#).unwrap();
        assert_eq!(
            set.project_id,
            Some(Some("abc".into())),
            "string value must be Some(Some(value)) = set"
        );
    }

    #[test]
    fn entry_patch_task_id_three_way_distinction() {
        let absent: EntryPatch = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.task_id, None, "absent field must be None");

        let cleared: EntryPatch =
            serde_json::from_str(r#"{"task_id": null}"#).unwrap();
        assert_eq!(
            cleared.task_id,
            Some(None),
            "explicit null must be Some(None) = clear"
        );

        let set: EntryPatch =
            serde_json::from_str(r#"{"task_id": "xyz"}"#).unwrap();
        assert_eq!(
            set.task_id,
            Some(Some("xyz".into())),
            "string value must be Some(Some(value)) = set"
        );
    }

    #[test]
    fn entry_patch_end_at_three_way_distinction() {
        let absent: EntryPatch = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.end_at, None, "absent field must be None");

        let cleared: EntryPatch =
            serde_json::from_str(r#"{"end_at": null}"#).unwrap();
        assert_eq!(
            cleared.end_at,
            Some(None),
            "explicit null must be Some(None) = clear"
        );

        let set: EntryPatch =
            serde_json::from_str(r#"{"end_at": "2026-05-23T12:00:00Z"}"#).unwrap();
        assert_eq!(
            set.end_at,
            Some(Some("2026-05-23T12:00:00Z".into())),
            "string value must be Some(Some(value)) = set"
        );
    }
}
