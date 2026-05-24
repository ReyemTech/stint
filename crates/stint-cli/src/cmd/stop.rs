use anyhow::Result;
use stint_core::verbs;

use super::open_store;

pub async fn run(json: bool) -> Result<()> {
    let store = open_store().await?;
    let view = verbs::stop(&store).await?;
    crate::render::render(&view, json, |v| {
        println!("Stopped: {}", v.local_uuid);
    });
    Ok(())
}
