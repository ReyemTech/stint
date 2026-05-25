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

pub async fn run(c: ConfigCmd, json: bool) -> Result<()> {
    let store = open_store().await?;
    let settings = Settings::new(store.clone());
    let secrets = Secrets::default();

    match c {
        ConfigCmd::Set { key, value } => {
            let stored_value = if SECRET_KEYS.contains(&key.as_str()) {
                let v = match value {
                    Some(v) => v,
                    None => rpassword::prompt_password(format!("{key}: "))?,
                };
                secrets.set(&key, &v)?;
                // Mask secrets in the JSON ack — the value was just written
                // to the Keychain; echoing it back defeats the point.
                "••••".to_string()
            } else {
                let v = value.ok_or_else(|| anyhow!("value required for {key}"))?;
                settings.set(&key, &v).await?;
                v
            };
            let ack = serde_json::json!({ "key": key, "value": stored_value });
            crate::render::render(&ack, json, |_| {
                if SECRET_KEYS.contains(&key.as_str()) {
                    println!("Saved {key} to Keychain.");
                } else {
                    println!("Saved {key}.");
                }
            });
            Ok(())
        }
        ConfigCmd::Show => {
            // Use a Vec<(k,v)> so the human path preserves the natural
            // ordering from list_prefixed; serde_json::Map without
            // `preserve_order` would shuffle keys.
            let mut entries: Vec<(String, String)> = settings.list_prefixed("").await?;
            for k in SECRET_KEYS {
                let present = secrets.get(k)?.is_some();
                entries.push((
                    (*k).to_string(),
                    if present { "••••" } else { "(unset)" }.to_string(),
                ));
            }
            if json {
                let payload: serde_json::Value = entries
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect::<serde_json::Map<_, _>>()
                    .into();
                crate::render::render(&payload, true, |_| {});
            } else {
                for (k, v) in &entries {
                    println!("{k} = {v}");
                }
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
            let identity = me.email.unwrap_or(me.id);
            let ack = serde_json::json!({ "connected": true, "identity": identity });
            crate::render::render(&ack, json, |_| {
                println!("✓ connected as {identity}");
            });
            Ok(())
        }
        // OAuth login/logout do interactive flows + structured prompts; the
        // useful "result" is the side-effect (Keychain blob written/cleared).
        // Emit a minimal ack in --json mode; otherwise fall through to the
        // existing implementation that prints its own progress.
        ConfigCmd::Login => {
            super::config_login::run_login(store).await?;
            if json {
                let ack = serde_json::json!({ "logged_in": true });
                crate::render::render(&ack, true, |_| {});
            }
            Ok(())
        }
        ConfigCmd::Logout => {
            super::config_login::run_logout(store).await?;
            if json {
                let ack = serde_json::json!({ "logged_out": true });
                crate::render::render(&ack, true, |_| {});
            }
            Ok(())
        }
    }
}
