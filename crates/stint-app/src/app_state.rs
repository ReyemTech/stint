use std::sync::Arc;
use stint_core::store::Store;

pub struct AppState {
    pub store: Arc<Store>,
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        stint_core::paths::ensure_data_dir()?;
        let db_path = stint_core::paths::database_path()?;
        let store = Store::connect(&db_path).await?;
        Ok(Self { store: Arc::new(store) })
    }
}
