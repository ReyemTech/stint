use anyhow::{anyhow, Result};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::sync::pull::{pull, Trigger};

use super::open_store;

#[derive(clap::Args)]
pub struct Args {
    /// Surface a conflict without resolving it (default behaviour).
    #[arg(long, conflicts_with_all = ["switch", "stop_remote"])]
    pub dismiss: bool,
    /// Stop the remote running timer if a conflict is detected.
    #[arg(long, conflicts_with_all = ["switch", "dismiss"])]
    pub stop_remote: bool,
    /// Stop the local running timer and adopt the remote one.
    #[arg(long, conflicts_with_all = ["stop_remote", "dismiss"])]
    pub switch: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let store = open_store().await?;
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
    let client = SolidtimeClient::new(&url, provider).with_org(org);

    let report = pull(&store, &client, Trigger::Manual).await?;

    println!(
        "+{} entries, ~{} updates, -{} deletes",
        report.inserted, report.updated, report.deleted
    );
    if let Some(adopted) = &report.adopted {
        println!("Adopted remote running timer (local uuid: {adopted})");
    }
    if let Some(c) = &report.conflict {
        eprintln!(
            "Conflict: remote timer \"{}\" started {} differs from local \"{}\".",
            c.remote_description, c.remote_start_at, c.local_description
        );
        if args.stop_remote {
            eprintln!("(--stop-remote requested; resolution support lands in Task 10)");
        } else if args.switch {
            eprintln!("(--switch requested; resolution support lands in Task 10)");
        } else {
            eprintln!("Re-run with --stop-remote, --switch, or --dismiss.");
        }
    }
    Ok(())
}
