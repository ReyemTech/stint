use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestEnv {
    pub dir: TempDir,
}

impl TestEnv {
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    pub fn db_path(&self) -> PathBuf {
        self.dir.path().join("stint.db")
    }
}
