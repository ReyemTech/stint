use anyhow::{anyhow, Result};
use clap::Subcommand;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::Reference;
use stint_core::sync::refresh::refresh_reference_data;

use super::open_store;

#[derive(Subcommand)]
pub enum ProjectsCmd {
    /// List cached projects (run `projects refresh` first to pull).
    List,
    /// Pull projects/tasks/tags from Solidtime.
    Refresh,
    /// Print the raw Solidtime `/projects` response. Diagnostic only.
    Raw,
}

pub async fn run(p: ProjectsCmd) -> Result<()> {
    let store = open_store().await?;
    match p {
        ProjectsCmd::List => {
            let r = Reference::new(store);
            for p in r.list_projects().await? {
                let bill = if p.billable_default != 0 { "$" } else { " " };
                println!("{bill} {}  {}", p.id, p.name);
            }
            Ok(())
        }
        ProjectsCmd::Refresh => {
            let client = build_client(&store).await?;
            refresh_reference_data(&store, &client).await?;
            println!("✓ refreshed");
            Ok(())
        }
        ProjectsCmd::Raw => {
            let client = build_client(&store).await?;
            let body = client.list_projects_raw().await?;
            println!("{body}");
            Ok(())
        }
    }
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
