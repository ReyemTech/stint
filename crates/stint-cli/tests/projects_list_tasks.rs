//! `stint projects list-tasks <project-id>` returns tasks for a project.

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn list_tasks_empty_when_no_data() {
    let tempdir = TempDir::new().unwrap();
    let output = Command::cargo_bin("stint")
        .unwrap()
        .env("STINT_DATA_DIR", tempdir.path())
        .args(["--json", "projects", "list-tasks", "proj-abc"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&output).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 0);
}
