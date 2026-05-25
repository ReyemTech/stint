//! Claude Code harness — stub for Task 23. Real implementation lands in Task 24.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::Result;

pub struct ClaudeCode;

impl Harness for ClaudeCode {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn display(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self) -> bool {
        which::which("claude").is_ok()
    }

    fn install_mcp(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("Claude Code MCP install — Task 24")
    }

    fn install_skill(&self, _dry_run: bool) -> Result<InstallAction> {
        unimplemented!("Claude Code skill install — Task 24")
    }

    fn uninstall(&self) -> Result<()> {
        unimplemented!("Claude Code uninstall — Task 24")
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
