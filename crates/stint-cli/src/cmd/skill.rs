//! `stint skill` — install / uninstall / status across editor harnesses.
//!
//! Routes to a concrete [`stint_cli::skill::harness::Harness`] implementation
//! based on the `<harness>` argument (or the interactive picker when omitted).

use anyhow::{anyhow, Result};
use stint_cli::skill;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Install stint's MCP server + skill/rules file into the given harness.
    Install {
        /// Harness name (claude, codex, opencode). Omit for interactive picker.
        harness: Option<String>,
        /// Print what would happen without modifying files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Uninstall stint from the given harness.
    Uninstall {
        /// Harness name (claude, codex, opencode). Omit for interactive picker.
        harness: Option<String>,
    },
    /// Show which harnesses have stint installed.
    Status,
}

pub async fn run(cmd: Command, json: bool) -> Result<()> {
    match cmd {
        Command::Install { harness, dry_run } => {
            let h = resolve_harness(harness)?;
            let mcp = h.install_mcp(dry_run)?;
            let skill_action = h.install_skill(dry_run)?;
            crate::render::render(
                &serde_json::json!({
                    "harness": h.name(),
                    "mcp": mcp,
                    "skill": skill_action,
                    "dry_run": dry_run,
                }),
                json,
                |_| {
                    println!("Installed stint for {}", h.display());
                    println!("  MCP:   {mcp:?}");
                    println!("  Skill: {skill_action:?}");
                    if dry_run {
                        println!("  (dry run — no files modified)");
                    }
                },
            );
        }
        Command::Uninstall { harness } => {
            let h = resolve_harness(harness)?;
            h.uninstall()?;
            crate::render::render(
                &serde_json::json!({"uninstalled": true, "harness": h.name()}),
                json,
                |_| println!("Uninstalled stint from {}", h.display()),
            );
        }
        Command::Status => {
            let rows: Result<Vec<_>> = skill::all_harnesses().iter().map(|h| h.status()).collect();
            let rows = rows?;
            crate::render::render(&rows, json, |rs| {
                for r in rs {
                    println!(
                        "{:14}  detected={:5}  mcp={:5}  skill={:5}",
                        r.display, r.detected, r.mcp_installed, r.skill_installed
                    );
                }
            });
        }
    }
    Ok(())
}

fn resolve_harness(name: Option<String>) -> Result<Box<dyn skill::harness::Harness>> {
    match name {
        Some(n) => skill::find(&n).ok_or_else(|| anyhow!("unknown harness: {n}")),
        None => skill::picker::pick(),
    }
}
