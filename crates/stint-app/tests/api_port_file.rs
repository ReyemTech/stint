//! `api.port` file is written on bind, removed on drop.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvRestore {
    previous: Option<std::ffi::OsString>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("STINT_DATA_DIR", value),
            None => std::env::remove_var("STINT_DATA_DIR"),
        }
    }
}

fn port_file_for(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("api.port")
}

#[test]
fn writes_port_file_on_bind() {
    let tempdir = TempDir::new().unwrap();
    let _guard = env_lock().lock().unwrap();
    let _restore = EnvRestore {
        previous: std::env::var_os("STINT_DATA_DIR"),
    };
    std::env::set_var("STINT_DATA_DIR", tempdir.path());

    let port = stint_app::http::write_port_file_for_test(49792).unwrap();
    assert_eq!(port, 49792);
    let path = port_file_for(tempdir.path());
    assert!(path.exists(), "port file not at {}", path.display());
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(contents, "49792\n");
}

#[test]
fn removes_port_file() {
    let tempdir = TempDir::new().unwrap();
    let _guard = env_lock().lock().unwrap();
    let _restore = EnvRestore {
        previous: std::env::var_os("STINT_DATA_DIR"),
    };
    std::env::set_var("STINT_DATA_DIR", tempdir.path());

    stint_app::http::write_port_file_for_test(49792).unwrap();
    let path = port_file_for(tempdir.path());
    assert!(path.exists());

    stint_app::http::remove_port_file_for_test().unwrap();
    assert!(!path.exists());
}
