use crate::{
    solidtime::SolidtimeClient,
    store::{
        reference::{ClientRow, ProjectRow, Reference, TagRow, TaskRow},
        Store,
    },
    Result,
};

/// Pull all four reference-data entity lists from Solidtime and reconcile
/// the local cache. For each entity we:
///
/// 1. Upsert the rows the server returned.
/// 2. Prune any local row that is no longer in the server's set:
///    - **Projects + clients** are soft-archived (`archived = 1`) so the
///      picker hides them but historical entries can still resolve the
///      project / client name.
///    - **Tasks + tags** are hard-deleted (no `archived` column). Entries
///      with a dangling `task_id` simply show no task name.
///
/// Without the prune step (the pre-Phase-6 behavior), anything deleted on
/// Solidtime lingered locally forever — the user saw stale projects in
/// every picker until they nuked the database.
pub async fn refresh_reference_data(store: &Store, client: &SolidtimeClient) -> Result<()> {
    let r = Reference::new(store.clone());

    let clients = client.list_clients().await?;
    let client_ids: Vec<&str> = clients.iter().map(|c| c.id.as_str()).collect();
    let client_rows: Vec<ClientRow> = clients
        .iter()
        .map(|c| ClientRow {
            id: c.id.clone(),
            name: c.name.clone(),
            archived: if c.archived { 1 } else { 0 },
        })
        .collect();
    r.upsert_clients(&client_rows).await?;
    r.archive_clients_not_in(&client_ids).await?;

    let projects = client.list_projects().await?;
    let project_ids: Vec<&str> = projects.iter().map(|p| p.id.as_str()).collect();
    let proj_rows: Vec<ProjectRow> = projects
        .iter()
        .map(|p| ProjectRow {
            id: p.id.clone(),
            name: p.name.clone(),
            color: p.color.clone(),
            client_id: p.client_id.clone(),
            client_name: None,
            archived: if p.archived { 1 } else { 0 },
            billable_default: if p.is_billable { 1 } else { 0 },
        })
        .collect();
    r.upsert_projects(&proj_rows).await?;
    r.archive_projects_not_in(&project_ids).await?;

    let tasks = client.list_tasks().await?;
    let task_ids: Vec<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let task_rows: Vec<TaskRow> = tasks
        .iter()
        .map(|t| TaskRow {
            id: t.id.clone(),
            project_id: t.project_id.clone(),
            name: t.name.clone(),
            done: if t.done { 1 } else { 0 },
        })
        .collect();
    r.upsert_tasks(&task_rows).await?;
    r.delete_tasks_not_in(&task_ids).await?;

    let tags = client.list_tags().await?;
    let tag_ids: Vec<&str> = tags.iter().map(|t| t.id.as_str()).collect();
    let tag_rows: Vec<TagRow> = tags
        .iter()
        .map(|t| TagRow {
            id: t.id.clone(),
            name: t.name.clone(),
        })
        .collect();
    r.upsert_tags(&tag_rows).await?;
    r.delete_tags_not_in(&tag_ids).await?;

    Ok(())
}
