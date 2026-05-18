use anyhow::Result;
use stint_core::timer::TimerService;

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Entry UUID (or its 8-character prefix).
    pub id: String,
    /// New description.
    #[arg(long)]
    pub description: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);

    if let Some(d) = args.description {
        timer.update_description(&args.id, &d).await?;
        println!("Updated description for {}.", &args.id);
    } else {
        println!("Nothing to update. Pass --description to change something.");
    }
    Ok(())
}
