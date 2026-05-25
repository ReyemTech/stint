//! Codex harness — stub for Task 23. Real implementation lands in Task 25.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::Result;

pub struct Codex;

impl Harness for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn display(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self) -> bool {
        which::which("codex").is_ok()
            || dirs::home_dir()
                .map(|h| h.join(".codex").exists())
                .unwrap_or(false)
    }

    fn install_mcp(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("Codex MCP install — Task 25")
    }

    fn install_skill(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("Codex skill install — Task 25")
    }

    fn uninstall(&self) -> Result<()> {
        unimplemented!("Codex uninstall — Task 25")
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
