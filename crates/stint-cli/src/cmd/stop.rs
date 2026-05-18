use anyhow::Result;
use stint_core::timer::TimerService;

use super::open_store;

pub async fn run() -> Result<()> {
    let store = open_store().await?;
    let timer = TimerService::new(store);
    let id = timer.stop().await?;
    println!("Stopped: {id}");
    Ok(())
}
