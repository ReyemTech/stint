use std::path::Path;

use serde::Deserialize;

#[derive(Debug)]
pub struct ReleaseInfo {
    pub version: String,
    pub tarball_url: String,
    pub checksums_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// Fetch the latest stint release metadata from the GitHub Releases API.
///
/// `api_base` is the base URL (e.g. `https://api.github.com` in production,
/// or a wiremock server URI in tests). The function appends
/// `/repos/reyemtech/stint/releases/latest` and parses the response for the
/// universal macOS tarball plus the `checksums.txt` asset.
pub fn fetch_latest_release_blocking(api_base: &str) -> anyhow::Result<ReleaseInfo> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("stint/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let resp: GhRelease = client
        .get(format!("{api_base}/repos/reyemtech/stint/releases/latest"))
        .send()?
        .error_for_status()?
        .json()?;

    let version = resp.tag_name.trim_start_matches('v').to_string();
    let mut tarball_url = None;
    let mut checksums_url = None;
    for a in resp.assets {
        if a.name.starts_with("stint-") && a.name.ends_with("-universal-apple-darwin.tar.gz") {
            tarball_url = Some(a.browser_download_url);
        } else if a.name == "checksums.txt" {
            checksums_url = Some(a.browser_download_url);
        }
    }
    Ok(ReleaseInfo {
        version,
        tarball_url: tarball_url.ok_or_else(|| anyhow::anyhow!("tarball asset not found"))?,
        checksums_url: checksums_url.ok_or_else(|| anyhow::anyhow!("checksums.txt not found"))?,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub enum InstallMethod {
    /// Inside an `.app/Contents/MacOS/` bundle — managed by the GUI.
    AppBundled,
    /// Plain file in PATH (curl|sh CLI-only) — self-updatable.
    Standalone,
}

/// Detect how the running stint binary was installed, based on its resolved
/// executable path.
pub fn install_method(resolved_exe: &Path) -> InstallMethod {
    if resolved_exe
        .to_string_lossy()
        .contains(".app/Contents/MacOS/")
    {
        InstallMethod::AppBundled
    } else {
        InstallMethod::Standalone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_app_managed_install() {
        let p = PathBuf::from("/Applications/Stint.app/Contents/MacOS/stint");
        assert_eq!(install_method(&p), InstallMethod::AppBundled);
    }

    #[test]
    fn detects_standalone_install() {
        let p = PathBuf::from("/Users/alice/.local/bin/stint");
        assert_eq!(install_method(&p), InstallMethod::Standalone);

        let p2 = PathBuf::from("/usr/local/bin/stint");
        assert_eq!(install_method(&p2), InstallMethod::Standalone);
    }
}

#[cfg(test)]
mod release_api_tests {
    use super::*;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn fetch_latest_extracts_version_and_tarball_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/reyemtech/stint/releases/latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "tag_name": "v0.2.0",
                "assets": [
                    { "name": "Stint-0.2.0.dmg", "browser_download_url": "https://x/Stint-0.2.0.dmg" },
                    { "name": "stint-0.2.0-universal-apple-darwin.tar.gz", "browser_download_url": "https://x/stint-0.2.0-universal-apple-darwin.tar.gz" },
                    { "name": "checksums.txt", "browser_download_url": "https://x/checksums.txt" }
                ]
            })))
            .mount(&server)
            .await;

        let uri = server.uri();
        let info = tokio::task::spawn_blocking(move || fetch_latest_release_blocking(&uri))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.version, "0.2.0");
        assert_eq!(
            info.tarball_url,
            "https://x/stint-0.2.0-universal-apple-darwin.tar.gz"
        );
        assert_eq!(info.checksums_url, "https://x/checksums.txt");
    }
}
