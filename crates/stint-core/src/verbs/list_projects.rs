//! `list_projects` verb — surface the locally-cached projects reference table.

use crate::store::reference::Reference;
use crate::store::Store;
use crate::verbs::types::ProjectView;
use crate::Result;

pub async fn list_projects(store: &Store) -> Result<Vec<ProjectView>> {
    let reference = Reference::new(store.clone());
    let rows = reference.list_projects().await?;
    Ok(rows.into_iter().map(ProjectView::from).collect())
}
