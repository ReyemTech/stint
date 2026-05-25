//! Codex harness.
//!
//! - **MCP**: merges `[mcp_servers.stint]` into `~/.codex/config.toml` via
//!   `toml_edit`, preserving any existing keys/comments.
//! - **Skill**: appends or replaces a `<!-- stint:begin -->` … `<!-- stint:end -->`
//!   block in `~/.codex/AGENTS.md`. The markers let us update the fragment
//!   without disturbing the user's surrounding notes.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, Array, DocumentMut, Item, Table};

const AGENTS_FRAGMENT: &str = include_str!("../../skills/agents.md");
const BEGIN_MARKER: &str = "<!-- stint:begin -->";
const END_MARKER: &str = "<!-- stint:end -->";

pub struct Codex;

impl Codex {
    fn home() -> Result<PathBuf> {
        dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))
    }

    fn config_path() -> Result<PathBuf> {
        Ok(Self::home()?.join(".codex/config.toml"))
    }

    fn agents_path() -> Result<PathBuf> {
        Ok(Self::home()?.join(".codex/AGENTS.md"))
    }

    /// Backup `path` to `path.bak` if it exists.
    fn backup(path: &std::path::Path) -> Result<()> {
        if path.exists() {
            let backup = path.with_extension(
                format!(
                    "{}.bak",
                    path.extension().and_then(|s| s.to_str()).unwrap_or("")
                )
                .trim_start_matches('.'),
            );
            fs::copy(path, &backup)
                .with_context(|| format!("backing up {} to {}", path.display(), backup.display()))?;
        }
        Ok(())
    }

    fn desired_block() -> String {
        format!(
            "{BEGIN_MARKER}\n<!-- managed by `stint skill install codex` — edits between markers will be overwritten -->\n{}\n{END_MARKER}\n",
            AGENTS_FRAGMENT.trim_end()
        )
    }

    fn config_has_stint(doc: &DocumentMut) -> bool {
        doc.get("mcp_servers")
            .and_then(|i| i.as_table())
            .and_then(|t| t.get("stint"))
            .is_some()
    }
}

impl Harness for Codex {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn display(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self) -> bool {
        which::which("codex").is_ok() || Self::home().map(|h| h.join(".codex").exists()).unwrap_or(false)
    }

    fn install_mcp(&self, dry_run: bool) -> Result<InstallAction> {
        let path = Self::config_path()?;
        if dry_run {
            return Ok(InstallAction::Skipped);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let original = if path.exists() {
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
        } else {
            String::new()
        };

        let mut doc: DocumentMut = original
            .parse()
            .with_context(|| format!("parsing {} as TOML", path.display()))?;

        // Build the desired [mcp_servers.stint] sub-table.
        let mut server = Table::new();
        server["command"] = value("stint");
        let mut args = Array::new();
        args.push("mcp");
        server["args"] = value(args);

        // Ensure mcp_servers parent table exists.
        let servers = doc
            .entry("mcp_servers")
            .or_insert(Item::Table(Table::new()));
        let servers_tbl = servers
            .as_table_mut()
            .ok_or_else(|| anyhow!("`mcp_servers` exists but is not a table"))?;

        let already = servers_tbl
            .get("stint")
            .and_then(|i| i.as_table())
            .map(|existing| {
                existing
                    .get("command")
                    .and_then(|v| v.as_str())
                    == Some("stint")
                    && existing
                        .get("args")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .map(|v| v.as_str().unwrap_or_default().to_string())
                                .collect::<Vec<_>>()
                        })
                        == Some(vec!["mcp".to_string()])
            })
            .unwrap_or(false);

        if already {
            return Ok(InstallAction::AlreadyUpToDate);
        }

        let was_present = servers_tbl.contains_key("stint");
        servers_tbl.insert("stint", Item::Table(server));

        let new_contents = doc.to_string();
        if new_contents == original {
            return Ok(InstallAction::AlreadyUpToDate);
        }
        Self::backup(&path)?;
        fs::write(&path, new_contents)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(if was_present {
            InstallAction::Updated
        } else if original.is_empty() {
            InstallAction::Installed
        } else {
            InstallAction::Updated
        })
    }

    fn install_skill(&self, dry_run: bool) -> Result<InstallAction> {
        let path = Self::agents_path()?;
        if dry_run {
            return Ok(InstallAction::Skipped);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let original = if path.exists() {
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?
        } else {
            String::new()
        };
        let desired_block = Self::desired_block();

        let new_contents = replace_block(&original, &desired_block);
        if new_contents == original {
            return Ok(InstallAction::AlreadyUpToDate);
        }

        let was_present = original.contains(BEGIN_MARKER);
        Self::backup(&path)?;
        fs::write(&path, new_contents)
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(if !path_existed(&original) {
            InstallAction::Installed
        } else if was_present {
            InstallAction::Updated
        } else {
            InstallAction::Updated
        })
    }

    fn uninstall(&self) -> Result<()> {
        // Drop the [mcp_servers.stint] entry, leaving the parent table intact.
        let cfg_path = Self::config_path()?;
        if cfg_path.exists() {
            let original = fs::read_to_string(&cfg_path)?;
            if let Ok(mut doc) = original.parse::<DocumentMut>() {
                if Self::config_has_stint(&doc) {
                    if let Some(tbl) = doc
                        .get_mut("mcp_servers")
                        .and_then(|i| i.as_table_mut())
                    {
                        tbl.remove("stint");
                    }
                    Self::backup(&cfg_path)?;
                    fs::write(&cfg_path, doc.to_string())?;
                }
            }
        }
        // Strip the AGENTS.md fragment.
        let ag_path = Self::agents_path()?;
        if ag_path.exists() {
            let original = fs::read_to_string(&ag_path)?;
            let stripped = strip_block(&original);
            if stripped != original {
                Self::backup(&ag_path)?;
                if stripped.trim().is_empty() {
                    fs::remove_file(&ag_path)?;
                } else {
                    fs::write(&ag_path, stripped)?;
                }
            }
        }
        Ok(())
    }

    fn status(&self) -> Result<HarnessStatus> {
        let cfg_path = Self::config_path().ok();
        let ag_path = Self::agents_path().ok();
        let mcp_installed = cfg_path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| s.parse::<DocumentMut>().ok())
            .map(|d| Self::config_has_stint(&d))
            .unwrap_or(false);
        let skill_installed = ag_path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|s| s.contains(BEGIN_MARKER))
            .unwrap_or(false);
        Ok(HarnessStatus {
            name: self.name(),
            display: self.display(),
            detected: self.detect(),
            mcp_installed,
            skill_installed,
            mcp_config_path: cfg_path,
            skill_path: ag_path,
        })
    }
}

fn path_existed(original: &str) -> bool {
    !original.is_empty()
}

/// Public wrapper so sibling harnesses (OpenCode) can reuse the marker logic.
pub fn replace_block_public(haystack: &str, block: &str) -> String {
    replace_block(haystack, block)
}

/// Public wrapper for the inverse — used by sibling harnesses on uninstall.
pub fn strip_block_public(haystack: &str) -> String {
    strip_block(haystack)
}

/// Replace the `<!-- stint:begin -->`…`<!-- stint:end -->` block in `haystack`
/// with `block`. If no markers are present, append `block` (separated by a
/// blank line if the file is non-empty).
fn replace_block(haystack: &str, block: &str) -> String {
    if let (Some(start), Some(end)) = (haystack.find(BEGIN_MARKER), haystack.find(END_MARKER)) {
        if start < end {
            let end_full = end + END_MARKER.len();
            let mut out = String::new();
            out.push_str(&haystack[..start]);
            out.push_str(block.trim_end());
            // Preserve any content after the end marker, dropping a single
            // trailing newline that often follows the marker.
            let mut tail = &haystack[end_full..];
            if let Some(stripped) = tail.strip_prefix('\n') {
                tail = stripped;
            }
            if !tail.is_empty() {
                out.push('\n');
                out.push_str(tail);
            } else {
                out.push('\n');
            }
            return out;
        }
    }
    if haystack.is_empty() {
        block.to_string()
    } else if haystack.ends_with("\n\n") {
        format!("{haystack}{block}")
    } else if haystack.ends_with('\n') {
        format!("{haystack}\n{block}")
    } else {
        format!("{haystack}\n\n{block}")
    }
}

fn strip_block(haystack: &str) -> String {
    if let (Some(start), Some(end)) = (haystack.find(BEGIN_MARKER), haystack.find(END_MARKER)) {
        if start < end {
            let end_full = end + END_MARKER.len();
            let mut out = String::new();
            let head = &haystack[..start];
            // Trim a trailing blank line on the head so we don't leave a gap.
            let head = head.trim_end_matches('\n');
            out.push_str(head);
            let mut tail = &haystack[end_full..];
            if let Some(stripped) = tail.strip_prefix('\n') {
                tail = stripped;
            }
            if !tail.is_empty() {
                out.push('\n');
                out.push_str(tail);
            } else if !out.is_empty() {
                out.push('\n');
            }
            return out;
        }
    }
    haystack.to_string()
}
