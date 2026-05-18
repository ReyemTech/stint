use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProjectsCmd {
    List,
}

pub async fn run(_p: ProjectsCmd) -> Result<()> {
    anyhow::bail!("not implemented yet")
}
