use std::sync::Arc;
use stint_core::store::Store;
use tokio::sync::RwLock;

pub struct AppState {
    pub store: Arc<Store>,
    /// Port the loopback HTTP API actually bound to this session. `None` when
    /// the API is disabled or the server hasn't completed its bind yet. Set
    /// by `http::maybe_spawn` after a successful `TcpListener::bind`.
    pub http_api_port: Arc<RwLock<Option<u16>>>,
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        stint_core::paths::ensure_data_dir()?;
        let db_path = stint_core::paths::database_path()?;
        let store = Store::connect(&db_path).await?;
        Ok(Self {
            store: Arc::new(store),
            http_api_port: Arc::new(RwLock::new(None)),
        })
    }
}
