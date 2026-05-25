//! End-to-end: spawn `stint mcp` as a child process and exchange MCP
//! messages over stdio. Verifies the server exposes 8 tools and round-trips
//! a `start` call into the local store.
//!
//! rmcp's stdio transport is line-delimited JSON-RPC — one message per
//! line. We bypass the rmcp client lib here on purpose: writing the wire
//! bytes ourselves exercises the framing the way a real Claude Code /
//! Codex / OpenCode client would.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::tempdir;

/// Block on the next newline-terminated message from the server.
fn read_one(stdout: &mut impl BufRead) -> Value {
    let mut line = String::new();
    let n = stdout.read_line(&mut line).expect("read line");
    assert!(n > 0, "server closed stdout before reply");
    serde_json::from_str(line.trim()).unwrap_or_else(|e| {
        panic!("server emitted non-JSON line {line:?}: {e}");
    })
}

#[test]
fn mcp_server_lists_tools_and_starts_entry() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("stint.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_stint"))
        .arg("mcp")
        .env("STINT_DB", &db)
        // Keep tracing output away from stdout — stdout is the MCP wire.
        .env("STINT_LOG", "off")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn `stint mcp`");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // 1) initialize
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "stint-mcp-e2e", "version": "0" }
        }
    });
    writeln!(stdin, "{init}").unwrap();
    let resp = read_one(&mut stdout);
    assert_eq!(resp["id"], 1, "initialize id mismatch: {resp}");
    assert!(
        resp["result"]["protocolVersion"].is_string(),
        "missing protocolVersion: {resp}"
    );

    // 2) initialized notification (no response expected)
    let init_notif = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    writeln!(stdin, "{init_notif}").unwrap();

    // 3) tools/list — must surface all 8 verbs
    let req = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"});
    writeln!(stdin, "{req}").unwrap();
    let resp = read_one(&mut stdout);
    let names: Vec<String> = resp["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("no tools array: {resp}"))
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    for expected in [
        "start",
        "stop",
        "current",
        "list_entries",
        "list_projects",
        "list_tasks",
        "update_entry",
        "delete_entry",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing tool {expected:?}; got {names:?}"
        );
    }
    assert_eq!(
        names.len(),
        8,
        "expected exactly 8 tools, got {}: {names:?}",
        names.len()
    );

    // 4) tools/call start — verify it actually mutates the store
    let req = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "start",
            "arguments": { "description": "mcp test" }
        }
    });
    writeln!(stdin, "{req}").unwrap();
    let resp = read_one(&mut stdout);
    let payload_text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("unexpected response shape: {resp}"));
    let payload: Value = serde_json::from_str(payload_text).unwrap();
    assert_eq!(payload["description"], "mcp test");
    assert_eq!(
        payload["source"], "mcp",
        "source must be forced to 'mcp' regardless of input"
    );
    assert!(payload["local_uuid"].is_string());

    // 5) tools/call current — should reflect the just-started entry
    let req = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": { "name": "current", "arguments": {} }
    });
    writeln!(stdin, "{req}").unwrap();
    let resp = read_one(&mut stdout);
    let payload_text = resp["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("unexpected response shape: {resp}"));
    let payload: Value = serde_json::from_str(payload_text).unwrap();
    assert_eq!(payload["description"], "mcp test");

    // Drop stdin so the server's read loop sees EOF and exits cleanly,
    // then kill as a safety net if it lingers.
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
