//! `list_tasks` verb — return reference tasks, optionally filtered by project.

use crate::store::reference::Reference;
use crate::store::Store;
use crate::verbs::types::TaskView;
use crate::Result;

pub async fn list_tasks(store: &Store, project_id: Option<String>) -> Result<Vec<TaskView>> {
    let reference = Reference::new(store.clone());
    let rows = match project_id {
        Some(pid) => reference.list_tasks(&pid).await?,
        None => reference.list_all_tasks().await?,
    };
    Ok(rows.into_iter().map(TaskView::from).collect())
}
