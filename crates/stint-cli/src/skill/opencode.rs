//! OpenCode harness — stub for Task 23. Real implementation lands in Task 26.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::Result;

pub struct OpenCode;

impl Harness for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display(&self) -> &'static str {
        "OpenCode"
    }

    fn detect(&self) -> bool {
        which::which("opencode").is_ok()
            || dirs::config_dir()
                .map(|c| c.join("opencode").exists())
                .unwrap_or(false)
    }

    fn install_mcp(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("OpenCode MCP install — Task 26")
    }

    fn install_skill(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("OpenCode skill install — Task 26")
    }

    fn uninstall(&self) -> Result<()> {
        unimplemented!("OpenCode uninstall — Task 26")
    }

    fn status(&self) -> Result<HarnessStatus> {
        Ok(HarnessStatus {
            name: self.name(),
            display: self.display(),
            detected: self.detect(),
            mcp_installed: false,
            skill_installed: false,
            mcp_config_path: None,
            skill_path: None,
        })
    }
}
