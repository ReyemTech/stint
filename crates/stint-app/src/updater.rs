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

/// Dev-only preview escape hatch. When `STINT_DEV_VERSION_OVERRIDE` is set
/// to a valid semver string lower than the real running version,
/// `check_for_updates` short-circuits and returns synthetic
/// `available: true` data (pretending the override is the running version
/// and the real version is the upgrade target). Lets a maintainer see the
/// update affordances — About badge, popover indicator, Settings panel
/// state machine — without editing tauri.conf.json or shipping a fake
/// release.
///
/// Returns `None` if unset or unparseable, so production users (who never
/// set the env var) get the real network check every time.
///
/// `install_update` is NOT mocked — clicking Install while the override is
/// active will fail with "no update available" because the real updater
/// sees current_version == latest. That's the right behavior: don't let
/// the dev preview path execute a real install.
#[cfg(feature = "updater")]
fn dev_version_override() -> Option<semver::Version> {
    std::env::var("STINT_DEV_VERSION_OVERRIDE")
        .ok()
        .and_then(|s| s.parse().ok())
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle, channel: String) -> Result<UpdateInfo, String> {
    #[cfg(feature = "updater")]
    {
        use tauri_plugin_updater::UpdaterExt;

        if let Some(override_ver) = dev_version_override() {
            tracing::warn!(
                override_version = %override_ver,
                "check_for_updates: dev version override active — returning synthetic UpdateInfo"
            );
            let real_version = app.package_info().version.to_string();
            return Ok(UpdateInfo {
                available: true,
                current_version: override_ver.to_string(),
                latest_version: Some(real_version),
                notes: Some("see CHANGELOG.md".to_string()),
            });
        }

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

/// Download the new bundle and replace the on-disk app. Does NOT restart;
/// the running process keeps executing the old binary until the user clicks
/// "Restart Stint" (which calls `restart_app`) or quits and reopens the app
/// manually.
///
/// This split exists because tauri-plugin-updater's `download_and_install`
/// extracts the new bundle but never relaunches on macOS — the previous
/// single-call `apply_update` had no `app.restart()` so users clicked
/// "Install & restart" and saw nothing happen.
#[tauri::command]
pub async fn install_update(app: AppHandle, channel: String) -> Result<(), String> {
    #[cfg(feature = "updater")]
    {
        use tauri_plugin_updater::UpdaterExt;
        tracing::info!(channel = %channel, "install_update: starting");
        let endpoint = resolve_endpoint(Channel::from_setting(&channel));
        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint.parse().map_err(|e| format!("{e}"))?])
            .map_err(|e| e.to_string())?
            .build()
            .map_err(|e| e.to_string())?;
        let update = updater
            .check()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "install_update: check failed");
                e.to_string()
            })?
            .ok_or_else(|| {
                tracing::warn!("install_update: no update available at install time");
                "no update available".to_string()
            })?;
        let version = update.version.clone();
        tracing::info!(version = %version, "install_update: downloading + installing");
        update
            .download_and_install(|_, _| (), || ())
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "install_update: download_and_install failed");
                e.to_string()
            })?;
        tracing::info!(version = %version, "install_update: complete, awaiting restart");
        Ok(())
    }
    #[cfg(not(feature = "updater"))]
    {
        let _ = (app, channel);
        Err("updater disabled in this build".into())
    }
}

/// Terminate the running process so macOS relaunches the now-updated bundle.
/// Returns `()` because `AppHandle::restart` diverges — the call never
/// actually completes from the user's perspective.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    tracing::info!("restart_app: terminating to relaunch updated bundle");
    app.restart();
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
