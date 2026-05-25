//! MCP server over stdio. Invoked via `stint mcp`. The MCP client (Claude
//! Code, Codex, OpenCode, …) spawns this as a child process; we exit when
//! the client closes stdin.
//!
//! See `docs/superpowers/specs/2026-05-23-stint-phase-6-deeper-integration-design.md#7-mcp-server`.

mod tools;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

pub async fn run() -> Result<()> {
    let store = crate::cmd::open_store().await?;
    let server = tools::StintServer::new(store);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
