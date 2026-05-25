pub mod api;
pub mod calendar;
pub mod mcp;
pub mod config;
pub mod config_login;
pub mod delete;
pub mod edit;
pub mod list;
pub mod projects;
pub mod pull;
pub mod restart;
pub mod start;
pub mod stop;
pub mod sync;
pub mod today;
pub mod update;

use anyhow::Result;
use stint_core::recovery::{recover_on_startup, RecoveryDecision, StaleInfo};
use stint_core::{paths, store::Store};

/// Open the store at the default path, allowing override via STINT_DB env.
pub async fn open_store() -> Result<Store> {
    let path = match std::env::var("STINT_DB") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => {
            paths::ensure_data_dir()?;
            paths::database_path()?
        }
    };
    Ok(Store::connect(&path).await?)
}

pub async fn maybe_recover(store: &Store) -> Result<()> {
    let outcome = recover_on_startup(store, |info: StaleInfo| {
        eprintln!(
            "stint stopped at {} with timer still running ('{}', {}s elapsed).",
            info.last_heartbeat_at, info.description, info.age_secs,
        );
        eprintln!("(K)eep running, (S)top at last heartbeat, (D)iscard? [K/s/d]");
        let mut buf = String::new();
        if std::io::stdin().read_line(&mut buf).is_err() {
            return RecoveryDecision::Discard;
        }
        match buf.trim().to_ascii_lowercase().as_str() {
            "s" => RecoveryDecision::StopAtLastHeartbeat,
            "d" => RecoveryDecision::Discard,
            _ => RecoveryDecision::KeepRunning,
        }
    })
    .await?;
    let _ = outcome;
    Ok(())
}
