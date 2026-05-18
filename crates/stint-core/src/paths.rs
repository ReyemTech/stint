use crate::Result;
use std::path::PathBuf;

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().ok_or_else(|| {
        crate::Error::Invariant("no data_dir available on this platform".into())
    })?;
    Ok(base.join("stint"))
}

pub fn database_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("stint.db"))
}

pub fn ensure_data_dir() -> Result<PathBuf> {
    let dir = data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
