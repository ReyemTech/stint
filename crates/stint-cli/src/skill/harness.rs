//! `Harness` trait — the abstraction every editor/agent integration implements.
//!
//! Each implementation knows how to (1) detect whether its host is installed,
//! (2) wire stint as an MCP server in the host's config file, and (3) drop a
//! skill or rules fragment into the host's expected location.

use anyhow::Result;
use serde::Serialize;
use std::path::PathBuf;

/// Outcome of a single install step (MCP server or skill file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallAction {
    /// Created the file or config entry from scratch.
    Installed,
    /// File/entry exists and matches the desired content — nothing to do.
    AlreadyUpToDate,
    /// File/entry existed but differed; overwrote with the canonical content.
    Updated,
    /// Skipped (e.g. dry-run or harness not installed).
    Skipped,
}

/// Snapshot of one harness's current install status.
#[derive(Debug, Clone, Serialize)]
pub struct HarnessStatus {
    pub name: &'static str,
    pub display: &'static str,
    pub detected: bool,
    pub mcp_installed: bool,
    pub skill_installed: bool,
    pub mcp_config_path: Option<PathBuf>,
    pub skill_path: Option<PathBuf>,
}

/// Abstraction for a single editor / agent harness (Claude Code, Codex, …).
pub trait Harness: Send + Sync {
    /// Stable lowercase identifier used on the command line (`claude`, `codex`, …).
    fn name(&self) -> &'static str;

    /// Human-readable label for picker UIs (`Claude Code`, `Codex`, …).
    fn display(&self) -> &'static str;

    /// True if the host appears to be installed (binary on PATH, config dir
    /// present, etc.). Used to flag suggestions in `stint skill status`.
    fn detect(&self) -> bool;

    /// Register stint as an MCP server in the harness's config file.
    fn install_mcp(&self, dry_run: bool) -> Result<InstallAction>;

    /// Drop the stint skill / AGENTS.md fragment into place.
    fn install_skill(&self, dry_run: bool) -> Result<InstallAction>;

    /// Remove both the MCP entry and the skill/fragment. Best-effort: missing
    /// pieces should not be treated as errors.
    fn uninstall(&self) -> Result<()>;

    /// Inspect on-disk state without mutating anything.
    fn status(&self) -> Result<HarnessStatus>;
}
