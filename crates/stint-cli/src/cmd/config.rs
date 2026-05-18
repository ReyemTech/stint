use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ConfigCmd {
    Show,
}

pub async fn run(_c: ConfigCmd) -> Result<()> {
    anyhow::bail!("not implemented yet")
}
