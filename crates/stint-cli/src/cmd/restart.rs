use anyhow::{anyhow, Result};
use stint_core::store::entries::Entries;
use stint_core::verbs::{self, StartParams};

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Local UUID of an existing entry to clone. The new timer inherits
    /// its description, project, task, and billable flag.
    pub local_uuid: String,
}

pub async fn run(args: Args, json: bool) -> Result<()> {
    let store = open_store().await?;
    let entries = Entries::new(store.clone());
    let template = entries
        .get(&args.local_uuid)
        .await?
        .ok_or_else(|| anyhow!("entry {} not found", args.local_uuid))?;

    // Best-effort stop of any running timer so the start below doesn't
    // collide. Ignore "no timer running" — that's the expected idle case.
    let _ = verbs::stop(&store).await;

    let view = verbs::start(
        &store,
        StartParams {
            description: template.description.clone(),
            project_id: template.project_id,
            task_id: template.task_id,
            billable: template.billable != 0,
            start_at: None,
            source: "cli".into(),
        },
    )
    .await?;

    crate::render::render(&view, json, |v| {
        println!("Restarted: {} ({})", v.description, v.local_uuid);
    });
    Ok(())
}
