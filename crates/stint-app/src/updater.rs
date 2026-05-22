use serde::Serialize;
use tauri::AppHandle;

#[cfg(feature = "updater")]
use crate::updater_endpoint::{resolve_endpoint, Channel};

#[derive(Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle, channel: String) -> Result<UpdateInfo, String> {
    #[cfg(feature = "updater")]
    {
        use tauri_plugin_updater::UpdaterExt;
        let endpoint = resolve_endpoint(Channel::from_setting(&channel));
        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint.parse().map_err(|e| format!("{e}"))?])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;

        match updater.check().await {
            Ok(Some(update)) => Ok(UpdateInfo {
                available: true,
                current_version: app.package_info().version.to_string(),
                latest_version: Some(update.version.clone()),
                notes: update.body.clone(),
            }),
            Ok(None) => Ok(UpdateInfo {
                available: false,
                current_version: app.package_info().version.to_string(),
                latest_version: None,
                notes: None,
            }),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(feature = "updater"))]
    {
        let _ = (app, channel);
        Err("updater disabled in this build".into())
    }
}

#[tauri::command]
pub async fn apply_update(app: AppHandle, channel: String) -> Result<(), String> {
    #[cfg(feature = "updater")]
    {
        use tauri_plugin_updater::UpdaterExt;
        let endpoint = resolve_endpoint(Channel::from_setting(&channel));
        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint.parse().map_err(|e| format!("{e}"))?])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
        if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
            update
                .download_and_install(|_, _| (), || ())
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(not(feature = "updater"))]
    {
        let _ = (app, channel);
        Err("updater disabled in this build".into())
    }
}

#[cfg(test)]
mod tests {
    use crate::updater_endpoint::Channel;

    #[test]
    fn channel_roundtrip() {
        assert_eq!(Channel::from_setting("stable").as_setting(), "stable");
        assert_eq!(Channel::from_setting("beta").as_setting(), "beta");
        assert_eq!(Channel::from_setting("garbage").as_setting(), "stable");
    }
}
