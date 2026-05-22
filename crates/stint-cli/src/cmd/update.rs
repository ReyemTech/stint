use std::path::Path;

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
