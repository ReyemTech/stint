use anyhow::{Context, Result};
use clap::CommandFactory;
use std::path::PathBuf;

pub fn run(out_dir: PathBuf) -> Result<()> {
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let cmd = crate::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    let mut buf: Vec<u8> = Default::default();
    man.render(&mut buf)?;

    let path = out_dir.join("stint.1");
    std::fs::write(&path, &buf).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
