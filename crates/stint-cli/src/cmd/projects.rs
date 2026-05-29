use anyhow::{anyhow, Result};
use clap::{Args, Subcommand};
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;
use stint_core::store::reference::Reference;
use stint_core::sync::refresh::refresh_reference_data;
use stint_core::verbs;

use super::open_store;

#[derive(Subcommand)]
pub enum ProjectsCmd {
    /// List cached projects (run `projects refresh` first to pull).
    List,
    /// List tasks for a project (by Solidtime project ID).
    ListTasks(ListTasksArgs),
    /// Pull projects/tasks/tags from Solidtime.
    Refresh,
    /// Print the raw Solidtime `/projects` response. Diagnostic only.
    Raw,
}

#[derive(Args)]
pub struct ListTasksArgs {
    /// Solidtime project ID to filter tasks by.
    pub project_id: String,
}

pub async fn run(p: ProjectsCmd, json: bool) -> Result<()> {
    let store = open_store().await?;
    match p {
        ProjectsCmd::List => {
            let views = verbs::list_projects(&store).await?;
            if json {
                crate::render::render(&views, true, |_| {});
                return Ok(());
            }
            // Human output preserves the legacy `$` billable-default marker.
            // `ProjectView` omits `billable_default` from the wire-stable
            // shape, so we re-fetch rows for that one field.
            let r = Reference::new(store);
            for p in r.list_projects().await? {
                let bill = if p.billable_default != 0 { "$" } else { " " };
                println!("{bill} {}  {}", p.id, p.name);
            }
            Ok(())
        }
        ProjectsCmd::ListTasks(args) => {
            let tasks = verbs::list_tasks(&store, Some(args.project_id)).await?;
            crate::render::render(&tasks, json, |tasks| {
                if tasks.is_empty() {
                    println!("(no tasks)");
                } else {
                    for t in tasks {
                        println!("  {}  {}", t.solidtime_id, t.name);
                    }
                }
            });
            Ok(())
        }
        ProjectsCmd::Refresh => {
            let client = build_client(&store).await?;
            refresh_reference_data(&store, &client).await?;
            let ack = serde_json::json!({ "refreshed": true });
            crate::render::render(&ack, json, |_| println!("✓ refreshed"));
            Ok(())
        }
        ProjectsCmd::Raw => {
            let client = build_client(&store).await?;
            let body = client.list_projects_raw().await?;
            // Admin diagnostic — body is already JSON from the server.
            // Print it verbatim regardless of `--json`.
            let _ = json;
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
