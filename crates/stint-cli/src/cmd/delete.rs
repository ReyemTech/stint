use anyhow::Result;
use stint_core::timer::TimerService;

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Entry UUID (or its 8-character prefix).
    pub id: String,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);
    timer.delete(&args.id).await?;
    println!("Deleted {}.", args.id);
    Ok(())
}
