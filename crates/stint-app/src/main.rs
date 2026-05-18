mod app_state;
mod commands;
mod tray;
mod windows;

use anyhow::Result;
use app_state::AppState;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("STINT_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let app_state = AppState::init().await?;

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .manage(RwLock::new(app_state))
        .setup(|app| {
            // Show main window for now; tray + popover wiring lands in Tasks 16-18.
            windows::show_main(&app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}
