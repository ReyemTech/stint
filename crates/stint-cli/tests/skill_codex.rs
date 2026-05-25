//! Codex harness install / uninstall behaviour against a tempdir HOME.

use std::env;
use std::fs;
use stint_cli::skill::codex::Codex;
use stint_cli::skill::harness::{Harness, InstallAction};
use tempfile::tempdir;

fn with_temp_home<F: FnOnce()>(f: F) {
    let tmp = tempdir().expect("tempdir");
    let prev = env::var_os("HOME");
    unsafe {
        env::set_var("HOME", tmp.path());
    }
    f();
    unsafe {
        match prev {
            Some(p) => env::set_var("HOME", p),
            None => env::remove_var("HOME"),
        }
    }
}

fn home() -> std::path::PathBuf {
    dirs::home_dir().unwrap()
}

#[test]
fn codex_mcp_install_creates_config_with_stint_block() {
    with_temp_home(|| {
        let h = Codex;
        let action = h.install_mcp(false).expect("install_mcp");
        assert!(
            matches!(action, InstallAction::Installed),
            "expected Installed, got {action:?}"
        );
        let cfg = fs::read_to_string(home().join(".codex/config.toml")).unwrap();
        assert!(
            cfg.contains("[mcp_servers.stint]"),
            "missing stint table:\n{cfg}"
        );
        assert!(cfg.contains("command = \"stint\""));
        assert!(cfg.contains("args = [\"mcp\"]"));
    });
}

#[test]
fn codex_mcp_install_is_idempotent() {
    with_temp_home(|| {
        let h = Codex;
        h.install_mcp(false).expect("first");
        let second = h.install_mcp(false).expect("second");
        assert!(
            matches!(second, InstallAction::AlreadyUpToDate),
            "got {second:?}"
        );
    });
}

#[test]
fn codex_mcp_install_preserves_existing_keys() {
    with_temp_home(|| {
        let cfg_path = home().join(".codex/config.toml");
        fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
        fs::write(
            &cfg_path,
            "model = \"o4-mini\"\n\n[mcp_servers.other]\ncommand = \"foo\"\n",
        )
        .unwrap();
        let h = Codex;
        h.install_mcp(false).expect("install_mcp");
        let cfg = fs::read_to_string(&cfg_path).unwrap();
        assert!(cfg.contains("model = \"o4-mini\""));
        assert!(cfg.contains("[mcp_servers.other]"));
        assert!(cfg.contains("[mcp_servers.stint]"));
        let backup = cfg_path.with_extension("toml.bak");
        assert!(backup.exists(), "no backup at {}", backup.display());
    });
}

#[test]
fn codex_skill_install_appends_block_to_agents_md() {
    with_temp_home(|| {
        let h = Codex;
        let action = h.install_skill(false).expect("install_skill");
        assert!(matches!(action, InstallAction::Installed));
        let agents = fs::read_to_string(home().join(".codex/AGENTS.md")).unwrap();
        assert!(agents.contains("<!-- stint:begin -->"));
        assert!(agents.contains("<!-- stint:end -->"));
        assert!(agents.contains("stint (time tracker)"));
    });
}

#[test]
fn codex_skill_install_is_idempotent() {
    with_temp_home(|| {
        let h = Codex;
        h.install_skill(false).expect("first");
        let second = h.install_skill(false).expect("second");
        assert!(
            matches!(second, InstallAction::AlreadyUpToDate),
            "got {second:?}"
        );
    });
}

#[test]
fn codex_skill_install_replaces_existing_block() {
    with_temp_home(|| {
        let ag_path = home().join(".codex/AGENTS.md");
        fs::create_dir_all(ag_path.parent().unwrap()).unwrap();
        fs::write(
            &ag_path,
            "# my notes\n\n<!-- stint:begin -->\nstale\n<!-- stint:end -->\n\n## other\n",
        )
        .unwrap();
        let h = Codex;
        let action = h.install_skill(false).expect("install_skill");
        assert!(matches!(action, InstallAction::Updated));
        let agents = fs::read_to_string(&ag_path).unwrap();
        assert!(agents.contains("# my notes"));
        assert!(agents.contains("## other"));
        assert!(agents.contains("stint (time tracker)"));
        assert!(!agents.contains("stale"));
    });
}

#[test]
fn codex_skill_install_dry_run_writes_nothing() {
    with_temp_home(|| {
        let h = Codex;
        assert!(matches!(
            h.install_skill(true).unwrap(),
            InstallAction::Skipped
        ));
        assert!(matches!(
            h.install_mcp(true).unwrap(),
            InstallAction::Skipped
        ));
        assert!(!home().join(".codex/AGENTS.md").exists());
        assert!(!home().join(".codex/config.toml").exists());
    });
}

#[test]
fn codex_uninstall_strips_stint_pieces_only() {
    with_temp_home(|| {
        let cfg_path = home().join(".codex/config.toml");
        let ag_path = home().join(".codex/AGENTS.md");
        fs::create_dir_all(cfg_path.parent().unwrap()).unwrap();
        fs::write(
            &cfg_path,
            "model = \"o4-mini\"\n\n[mcp_servers.other]\ncommand = \"foo\"\n",
        )
        .unwrap();
        fs::write(&ag_path, "# my notes\n\n## other\n").unwrap();

        let h = Codex;
        h.install_mcp(false).expect("install_mcp");
        h.install_skill(false).expect("install_skill");
        h.uninstall().expect("uninstall");

        let cfg = fs::read_to_string(&cfg_path).unwrap();
        assert!(cfg.contains("[mcp_servers.other]"));
        assert!(!cfg.contains("[mcp_servers.stint]"));

        let agents = fs::read_to_string(&ag_path).unwrap();
        assert!(agents.contains("# my notes"));
        assert!(!agents.contains("stint:begin"));
    });
}

#[test]
fn codex_status_reflects_install_state() {
    with_temp_home(|| {
        let h = Codex;
        let before = h.status().expect("status");
        assert!(!before.mcp_installed);
        assert!(!before.skill_installed);
        h.install_mcp(false).unwrap();
        h.install_skill(false).unwrap();
        let after = h.status().expect("status");
        assert!(after.mcp_installed);
        assert!(after.skill_installed);
        assert_eq!(after.name, "codex");
    });
}
