use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::time::Duration;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::{google_oauth_config, is_configured};
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_save, CalendarOAuthBlob, CalendarStore,
};
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind};
use stint_core::config::secrets::Secrets;
use stint_core::ids;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::login_interactive;
use stint_core::store::Store;
use stint_core::time;

#[derive(Subcommand)]
pub enum CalendarCmd {
    /// Add a Google Calendar account (interactive OAuth flow).
    Add {
        #[arg(value_parser = ["google"])]
        provider: String,
    },
    /// List connected calendar accounts.
    List,
    /// Remove a connected calendar account by id.
    Remove { account_id: String },
    /// List or modify calendars for an account.
    Calendars {
        account_id: String,
        /// Calendar id to include.
        #[arg(long)]
        include: Option<String>,
        /// Calendar id to exclude.
        #[arg(long)]
        exclude: Option<String>,
        /// Set the default project on a calendar:
        /// `--set-default-project <CALENDAR_ID> <PROJECT_ID>`.
        #[arg(long, num_args = 2, value_names = ["CALENDAR_ID", "PROJECT_ID"])]
        set_default_project: Option<Vec<String>>,
        /// Clear the default project on a calendar:
        /// `--clear-default-project <CALENDAR_ID>`.
        #[arg(long)]
        clear_default_project: Option<String>,
    },
    /// Refresh one account's events (on_focus window).
    Refresh { account_id: String },
}

pub async fn run(c: CalendarCmd, store: Store) -> Result<()> {
    let cs = CalendarStore::new(store.clone());
    let secrets = Secrets::default();

    match c {
        CalendarCmd::Add { provider } if provider == "google" => add_google(&cs, &secrets).await,
        CalendarCmd::Add { provider } => Err(anyhow!("unknown provider {provider}")),
        CalendarCmd::List => {
            let accounts = cs.list_accounts().await?;
            if accounts.is_empty() {
                println!("No calendar accounts configured.");
                return Ok(());
            }
            for a in accounts {
                println!(
                    "{} {} {} <{}>",
                    a.id,
                    provider_label(a.provider),
                    a.display_name,
                    a.identifier
                );
            }
            Ok(())
        }
        CalendarCmd::Remove { account_id } => {
            cs.delete_account(&account_id).await?;
            let _ = calendar_blob_delete(&secrets, &account_id);
            println!("Removed account {account_id}.");
            Ok(())
        }
        CalendarCmd::Calendars {
            account_id,
            include,
            exclude,
            set_default_project,
            clear_default_project,
        } => {
            if let Some(id) = include {
                cs.set_calendar_included(&id, true).await?;
                println!("Included calendar {id}.");
            }
            if let Some(id) = exclude {
                cs.set_calendar_included(&id, false).await?;
                println!("Excluded calendar {id}.");
            }
            if let Some(pair) = set_default_project {
                let cal_id = &pair[0];
                let proj_id = &pair[1];
                cs.set_default_project(cal_id, Some(proj_id)).await?;
                println!("Set default project {proj_id} on calendar {cal_id}.");
            }
            if let Some(id) = clear_default_project {
                cs.set_default_project(&id, None).await?;
                println!("Cleared default project on calendar {id}.");
            }
            for c in cs.list_calendars(&account_id).await? {
                let mark = if c.included { "[x]" } else { "[ ]" };
                let default = match &c.default_project_id {
                    Some(p) => format!(" (default: {p})"),
                    None => String::new(),
                };
                println!("{mark} {} {}{default}", c.id, c.name);
            }
            Ok(())
        }
        CalendarCmd::Refresh { account_id } => {
            let provider =
                stint_core::calendar::google::build_provider_from_blob(&secrets, &account_id)?;
            let n =
                refresh_account(&cs, &account_id, provider.as_ref(), Ranges::on_focus()).await?;
            println!("Refreshed {n} events.");
            Ok(())
        }
    }
}

async fn add_google(cs: &CalendarStore, secrets: &Secrets) -> Result<()> {
    if !is_configured() {
        return Err(anyhow!(
            "Google OAuth credentials are not configured in this build. \
             Set STINT_GOOGLE_CLIENT_ID and STINT_GOOGLE_CLIENT_SECRET at build time."
        ));
    }

    let cfg = google_oauth_config();
    let client_id = cfg.client_id.clone();
    let cfg_client_secret = cfg.client_secret.clone();
    let oauth_client = OAuthClient::new(cfg);

    println!("Opening browser to sign in to Google.");
    println!("If the browser does not open, visit this URL manually:");
    let tokens = login_interactive(&oauth_client, Duration::from_secs(300), "Google", |url| {
        println!("  {url}");
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("(Could not auto-open browser: {e})");
        }
    })
    .await
    .context("Google OAuth flow failed")?;

    let account_uuid = ids::new_local_uuid();
    calendar_blob_save(
        secrets,
        &account_uuid,
        &CalendarOAuthBlob {
            client_id: client_id.clone(),
            client_secret: cfg_client_secret.clone(),
            tokens: tokens.clone(),
        },
    )?;

    let http = GoogleClient::new();
    let identifier = match http.get_primary_calendar(&tokens.access_token).await {
        Ok(id) => id,
        Err(e) => {
            // Falls back to the list-based heuristic if the primary endpoint
            // fails (network blip, unusual permissions, etc.). Always
            // graceful — we'd rather show a slightly-wrong identifier than
            // refuse to add the account.
            tracing::warn!(error = %e, "calendars/primary failed; falling back to list");
            let cals = http.list_calendars(&tokens.access_token).await?;
            stint_core::calendar::google::resolve_account_identifier(&cals, &account_uuid)
        }
    };

    let account = CalendarAccount {
        id: account_uuid.clone(),
        provider: ProviderKind::Google,
        display_name: identifier.clone(),
        identifier,
        caldav_url: None,
        enabled: true,
        created_at: time::now_utc(),
    };
    cs.add_account(&account).await?;

    let provider = stint_core::calendar::google::build_provider_from_blob(secrets, &account_uuid)?;
    let n = refresh_account(cs, &account_uuid, provider.as_ref(), Ranges::on_add()).await?;
    println!(
        "Added Google account: {} ({account_uuid}). Fetched {n} events.",
        account.identifier
    );
    Ok(())
}

fn provider_label(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Google => "google",
    }
}
