use anyhow::Result;
use stint_core::verbs;

use super::open_store;

pub async fn run(json: bool) -> Result<()> {
    let store = open_store().await?;
    let view = verbs::current(&store).await?;
    crate::render::render(&view, json, |v| match v {
        Some(entry) => println!("Running: {} ({})", entry.description, entry.local_uuid),
        None => println!("No timer running."),
    });
    Ok(())
}
