//! Claude Code harness.
//!
//! - **MCP**: shells out to `claude mcp add stint --scope user -- stint mcp`.
//!   This is best-effort; if the `claude` binary is missing we mark
//!   [`InstallAction::Skipped`] rather than fail the entire install (the
//!   skill file is still useful on its own).
//! - **Skill**: writes the embedded SKILL.md to
//!   `~/.claude/skills/stint/SKILL.md`, idempotent via byte comparison,
//!   with a `.bak` backup on overwrite.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Canonical skill content shipped with stint.
const SKILL_CONTENT: &str = include_str!("../../skills/stint/SKILL.md");

pub struct ClaudeCode;

impl ClaudeCode {
    fn skill_path() -> Result<PathBuf> {
        Ok(dirs::home_dir()
            .ok_or_else(|| anyhow!("could not determine home directory"))?
            .join(".claude/skills/stint/SKILL.md"))
    }

    /// Heuristic: a managed stint MCP server is "installed" if a Claude Code
    /// MCP config file references it. Locations differ across Claude Code
    /// versions, so we check the common ones.
    fn mcp_installed() -> bool {
        let Some(home) = dirs::home_dir() else {
            return false;
        };
        for candidate in [
            home.join(".claude.json"),
            home.join(".claude/mcp.json"),
            home.join(".config/claude/mcp.json"),
        ] {
            if let Ok(s) = fs::read_to_string(&candidate) {
                if s.contains("\"stint\"") {
                    return true;
                }
            }
        }
        false
    }
}

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

    fn install_mcp(&self, dry_run: bool) -> Result<InstallAction> {
        if dry_run {
            return Ok(InstallAction::Skipped);
        }
        if which::which("claude").is_err() {
            // No claude binary on PATH — skill file is still useful.
            return Ok(InstallAction::Skipped);
        }
        if Self::mcp_installed() {
            return Ok(InstallAction::AlreadyUpToDate);
        }
        let status = Command::new("claude")
            .args([
                "mcp", "add", "stint", "--scope", "user", "--", "stint", "mcp",
            ])
            .status()
            .context("failed to invoke `claude mcp add`")?;
        if !status.success() {
            return Err(anyhow!(
                "`claude mcp add stint` exited with status {status}"
            ));
        }
        Ok(InstallAction::Installed)
    }

    fn install_skill(&self, dry_run: bool) -> Result<InstallAction> {
        let path = Self::skill_path()?;
        if dry_run {
            return Ok(InstallAction::Skipped);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        if path.exists() {
            let existing =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if existing == SKILL_CONTENT {
                return Ok(InstallAction::AlreadyUpToDate);
            }
            // Backup before overwrite.
            let backup = path.with_extension("md.bak");
            fs::copy(&path, &backup)
                .with_context(|| format!("backing up to {}", backup.display()))?;
            fs::write(&path, SKILL_CONTENT)
                .with_context(|| format!("writing {}", path.display()))?;
            return Ok(InstallAction::Updated);
        }
        fs::write(&path, SKILL_CONTENT).with_context(|| format!("writing {}", path.display()))?;
        Ok(InstallAction::Installed)
    }

    fn uninstall(&self) -> Result<()> {
        // Skill file: remove if present.
        let path = Self::skill_path()?;
        if path.exists() {
            fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        }
        // Also try to drop the parent dir if it became empty.
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
        // MCP: best-effort; if `claude` is missing, skip silently.
        if which::which("claude").is_ok() {
            let _ = Command::new("claude")
                .args(["mcp", "remove", "stint", "--scope", "user"])
                .status();
        }
        Ok(())
    }

    fn status(&self) -> Result<HarnessStatus> {
        let skill_path = Self::skill_path().ok();
        let skill_installed = skill_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        Ok(HarnessStatus {
            name: self.name(),
            display: self.display(),
            detected: self.detect(),
            mcp_installed: Self::mcp_installed(),
            skill_installed,
            mcp_config_path: dirs::home_dir().map(|h| h.join(".claude.json")),
            skill_path,
        })
    }
}
