use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::queue::Queue;
use stint_core::sync::drain_once;

use super::open_store;

#[derive(Args)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub sub: Option<SyncCmd>,
}

#[derive(Subcommand)]
pub enum SyncCmd {
    /// Drain the sync queue once (default when no subcommand given).
    Drain,
    /// Resurrect queue rows previously parked far in the future by the
    /// abandon-on-4xx path. Their attempts counter resets so the worker
    /// gives the new code a fresh try.
    RetryAbandoned,
}

/// `stint sync` with no subcommand defaults to drain — keeps the old
/// invocation working.
pub async fn run(args: SyncArgs) -> Result<()> {
    match args.sub.unwrap_or(SyncCmd::Drain) {
        SyncCmd::Drain => drain().await,
        SyncCmd::RetryAbandoned => retry_abandoned().await,
    }
}

async fn drain() -> Result<()> {
    let store = open_store().await?;
    let client = build_client(&store).await?;
    let n = drain_once(&store, &client).await?;
    println!("Drained {n} item(s) from the sync queue.");
    Ok(())
}

async fn retry_abandoned() -> Result<()> {
    let store = open_store().await?;
    let n = Queue::new(store.clone()).resurrect_abandoned().await?;
    println!("Reset {n} abandoned queue row(s); next drain will retry them.");
    if n == 0 {
        return Ok(());
    }
    // Drain immediately so the user sees the result without waiting for
    // the background worker tick.
    let client = build_client(&store).await?;
    let drained = drain_once(&store, &client).await?;
    println!("Drained {drained} item(s).");
    Ok(())
}

async fn build_client(store: &stint_core::store::Store) -> Result<SolidtimeClient> {
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();
    let url = settings
        .get("solidtime.url")
        .await?
        .ok_or_else(|| anyhow!("solidtime.url not set"))?;
    let org = settings
        .get("solidtime.org")
        .await?
        .ok_or_else(|| anyhow!("solidtime.org not set"))?;
    let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
    Ok(SolidtimeClient::new(&url, provider).with_org(org))
}
