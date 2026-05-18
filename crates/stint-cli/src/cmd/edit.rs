use anyhow::Result;

#[derive(clap::Args)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
    anyhow::bail!("not implemented yet")
}
