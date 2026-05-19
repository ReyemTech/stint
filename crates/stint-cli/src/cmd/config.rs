use anyhow::{anyhow, Context, Result};
use clap::Subcommand;
use stint_core::config::{secrets::Secrets, Settings};
use stint_core::solidtime::auth::build_token_provider;
use stint_core::solidtime::SolidtimeClient;

use super::open_store;

#[derive(Subcommand)]
pub enum ConfigCmd {
    /// Set a configuration value (use `solidtime.token` to prompt for a secret).
    Set { key: String, value: Option<String> },
    /// Show all non-secret settings; tokens are masked.
    Show,
    /// Verify that the configured Solidtime URL + token work.
    Test,
    /// Run OAuth 2.0 PKCE login against the configured Solidtime instance.
    Login,
    /// Remove the OAuth token blob from Keychain.
    Logout,
}

const SECRET_KEYS: &[&str] = &["solidtime.token"];

pub async fn run(c: ConfigCmd) -> Result<()> {
    let store = open_store().await?;
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();

    match c {
        ConfigCmd::Set { key, value } => {
            if SECRET_KEYS.contains(&key.as_str()) {
                let v = match value {
                    Some(v) => v,
                    None => rpassword::prompt_password(format!("{key}: "))?,
                };
                secrets.set(&key, &v)?;
                println!("Saved {key} to Keychain.");
            } else {
                let v = value.ok_or_else(|| anyhow!("value required for {key}"))?;
                settings.set(&key, &v).await?;
                println!("Saved {key}.");
            }
            Ok(())
        }
        ConfigCmd::Show => {
            for (k, v) in settings.list_prefixed("").await? {
                println!("{k} = {v}");
            }
            for k in SECRET_KEYS {
                let present = secrets.get(k)?.is_some();
                println!("{k} = {}", if present { "••••" } else { "(unset)" });
            }
            Ok(())
        }
        ConfigCmd::Test => {
            let url = settings
                .get("solidtime.url")
                .await?
                .ok_or_else(|| anyhow!("solidtime.url not set"))?;
            let (provider, _oauth_client) = build_token_provider(&settings, &secrets, &url).await?;
            let client = SolidtimeClient::new(&url, provider);
            let me = client
                .test_connection()
                .await
                .context("solidtime ping failed")?;
            println!("✓ connected as {}", me.email.unwrap_or(me.id));
            Ok(())
        }
        ConfigCmd::Login => super::config_login::run_login(store).await,
        ConfigCmd::Logout => super::config_login::run_logout(store).await,
    }
}
