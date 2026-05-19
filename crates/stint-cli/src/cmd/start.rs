use anyhow::Result;
use stint_core::timer::{StartArgs, TimerService};

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Description of what you're working on.
    pub description: String,
    /// Project ID (Solidtime UUID).
    #[arg(long)]
    pub project: Option<String>,
    /// Task ID (Solidtime UUID).
    #[arg(long)]
    pub task: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);
    let id = timer
        .start(StartArgs {
            description: args.description.clone(),
            project_id: args.project,
            task_id: args.task,
            billable: false,
            source: "cli".into(),
        })
        .await?;
    println!("Started: {} ({})", args.description, id);
    Ok(())
}
