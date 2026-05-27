//! `api.port` file is written on bind, removed on drop.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn port_file_for(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("api.port")
}

#[tokio::test]
async fn writes_port_file_on_bind() {
    let tempdir = TempDir::new().unwrap();
    std::env::set_var("STINT_DATA_DIR", tempdir.path());

    let port = stint_app::http::write_port_file_for_test(49792).unwrap();
    assert_eq!(port, 49792);
    let path = port_file_for(tempdir.path());
    assert!(path.exists(), "port file not at {}", path.display());
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(contents.trim(), "49792");
}

#[tokio::test]
async fn removes_port_file() {
    let tempdir = TempDir::new().unwrap();
    std::env::set_var("STINT_DATA_DIR", tempdir.path());
    stint_app::http::write_port_file_for_test(49792).unwrap();
    let path = port_file_for(tempdir.path());
    assert!(path.exists());

    stint_app::http::remove_port_file_for_test().unwrap();
    assert!(!path.exists());
}
