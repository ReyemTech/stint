use crate::{
    solidtime::SolidtimeClient,
    store::{
        reference::{ClientRow, ProjectRow, Reference, TagRow, TaskRow},
        Store,
    },
    Result,
};

pub async fn refresh_reference_data(store: &Store, client: &SolidtimeClient) -> Result<()> {
    let r = Reference::new(store.clone());

    let clients = client.list_clients().await?;
    let client_rows: Vec<ClientRow> = clients
        .into_iter()
        .map(|c| ClientRow {
            id: c.id,
            name: c.name,
            archived: if c.archived { 1 } else { 0 },
        })
        .collect();
    r.upsert_clients(&client_rows).await?;

    let projects = client.list_projects().await?;
    let proj_rows: Vec<ProjectRow> = projects
        .into_iter()
        .map(|p| ProjectRow {
            id: p.id,
            name: p.name,
            color: p.color,
            client_id: p.client_id,
            client_name: None,
            archived: if p.archived { 1 } else { 0 },
        })
        .collect();
    r.upsert_projects(&proj_rows).await?;

    let tasks = client.list_tasks().await?;
    let task_rows: Vec<TaskRow> = tasks
        .into_iter()
        .map(|t| TaskRow {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            done: if t.done { 1 } else { 0 },
        })
        .collect();
    r.upsert_tasks(&task_rows).await?;

    let tags = client.list_tags().await?;
    let tag_rows: Vec<TagRow> = tags
        .into_iter()
        .map(|t| TagRow {
            id: t.id,
            name: t.name,
        })
        .collect();
    r.upsert_tags(&tag_rows).await?;

    Ok(())
}
