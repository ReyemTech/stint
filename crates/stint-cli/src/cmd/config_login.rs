use anyhow::{anyhow, Context, Result};
use std::time::Duration;
use stint_core::config::secrets::Secrets;
use stint_core::config::Settings;
use stint_core::oauth::client::{OAuthClient, OAuthConfig};
use stint_core::solidtime::auth::{
    login_interactive, oauth_blob_delete, oauth_blob_save, OAuthBlob,
};
use stint_core::store::Store;

const FLOW_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

pub async fn run_login(store: Store) -> Result<()> {
    let settings = Settings::new(store.clone());
    let base_url = settings.get("solidtime.url").await?.ok_or_else(|| {
        anyhow!("solidtime.url is not set; run `stint config set solidtime.url <URL>` first")
    })?;

    let client_id = match settings.get("solidtime.oauth.client_id").await? {
        Some(id) => id,
        None => {
            eprintln!("solidtime.oauth.client_id is not set.");
            eprintln!(
                "Register an OAuth client on your Solidtime instance (see README), then run:"
            );
            eprintln!("  stint config set solidtime.oauth.client_id <CLIENT-ID>");
            return Err(anyhow!("missing OAuth client ID"));
        }
    };

    let secrets = Secrets::default();
    let client = OAuthClient::new(OAuthConfig {
        authorize_url: format!("{}/oauth/authorize", base_url.trim_end_matches('/')),
        token_url: format!("{}/oauth/token", base_url.trim_end_matches('/')),
        client_id: client_id.clone(),
        redirect_uri: "http://127.0.0.1:0/callback".into(),
        // Solidtime/Passport may reject explicit scopes if `Passport::tokensCan`
        // is not configured (scope enforcement is documented as not yet
        // implemented). Send an empty scope list — the server falls back to
        // whatever default it has configured.
        scopes: vec![],
    });

    println!("Opening browser to sign in to {base_url}.");
    println!("If the browser does not open, visit this URL manually:");
    let tokens = login_interactive(&client, FLOW_TIMEOUT, |url| {
        println!("  {url}");
        if let Err(e) = webbrowser::open(&url) {
            eprintln!("(Could not auto-open browser: {e})");
        }
    })
    .await
    .context("OAuth flow failed")?;

    let blob = OAuthBlob { client_id, tokens };
    oauth_blob_save(&secrets, &blob).context("persist OAuth blob")?;
    settings.set("solidtime.auth_mode", "oauth").await?;
    println!("Signed in. solidtime.auth_mode is now 'oauth'.");
    Ok(())
}

pub async fn run_logout(store: Store) -> Result<()> {
    let settings = Settings::new(store);
    let secrets = Secrets::default();
    oauth_blob_delete(&secrets).context("delete OAuth blob")?;

    // If a PAT exists, fall back to it. Otherwise leave auth_mode as-is so the
    // user knows they need to re-authenticate.
    if secrets.get("solidtime.token")?.is_some() {
        settings.set("solidtime.auth_mode", "api_token").await?;
        println!("OAuth tokens cleared. Falling back to the stored API token.");
    } else {
        println!("OAuth tokens cleared. Run `stint config set solidtime.token` or `stint config login` to re-authenticate.");
    }
    Ok(())
}
