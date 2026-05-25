//! `stint mcp` — run the MCP server on stdio. Task 20.

use anyhow::Result;

#[derive(clap::Args)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
    crate::mcp::run().await
}
