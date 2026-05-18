pub mod config;
pub mod delete;
pub mod edit;
pub mod list;
pub mod projects;
pub mod start;
pub mod stop;
pub mod sync;
pub mod today;

use anyhow::Result;
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
