use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use std::sync::Arc;
use std::time::Duration;
use stint_core::calendar::google::client::GoogleClient;
use stint_core::calendar::google::config::google_oauth_config;
use stint_core::calendar::google::GoogleProvider;
use stint_core::calendar::store::{
    calendar_blob_delete, calendar_blob_load, calendar_blob_save, CalendarOAuthBlob, CalendarStore,
};
use stint_core::calendar::sync::{refresh_account, Ranges};
use stint_core::calendar::types::{CalendarAccount, ProviderKind};
use stint_core::config::secrets::Secrets;
use stint_core::ids;
use stint_core::oauth::client::OAuthClient;
use stint_core::solidtime::auth::{
    login_interactive, OAuthTokenProvider, PersistFn, TokenProvider,
};
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
    /// List or toggle calendars for an account.
    Calendars {
        account_id: String,
        /// Calendar id to include.
        #[arg(long)]
        include: Option<String>,
        /// Calendar id to exclude.
        #[arg(long)]
        exclude: Option<String>,
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
        } => {
            if let Some(id) = include {
                cs.set_calendar_included(&id, true).await?;
                println!("Included calendar {id}.");
            }
            if let Some(id) = exclude {
                cs.set_calendar_included(&id, false).await?;
                println!("Excluded calendar {id}.");
            }
            for c in cs.list_calendars(&account_id).await? {
                let mark = if c.included { "[x]" } else { "[ ]" };
                println!("{mark} {} {}", c.id, c.name);
            }
            Ok(())
        }
        CalendarCmd::Refresh { account_id } => {
            let provider = build_google_provider_cli(&secrets, &account_id)?;
            let n =
                refresh_account(&cs, &account_id, provider.as_ref(), Ranges::on_focus()).await?;
            println!("Refreshed {n} events.");
            Ok(())
        }
    }
}

async fn add_google(cs: &CalendarStore, secrets: &Secrets) -> Result<()> {
    let cfg = google_oauth_config();
    let client_id = cfg.client_id.clone();
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
            tokens: tokens.clone(),
        },
    )?;

    let http = GoogleClient::new();
    let cals = http.list_calendars(&tokens.access_token).await?;
    let identifier = cals
        .iter()
        .find(|c| c.id == "primary")
        .map(|c| c.name.clone())
        .or_else(|| cals.first().map(|c| c.id.clone()))
        .unwrap_or_else(|| account_uuid.clone());

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

    let provider = build_google_provider_cli(secrets, &account_uuid)?;
    let n = refresh_account(cs, &account_uuid, provider.as_ref(), Ranges::on_add()).await?;
    println!(
        "Added Google account: {} ({account_uuid}). Fetched {n} events.",
        account.identifier
    );
    Ok(())
}

fn build_google_provider_cli(
    secrets: &Secrets,
    account_id: &str,
) -> Result<Box<dyn stint_core::calendar::provider::CalendarProvider>> {
    let blob = calendar_blob_load(secrets, account_id)?
        .ok_or_else(|| anyhow!("no OAuth credentials for account {account_id}"))?;
    let mut cfg = google_oauth_config();
    cfg.client_id = blob.client_id.clone();
    let oauth_client = OAuthClient::new(cfg);

    let secrets_clone = secrets.clone();
    let account_owned = account_id.to_string();
    let client_id_owned = blob.client_id.clone();
    let persist: PersistFn = Box::new(move |tokens| {
        let updated = CalendarOAuthBlob {
            client_id: client_id_owned.clone(),
            tokens: tokens.clone(),
        };
        calendar_blob_save(&secrets_clone, &account_owned, &updated)
    });

    let provider: Arc<dyn TokenProvider> =
        Arc::new(OAuthTokenProvider::new(oauth_client, blob.tokens, persist));
    Ok(Box::new(GoogleProvider::new(provider, GoogleClient::new())))
}

fn provider_label(p: ProviderKind) -> &'static str {
    match p {
        ProviderKind::Google => "google",
    }
}
