//! `stint api info` — report whether the loopback HTTP API is enabled
//! and where it's bound.
//!
//! The CLI does not start the HTTP server itself; that's the GUI's job. This
//! command only reads the persisted `api.enabled` / `api.host` / `api.port`
//! settings so consumers can discover the base URL.

use anyhow::Result;
use serde::Serialize;
use stint_core::config::{
    Settings, DEFAULT_API_HOST, KEY_API_ENABLED, KEY_API_HOST, KEY_API_PORT,
};

use super::open_store;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Report whether the HTTP API is enabled and where it's bound.
    Info,
}

#[derive(Serialize)]
struct Info {
    enabled: bool,
    host: String,
    port: Option<u16>,
    base_url: Option<String>,
}

pub async fn run(cmd: Command, json: bool) -> Result<()> {
    let store = open_store().await?;
    let s = Settings::new(store);
    match cmd {
        Command::Info => {
            let enabled = s.get(KEY_API_ENABLED).await?.as_deref() == Some("true");
            let host = s
                .get(KEY_API_HOST)
                .await?
                .unwrap_or_else(|| DEFAULT_API_HOST.into());
            let port: Option<u16> = s.get(KEY_API_PORT).await?.and_then(|p| p.parse().ok());
            let base_url = port.map(|p| format!("http://{host}:{p}"));
            let info = Info {
                enabled,
                host,
                port,
                base_url,
            };
            crate::render::render(&info, json, |i| {
                println!(
                    "enabled: {}\nhost:    {}\nport:    {}\nurl:     {}",
                    i.enabled,
                    i.host,
                    i.port
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| "-".into()),
                    i.base_url.as_deref().unwrap_or("-"),
                );
            });
            Ok(())
        }
    }
}
