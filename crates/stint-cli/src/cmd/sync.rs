use anyhow::{anyhow, Result};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::drain_once;

use super::open_store;

pub async fn run() -> Result<()> {
    let store = open_store().await?;
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();

    let url = settings.get("solidtime.url").await?.ok_or_else(|| anyhow!("solidtime.url not set"))?;
    let token = secrets.get("solidtime.token")?.ok_or_else(|| anyhow!("solidtime.token not set"))?;
    let org = settings.get("solidtime.org").await?.ok_or_else(|| anyhow!("solidtime.org not set"))?;

    let client = SolidtimeClient::new(&url, &token).with_org(org);
    let n = drain_once(&store, &client).await?;
    println!("Drained {n} item(s) from the sync queue.");
    Ok(())
}
