//! OpenCode harness.
//!
//! - **MCP**: merges `mcp.stint = { type, command, enabled }` into
//!   `~/.config/opencode/opencode.json` (preserves the rest of the JSON).
//! - **Skill**: writes the embedded SKILL.md to
//!   `~/.config/opencode/skills/stint/SKILL.md`, idempotent via byte
//!   comparison, with a `.bak` backup on overwrite. OpenCode reads agent
//!   skills from `<config-dir>/opencode/skills/<name>/SKILL.md`.

use crate::skill::harness::{Harness, HarnessStatus, InstallAction};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::PathBuf;

/// Canonical skill content shipped with stint.
const SKILL_CONTENT: &str = include_str!("../../skills/stint/SKILL.md");

pub struct OpenCode;

impl OpenCode {
    fn config_dir() -> Result<PathBuf> {
        Ok(dirs::config_dir()
            .ok_or_else(|| anyhow!("could not determine config directory"))?
            .join("opencode"))
    }

    fn mcp_config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("opencode.json"))
    }

    fn skill_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("skills/stint/SKILL.md"))
    }

    fn backup(path: &std::path::Path) -> Result<()> {
        if path.exists() {
            let backup = path.with_extension(format!(
                "{}.bak",
                path.extension().and_then(|s| s.to_str()).unwrap_or("")
            ));
            fs::copy(path, &backup).with_context(|| {
                format!("backing up {} to {}", path.display(), backup.display())
            })?;
        }
        Ok(())
    }

    fn desired_stint_entry() -> Value {
        json!({
            "type": "local",
            "command": ["stint", "mcp"],
            "enabled": true,
        })
    }
}

impl Harness for OpenCode {
    fn name(&self) -> &'static str {
        "opencode"
    }

    fn display(&self) -> &'static str {
        "OpenCode"
    }

    fn detect(&self) -> bool {
        which::which("opencode").is_ok() || Self::config_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn install_mcp(&self, dry_run: bool) -> Result<InstallAction> {
        let path = Self::mcp_config_path()?;
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

        let mut doc: Value = if original.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&original)
                .with_context(|| format!("parsing {} as JSON", path.display()))?
        };

        let root = doc
            .as_object_mut()
            .ok_or_else(|| anyhow!("opencode.json must be a JSON object"))?;

        let mcp_entry = root
            .entry("mcp".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let mcp_map = mcp_entry
            .as_object_mut()
            .ok_or_else(|| anyhow!("`mcp` exists but is not an object"))?;

        let desired = Self::desired_stint_entry();
        if mcp_map.get("stint") == Some(&desired) {
            return Ok(InstallAction::AlreadyUpToDate);
        }
        mcp_map.insert("stint".to_string(), desired);

        let new_contents = serde_json::to_string_pretty(&doc)? + "\n";
        Self::backup(&path)?;
        fs::write(&path, &new_contents).with_context(|| format!("writing {}", path.display()))?;
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
        let cfg_path = Self::mcp_config_path()?;
        if cfg_path.exists() {
            let original = fs::read_to_string(&cfg_path)?;
            if let Ok(mut doc) = serde_json::from_str::<Value>(&original) {
                let mut mutated = false;
                if let Some(root) = doc.as_object_mut() {
                    if let Some(mcp) = root.get_mut("mcp").and_then(|v| v.as_object_mut()) {
                        if mcp.remove("stint").is_some() {
                            mutated = true;
                        }
                    }
                }
                if mutated {
                    Self::backup(&cfg_path)?;
                    fs::write(&cfg_path, serde_json::to_string_pretty(&doc)? + "\n")?;
                }
            }
        }
        let skill_path = Self::skill_path()?;
        if skill_path.exists() {
            fs::remove_file(&skill_path)
                .with_context(|| format!("removing {}", skill_path.display()))?;
        }
        if let Some(parent) = skill_path.parent() {
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }

    fn status(&self) -> Result<HarnessStatus> {
        let cfg_path = Self::mcp_config_path().ok();
        let skill_path = Self::skill_path().ok();
        let mcp_installed = cfg_path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .and_then(|v| v.get("mcp").and_then(|m| m.get("stint")).map(|_| true))
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
