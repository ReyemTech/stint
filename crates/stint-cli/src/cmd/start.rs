use anyhow::Result;
use stint_core::timer::{StartArgs, TimerService};

use crate::at_parse;

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
    /// Backdate the start. Accepts relative ("15min ago", "1h ago"),
    /// bare HH:MM (today local time, shifted to yesterday if in the future),
    /// or RFC 3339.
    #[arg(long)]
    pub at: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);
    let start_at = match args.at.as_deref() {
        Some(s) => Some(at_parse::parse_at_arg(s)?),
        None => None,
    };
    let id = timer
        .start(StartArgs {
            description: args.description.clone(),
            project_id: args.project,
            task_id: args.task,
            billable: false,
            source: "cli".into(),
            start_at,
        })
        .await?;
    println!("Started: {} ({})", args.description, id);
    Ok(())
}
