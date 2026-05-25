//! Claude Code harness install / uninstall behaviour against a tempdir HOME.
//!
//! These tests swap `HOME` before driving the harness, so they need to run
//! single-threaded (the workspace test harness already enforces this).

use std::env;
use stint_cli::skill::claude_code::ClaudeCode;
use stint_cli::skill::harness::{Harness, InstallAction};
use tempfile::tempdir;

/// Run `f` with `HOME` set to a fresh tempdir, restoring the previous value
/// (or unsetting it) on the way out.
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

#[test]
fn claude_skill_install_creates_file() {
    with_temp_home(|| {
        let h = ClaudeCode;
        let action = h.install_skill(false).expect("install_skill");
        assert!(
            matches!(action, InstallAction::Installed),
            "expected Installed, got {action:?}"
        );
        let path = dirs::home_dir()
            .unwrap()
            .join(".claude/skills/stint/SKILL.md");
        assert!(
            path.exists(),
            "skill file not written at {}",
            path.display()
        );
    });
}

#[test]
fn claude_skill_install_is_idempotent() {
    with_temp_home(|| {
        let h = ClaudeCode;
        let first = h.install_skill(false).expect("first install");
        assert!(matches!(first, InstallAction::Installed));
        let second = h.install_skill(false).expect("second install");
        assert!(
            matches!(second, InstallAction::AlreadyUpToDate),
            "expected AlreadyUpToDate on re-install, got {second:?}"
        );
    });
}

#[test]
fn claude_skill_install_updates_when_content_differs() {
    with_temp_home(|| {
        let h = ClaudeCode;
        let path = dirs::home_dir()
            .unwrap()
            .join(".claude/skills/stint/SKILL.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "stale content").unwrap();

        let action = h.install_skill(false).expect("install_skill");
        assert!(
            matches!(action, InstallAction::Updated),
            "expected Updated, got {action:?}"
        );
        // Backup must have been written.
        let backup = path.with_extension("md.bak");
        assert!(backup.exists(), "no backup at {}", backup.display());
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "stale content");
    });
}

#[test]
fn claude_skill_install_dry_run_writes_nothing() {
    with_temp_home(|| {
        let h = ClaudeCode;
        let action = h.install_skill(true).expect("dry-run install_skill");
        assert!(matches!(action, InstallAction::Skipped));
        let path = dirs::home_dir()
            .unwrap()
            .join(".claude/skills/stint/SKILL.md");
        assert!(
            !path.exists(),
            "dry run wrote skill file at {}",
            path.display()
        );
    });
}

#[test]
fn claude_skill_uninstall_removes_file() {
    with_temp_home(|| {
        let h = ClaudeCode;
        h.install_skill(false).expect("install_skill");
        h.uninstall().expect("uninstall");
        let path = dirs::home_dir()
            .unwrap()
            .join(".claude/skills/stint/SKILL.md");
        assert!(!path.exists(), "skill file lingered at {}", path.display());
    });
}

#[test]
fn claude_status_reports_skill_present_after_install() {
    with_temp_home(|| {
        let h = ClaudeCode;
        let before = h.status().expect("status");
        assert!(!before.skill_installed);
        h.install_skill(false).expect("install_skill");
        let after = h.status().expect("status");
        assert!(after.skill_installed);
        assert_eq!(after.name, "claude");
        assert_eq!(after.display, "Claude Code");
    });
}
