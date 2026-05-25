//! Integration tests for the 8 extern "C" verb wrappers.
//!
//! Each test sets STINT_DATA_DIR to its own tempdir so the FFI's lazy store
//! cache opens against a fresh SQLite. cargo test --test-threads=1 ensures
//! sequential execution (env var manipulation isn't thread-safe).

use serde_json::Value;
use std::ffi::{c_char, CStr, CString};
use std::ptr;
use tempfile::TempDir;

/// Test guard that points STINT_DATA_DIR at a fresh tempdir. Drop restores
/// the previous value so other tests in the same process don't bleed state.
struct DataDirGuard {
    _tempdir: TempDir,
    prev: Option<String>,
}

impl DataDirGuard {
    fn new() -> Self {
        let prev = std::env::var("STINT_DATA_DIR").ok();
        let tempdir = TempDir::new().expect("create tempdir");
        std::env::set_var("STINT_DATA_DIR", tempdir.path());
        Self {
            _tempdir: tempdir,
            prev,
        }
    }
}

impl Drop for DataDirGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var("STINT_DATA_DIR", v),
            None => std::env::remove_var("STINT_DATA_DIR"),
        }
    }
}

fn call_with_params(
    verb: unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> i32,
    params: &str,
) -> Value {
    let cstr = CString::new(params).unwrap();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { verb(cstr.as_ptr(), &mut out) };
    assert_eq!(rc, 0, "FFI return code: {rc}");
    decode_envelope(out)
}

fn call_no_params(verb: unsafe extern "C" fn(*mut *mut c_char) -> i32) -> Value {
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { verb(&mut out) };
    assert_eq!(rc, 0, "FFI return code: {rc}");
    decode_envelope(out)
}

fn decode_envelope(out: *mut c_char) -> Value {
    assert!(!out.is_null());
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap_or_else(|e| panic!("bad envelope: {s}: {e}"));
    unsafe { stint_core::ffi::stint_free_string(out) };
    v
}

#[test]
fn ffi_start_happy_path() {
    let _guard = DataDirGuard::new();
    let env = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"writing tests","source":"ffi-test"}"#,
    );
    assert!(env["ok"].is_object(), "envelope: {env}");
    assert_eq!(env["ok"]["description"], "writing tests");
    assert_eq!(env["ok"]["source"], "ffi-test");
    assert!(env["ok"]["local_uuid"].is_string());
}

#[test]
fn ffi_start_invariant_already_running() {
    let _guard = DataDirGuard::new();
    let first = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"first","source":"ffi-test"}"#,
    );
    assert!(
        first["ok"].is_object(),
        "first start should succeed: {first}"
    );

    let env = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"second","source":"ffi-test"}"#,
    );
    assert_eq!(env["err"]["code"], 1, "envelope: {env}");
}

#[test]
fn ffi_current_when_running() {
    let _guard = DataDirGuard::new();
    let _ = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"x","source":"ffi-test"}"#,
    );
    let env = call_no_params(stint_core::ffi::stint_verb_current);
    // current returns Option<EntryView> — Some(view)
    assert_eq!(env["ok"]["description"], "x");
}

#[test]
fn ffi_current_when_no_timer() {
    let _guard = DataDirGuard::new();
    let env = call_no_params(stint_core::ffi::stint_verb_current);
    // Option::None serializes to null
    assert!(env["ok"].is_null(), "envelope: {env}");
}

#[test]
fn ffi_stop_after_start() {
    let _guard = DataDirGuard::new();
    let _ = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"y","source":"ffi-test"}"#,
    );
    let env = call_no_params(stint_core::ffi::stint_verb_stop);
    assert!(env["ok"]["end_at"].is_string(), "envelope: {env}");
}

#[test]
fn ffi_stop_with_no_running_timer_errors() {
    let _guard = DataDirGuard::new();
    let env = call_no_params(stint_core::ffi::stint_verb_stop);
    assert!(env.get("err").is_some(), "envelope: {env}");
}

#[test]
fn ffi_list_entries_empty() {
    let _guard = DataDirGuard::new();
    let env = call_with_params(stint_core::ffi::stint_verb_list_entries, "{}");
    assert!(env["ok"].is_array(), "envelope: {env}");
    assert_eq!(env["ok"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_list_projects_empty() {
    let _guard = DataDirGuard::new();
    let env = call_no_params(stint_core::ffi::stint_verb_list_projects);
    assert!(env["ok"].is_array(), "envelope: {env}");
}

#[test]
fn ffi_list_tasks_empty() {
    let _guard = DataDirGuard::new();
    let env = call_with_params(stint_core::ffi::stint_verb_list_tasks, "{}");
    assert!(env["ok"].is_array(), "envelope: {env}");
}

#[test]
fn ffi_update_entry_not_found() {
    let _guard = DataDirGuard::new();
    let env = call_with_params(
        stint_core::ffi::stint_verb_update_entry,
        r#"{"local_uuid":"does-not-exist","patch":{}}"#,
    );
    assert_eq!(env["err"]["code"], 2, "envelope: {env}");
}

#[test]
fn ffi_delete_entry_is_idempotent() {
    // verbs::delete_entry intentionally treats a missing row as success
    // (the verb contract is "ensure it's gone"). The FFI envelope mirrors
    // that — ok payload is `{}`.
    let _guard = DataDirGuard::new();
    let env = call_with_params(
        stint_core::ffi::stint_verb_delete_entry,
        r#"{"local_uuid":"does-not-exist"}"#,
    );
    assert_eq!(env["ok"], serde_json::json!({}), "envelope: {env}");
}

#[test]
fn ffi_delete_entry_actually_removes() {
    let _guard = DataDirGuard::new();
    let started = call_with_params(
        stint_core::ffi::stint_verb_start,
        r#"{"description":"to delete","source":"ffi-test"}"#,
    );
    let uuid = started["ok"]["local_uuid"].as_str().unwrap().to_owned();
    let _ = call_no_params(stint_core::ffi::stint_verb_stop);

    let payload = format!(r#"{{"local_uuid":"{uuid}"}}"#);
    let env = call_with_params(stint_core::ffi::stint_verb_delete_entry, &payload);
    assert_eq!(env["ok"], serde_json::json!({}), "delete envelope: {env}");

    // Verify the entry is gone via list_entries.
    let list = call_with_params(stint_core::ffi::stint_verb_list_entries, "{}");
    assert_eq!(list["ok"].as_array().unwrap().len(), 0);
}

#[test]
fn ffi_start_malformed_json_returns_serialization_error() {
    let _guard = DataDirGuard::new();
    let env = call_with_params(stint_core::ffi::stint_verb_start, "not json");
    assert_eq!(env["err"]["code"], 4, "envelope: {env}");
}

#[test]
fn ffi_start_null_params_returns_invariant_error() {
    let _guard = DataDirGuard::new();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_verb_start(ptr::null(), &mut out) };
    assert_eq!(rc, 0);
    let env = decode_envelope(out);
    assert_eq!(env["err"]["code"], 1, "envelope: {env}");
}
