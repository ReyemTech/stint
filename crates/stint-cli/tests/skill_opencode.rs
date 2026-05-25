//! OpenCode harness install / uninstall behaviour against a tempdir XDG home.

use serde_json::Value;
use std::env;
use std::fs;
use stint_cli::skill::harness::{Harness, InstallAction};
use stint_cli::skill::opencode::OpenCode;
use tempfile::tempdir;

fn with_temp_home<F: FnOnce()>(f: F) {
    let tmp = tempdir().expect("tempdir");
    let prev_home = env::var_os("HOME");
    let prev_xdg = env::var_os("XDG_CONFIG_HOME");
    unsafe {
        env::set_var("HOME", tmp.path());
        // Force `dirs::config_dir()` onto our tempdir on both Linux and macOS.
        env::set_var("XDG_CONFIG_HOME", tmp.path().join(".config"));
    }
    f();
    unsafe {
        match prev_home {
            Some(p) => env::set_var("HOME", p),
            None => env::remove_var("HOME"),
        }
        match prev_xdg {
            Some(p) => env::set_var("XDG_CONFIG_HOME", p),
            None => env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}

fn config_dir() -> std::path::PathBuf {
    dirs::config_dir().unwrap().join("opencode")
}

#[test]
fn opencode_mcp_install_creates_json_with_stint_entry() {
    with_temp_home(|| {
        let h = OpenCode;
        let action = h.install_mcp(false).expect("install_mcp");
        assert!(
            matches!(action, InstallAction::Installed),
            "expected Installed, got {action:?}"
        );
        let raw = fs::read_to_string(config_dir().join("opencode.json")).unwrap();
        let v: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["mcp"]["stint"]["type"], "local");
        assert_eq!(v["mcp"]["stint"]["enabled"], true);
        assert_eq!(v["mcp"]["stint"]["command"], serde_json::json!(["stint", "mcp"]));
    });
}

#[test]
fn opencode_mcp_install_is_idempotent() {
    with_temp_home(|| {
        let h = OpenCode;
        h.install_mcp(false).unwrap();
        let second = h.install_mcp(false).unwrap();
        assert!(
            matches!(second, InstallAction::AlreadyUpToDate),
            "got {second:?}"
        );
    });
}

#[test]
fn opencode_mcp_install_preserves_existing_keys() {
    with_temp_home(|| {
        let path = config_dir().join("opencode.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{
  "theme": "dark",
  "mcp": {
    "other": {"type": "local", "command": ["foo"]}
  }
}"#,
        )
        .unwrap();
        let h = OpenCode;
        h.install_mcp(false).unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["mcp"]["other"]["command"], serde_json::json!(["foo"]));
        assert_eq!(v["mcp"]["stint"]["type"], "local");
        let backup = path.with_extension("json.bak");
        assert!(backup.exists(), "no backup at {}", backup.display());
    });
}

#[test]
fn opencode_skill_install_appends_block_to_agents_md() {
    with_temp_home(|| {
        let h = OpenCode;
        let action = h.install_skill(false).expect("install_skill");
        assert!(matches!(action, InstallAction::Installed));
        let agents = fs::read_to_string(config_dir().join("AGENTS.md")).unwrap();
        assert!(agents.contains("<!-- stint:begin -->"));
        assert!(agents.contains("<!-- stint:end -->"));
        assert!(agents.contains("stint (time tracker)"));
    });
}

#[test]
fn opencode_skill_install_is_idempotent() {
    with_temp_home(|| {
        let h = OpenCode;
        h.install_skill(false).unwrap();
        let second = h.install_skill(false).unwrap();
        assert!(
            matches!(second, InstallAction::AlreadyUpToDate),
            "got {second:?}"
        );
    });
}

#[test]
fn opencode_dry_run_writes_nothing() {
    with_temp_home(|| {
        let h = OpenCode;
        assert!(matches!(
            h.install_mcp(true).unwrap(),
            InstallAction::Skipped
        ));
        assert!(matches!(
            h.install_skill(true).unwrap(),
            InstallAction::Skipped
        ));
        assert!(!config_dir().join("opencode.json").exists());
        assert!(!config_dir().join("AGENTS.md").exists());
    });
}

#[test]
fn opencode_uninstall_removes_only_stint() {
    with_temp_home(|| {
        let path = config_dir().join("opencode.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"theme":"dark","mcp":{"other":{"type":"local","command":["foo"]}}}"#,
        )
        .unwrap();
        let h = OpenCode;
        h.install_mcp(false).unwrap();
        h.install_skill(false).unwrap();
        h.uninstall().unwrap();
        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["mcp"].get("stint").is_none());
        assert!(v["mcp"].get("other").is_some());
        assert!(!config_dir().join("AGENTS.md").exists() || {
            let s = fs::read_to_string(config_dir().join("AGENTS.md")).unwrap();
            !s.contains("stint:begin")
        });
    });
}

#[test]
fn opencode_status_reflects_install_state() {
    with_temp_home(|| {
        let h = OpenCode;
        let before = h.status().expect("status");
        assert!(!before.mcp_installed);
        assert!(!before.skill_installed);
        h.install_mcp(false).unwrap();
        h.install_skill(false).unwrap();
        let after = h.status().expect("status");
        assert!(after.mcp_installed);
        assert!(after.skill_installed);
        assert_eq!(after.name, "opencode");
    });
}
