//! CLI surface for `stint skill` — covers the dispatch wrapper in
//! cmd/skill.rs. Harness-level behavior is exercised by the
//! skill_claude / skill_codex / skill_opencode test files. Here we just
//! verify the clap routing and JSON-vs-human render.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::env;
use tempfile::tempdir;

/// Run `f` with `HOME` set to a fresh tempdir, restoring afterwards.
/// Skill commands read $HOME (or USERPROFILE on Windows) for harness
/// install/status, so swapping it isolates from the developer's
/// real ~/.claude and ~/.codex directories.
fn with_temp_home<F: FnOnce(&std::path::Path)>(f: F) {
    let tmp = tempdir().expect("tempdir");
    let prev = env::var_os("HOME");
    unsafe {
        env::set_var("HOME", tmp.path());
    }
    f(tmp.path());
    unsafe {
        match prev {
            Some(p) => env::set_var("HOME", p),
            None => env::remove_var("HOME"),
        }
    }
}

#[test]
fn skill_status_human_lists_every_harness() {
    with_temp_home(|home| {
        Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["skill", "status"])
            .assert()
            .success()
            // The status renderer prints one row per known harness
            // using each harness's `display()` name.
            .stdout(predicate::str::contains("Claude Code"))
            .stdout(predicate::str::contains("Codex"))
            .stdout(predicate::str::contains("OpenCode"));
    });
}

#[test]
fn skill_status_json_emits_array_of_three_rows() {
    with_temp_home(|home| {
        let out = Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["--json", "skill", "status"])
            .output()
            .expect("skill status --json");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
        let rows = v.as_array().expect("array");
        assert_eq!(rows.len(), 3, "expected 3 harnesses, got: {v}");
        let names: Vec<&str> = rows.iter().map(|r| r["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"opencode"));
    });
}

#[test]
fn skill_install_unknown_harness_returns_error() {
    Command::cargo_bin("stint")
        .unwrap()
        .args(["skill", "install", "no-such-harness"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown harness"));
}

#[test]
fn skill_install_codex_dry_run_emits_dry_run_marker() {
    with_temp_home(|home| {
        Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["skill", "install", "codex", "--dry-run"])
            .assert()
            .success()
            .stdout(predicate::str::contains("dry run"));
    });
}

#[test]
fn skill_install_codex_writes_files_and_status_reflects_it() {
    with_temp_home(|home| {
        Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["skill", "install", "codex"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Installed stint for Codex"));

        // status now sees both pieces.
        let out = Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["--json", "skill", "status"])
            .output()
            .expect("status --json");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        let codex = v
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "codex")
            .unwrap();
        assert_eq!(codex["mcp_installed"], true);
        assert_eq!(codex["skill_installed"], true);
    });
}

#[test]
fn skill_uninstall_codex_removes_pieces() {
    with_temp_home(|home| {
        Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["skill", "install", "codex"])
            .assert()
            .success();
        Command::cargo_bin("stint")
            .unwrap()
            .env("HOME", home)
            .args(["--json", "skill", "uninstall", "codex"])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"uninstalled\""));
    });
}
