//! Codex harness.
//!
//! - **MCP**: merges `[mcp_servers.stint]` into `~/.codex/config.toml` via
//!   `toml_edit`, preserving any existing keys/comments.
//! - **Skill**: writes the embedded SKILL.md to
//!   `~/.agents/skills/stint/SKILL.md`, idempotent via byte comparison,
//!   with a `.bak` backup on overwrite. Codex reads agent skills from
//!   `~/.agents/skills/<name>/SKILL.md`.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, Array, DocumentMut, Item, Table};

/// Canonical skill content shipped with stint.
const SKILL_CONTENT: &str = include_str!("../../skills/stint/SKILL.md");

pub struct Codex;

impl Codex {
    fn home() -> Result<PathBuf> {
        dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))
    }

    fn config_path() -> Result<PathBuf> {
        Ok(Self::home()?.join(".codex/config.toml"))
    }

    fn skill_path() -> Result<PathBuf> {
        Ok(Self::home()?.join(".agents/skills/stint/SKILL.md"))
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
            fs::copy(path, &backup).with_context(|| {
                format!("backing up {} to {}", path.display(), backup.display())
            })?;
        }
        Ok(())
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
        which::which("codex").is_ok()
            || Self::home()
                .map(|h| h.join(".codex").exists())
                .unwrap_or(false)
    }

    fn install_mcp(&self, dry_run: bool) -> Result<InstallAction> {
        let path = Self::config_path()?;
        if dry_run {
            return Ok(InstallAction::Skipped);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
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
                existing.get("command").and_then(|v| v.as_str()) == Some("stint")
                    && existing.get("args").and_then(|v| v.as_array()).map(|a| {
                        a.iter()
                            .map(|v| v.as_str().unwrap_or_default().to_string())
                            .collect::<Vec<_>>()
                    }) == Some(vec!["mcp".to_string()])
            })
            .unwrap_or(false);

        if already {
            return Ok(InstallAction::AlreadyUpToDate);
        }

        servers_tbl.insert("stint", Item::Table(server));

        let new_contents = doc.to_string();
        if new_contents == original {
            return Ok(InstallAction::AlreadyUpToDate);
        }
        Self::backup(&path)?;
        fs::write(&path, new_contents).with_context(|| format!("writing {}", path.display()))?;
        Ok(if original.is_empty() {
            InstallAction::Installed
        } else {
            InstallAction::Updated
        })
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
        // Drop the [mcp_servers.stint] entry, leaving the parent table intact.
        let cfg_path = Self::config_path()?;
        if cfg_path.exists() {
            let original = fs::read_to_string(&cfg_path)?;
            if let Ok(mut doc) = original.parse::<DocumentMut>() {
                if Self::config_has_stint(&doc) {
                    if let Some(tbl) = doc.get_mut("mcp_servers").and_then(|i| i.as_table_mut()) {
                        tbl.remove("stint");
                    }
                    Self::backup(&cfg_path)?;
                    fs::write(&cfg_path, doc.to_string())?;
                }
            }
        }
        // Skill file: remove if present.
        let skill_path = Self::skill_path()?;
        if skill_path.exists() {
            fs::remove_file(&skill_path)
                .with_context(|| format!("removing {}", skill_path.display()))?;
        }
        // Also try to drop the parent dir if it became empty.
        if let Some(parent) = skill_path.parent() {
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }

    fn status(&self) -> Result<HarnessStatus> {
        let cfg_path = Self::config_path().ok();
        let skill_path = Self::skill_path().ok();
        let mcp_installed = cfg_path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| s.parse::<DocumentMut>().ok())
            .map(|d| Self::config_has_stint(&d))
            .unwrap_or(false);
        let skill_installed = skill_path.as_ref().map(|p| p.exists()).unwrap_or(false);
        Ok(HarnessStatus {
            name: self.name(),
            display: self.display(),
            detected: self.detect(),
            mcp_installed,
            skill_installed,
            mcp_config_path: cfg_path,
            skill_path,
        })
    }
}
