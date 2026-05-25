//! Tests for the settings + log + focus_id FFI surfaces.

use std::ffi::{c_char, CStr, CString};
use std::ptr;
use tempfile::TempDir;

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

#[test]
fn settings_set_get_clear_round_trip() {
    let _guard = DataDirGuard::new();
    let key = CString::new("focus.default_project").unwrap();
    let val = CString::new("focus-uuid-abc\tproject-uuid-xyz").unwrap();

    let rc = unsafe { stint_core::ffi::stint_settings_set(key.as_ptr(), val.as_ptr()) };
    assert_eq!(rc, 0);

    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_settings_get(key.as_ptr(), &mut out) };
    assert_eq!(rc, 0);
    assert!(!out.is_null());
    let got = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    assert_eq!(got, "focus-uuid-abc\tproject-uuid-xyz");
    unsafe { stint_core::ffi::stint_free_string(out) };

    let rc = unsafe { stint_core::ffi::stint_settings_clear(key.as_ptr()) };
    assert_eq!(rc, 0);

    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_settings_get(key.as_ptr(), &mut out) };
    assert_eq!(rc, 0);
    assert!(out.is_null(), "cleared key must return null pointer");
}

#[test]
fn settings_get_missing_key_returns_null() {
    let _guard = DataDirGuard::new();
    let key = CString::new("absent.key").unwrap();
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_settings_get(key.as_ptr(), &mut out) };
    assert_eq!(rc, 0);
    assert!(out.is_null());
}

#[test]
fn settings_null_pointers_return_misuse() {
    let key = CString::new("k").unwrap();
    let rc = unsafe { stint_core::ffi::stint_settings_set(ptr::null(), key.as_ptr()) };
    assert_eq!(rc, -2);
    let rc = unsafe { stint_core::ffi::stint_settings_set(key.as_ptr(), ptr::null()) };
    assert_eq!(rc, -2);
    let rc = unsafe { stint_core::ffi::stint_settings_get(ptr::null(), ptr::null_mut()) };
    assert_eq!(rc, -2);
    let rc = unsafe { stint_core::ffi::stint_settings_clear(ptr::null()) };
    assert_eq!(rc, -2);
}

#[test]
fn log_warn_does_not_panic() {
    let msg = CString::new("hello from swift").unwrap();
    unsafe { stint_core::ffi::stint_log_warn(msg.as_ptr()) };
    unsafe { stint_core::ffi::stint_log_warn(ptr::null()) };
}

#[test]
fn current_focus_id_returns_null_in_tests() {
    let mut out: *mut c_char = ptr::null_mut();
    let rc = unsafe { stint_core::ffi::stint_current_focus_id(&mut out) };
    assert_eq!(rc, 0);
    // In tests the dlsym lookup returns null (Swift framework isn't loaded).
    assert!(out.is_null());
}

#[test]
fn notify_indexer_is_noop_when_swift_absent() {
    // No assertion — just that it doesn't crash without a Swift framework loaded.
    stint_core::ffi::notify_indexer(
        stint_core::ffi::IndexerKind::EntryStarted,
        r#"{"local_uuid":"u1"}"#,
    );
    stint_core::ffi::notify_indexer(
        stint_core::ffi::IndexerKind::EntryStopped,
        r#"{"local_uuid":"u1"}"#,
    );
    stint_core::ffi::notify_indexer(stint_core::ffi::IndexerKind::ProjectsReplaced, "[]");
}
