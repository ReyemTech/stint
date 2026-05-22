use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

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

/// Parse a `checksums.txt` (shasum -a 256 format) and return the hex digest
/// for the requested filename. Each line is `<hash>  <filename>`.
pub fn parse_checksum_for(raw: &str, filename: &str) -> anyhow::Result<String> {
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        if name == filename {
            return Ok(hash.to_string());
        }
    }
    anyhow::bail!("checksum for {filename} not found in checksums.txt")
}

/// Compute the SHA-256 of `path` and compare to `expected_hex` (lowercase).
pub fn verify_sha256(path: &Path, expected_hex: &str) -> anyhow::Result<()> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    io::copy(&mut f, &mut hasher)?;
    let got = format!("{:x}", hasher.finalize());
    if got != expected_hex {
        anyhow::bail!("checksum mismatch: expected {expected_hex}, got {got}");
    }
    Ok(())
}

/// Atomically swap `staging` into `target` using `rename(2)`. On macOS this
/// preserves the inode of any process that already has the old binary open,
/// which is what lets us replace the running CLI binary safely.
pub fn atomic_replace(staging: &Path, target: &Path) -> anyhow::Result<()> {
    // Ensure the staged file has executable permissions before swap.
    let mut perms = fs::metadata(staging)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(staging, perms)?;
    fs::rename(staging, target)?;
    Ok(())
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

#[cfg(test)]
mod checksum_tests {
    use super::*;

    #[test]
    fn parses_checksums_txt_format() {
        // Standard shasum -a 256 output: "<hash>  <filename>"
        let raw = "\
abc123def456abc123def456abc123def456abc123def456abc123def456abc1  Stint-0.2.0.dmg
def456abc123def456abc123def456abc123def456abc123def456abc123def4  stint-0.2.0-universal-apple-darwin.tar.gz
";
        let want = "stint-0.2.0-universal-apple-darwin.tar.gz";
        let got = parse_checksum_for(raw, want).unwrap();
        assert_eq!(
            got,
            "def456abc123def456abc123def456abc123def456abc123def456abc123def4"
        );
    }

    #[test]
    fn verify_sha256_matches() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello world").unwrap();
        // sha256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        verify_sha256(
            tmp.path(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
        )
        .unwrap();
    }

    #[test]
    fn verify_sha256_mismatch() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello").unwrap();
        let err = verify_sha256(tmp.path(), "0000").unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"));
    }

    #[test]
    fn atomic_replace_preserves_running_binary() {
        // Simulates the macOS inode-swap semantics. Write "old", swap to "new",
        // ensure both old (via held fd) and new (via path) are readable.
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("stint");
        std::fs::write(&target, b"old").unwrap();
        let opened_fd = std::fs::File::open(&target).unwrap();

        let staging = tmp.path().join("stint.new");
        std::fs::write(&staging, b"new").unwrap();
        atomic_replace(&staging, &target).unwrap();

        // The opened fd still sees "old" (its inode is preserved).
        let mut buf = String::new();
        use std::io::Read;
        let mut fd = opened_fd;
        fd.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "old");

        // A fresh open sees "new".
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }
}
