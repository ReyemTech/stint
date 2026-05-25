//! C ABI surface for Swift consumers (the StintIntents framework).
//!
//! Every public `extern "C"` function writes a JSON envelope into `out_json`:
//!
//! ```text
//! { "ok":  <T> }
//! { "err": { "code": <int>, "message": "<str>" } }
//! ```
//!
//! Error codes are a stable public contract — do not renumber. See the spec
//! at `docs/superpowers/specs/2026-05-25-stint-phase-6-deeper-integration-design.md#71-envelope-contract`.
//!
//! Memory ownership: every `*out_json` is malloc'd by Rust via `CString::into_raw`.
//! Callers must free it via [`stint_free_string`]. Passing NULL to
//! `stint_free_string` is safe and is a no-op.
//!
//! Panic safety: each FFI fn body runs inside `catch_unwind`. A caught panic
//! becomes a `-1` envelope rather than undefined behavior across the C ABI.

use crate::Error;
use serde::Serialize;
use std::ffi::{c_char, CString};
use std::panic;

/// Stable error-code contract — see module docs.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
enum Code {
    Invariant = 1,
    NotFound = 2,
    // 3 = Conflict — reserved for future use (no current Error variant maps to it)
    Serialization = 4,
    Internal = 99,
    Panic = -1,
}

fn code_for(err: &Error) -> i32 {
    match err {
        Error::Invariant(_) => Code::Invariant as i32,
        Error::NotFound(_) => Code::NotFound as i32,
        Error::Serde(_) => Code::Serialization as i32,
        _ => Code::Internal as i32,
    }
}

/// Build a `{ok:T} | {err:{code,message}}` envelope JSON and write it to
/// `*out_json` as a heap-allocated CString. The caller (Swift) is
/// responsible for freeing the string via [`stint_free_string`].
fn write_envelope<T: Serialize>(out_json: *mut *mut c_char, result: Result<T, Error>) {
    if out_json.is_null() {
        return;
    }
    let body = match result {
        Ok(t) => serde_json::json!({ "ok": t }),
        Err(e) => serde_json::json!({
            "err": { "code": code_for(&e), "message": e.to_string() }
        }),
    };
    let s = body.to_string();
    let c = match CString::new(s) {
        Ok(c) => c,
        Err(_) => CString::new(r#"{"err":{"code":99,"message":"cstring contained internal NUL"}}"#)
            .unwrap(),
    };
    unsafe { *out_json = c.into_raw() };
}

/// Wrap an FFI body in `catch_unwind`. On panic, write a Panic envelope.
fn ffi_body<F, T>(out_json: *mut *mut c_char, f: F)
where
    F: FnOnce() -> Result<T, Error> + std::panic::UnwindSafe,
    T: Serialize,
{
    let result = panic::catch_unwind(f);
    match result {
        Ok(r) => write_envelope(out_json, r),
        Err(p) => {
            let msg = downcast_panic(p);
            let body = serde_json::json!({
                "err": { "code": Code::Panic as i32, "message": msg }
            });
            let c = CString::new(body.to_string()).unwrap_or_else(|_| {
                CString::new(r#"{"err":{"code":-1,"message":"panic"}}"#).unwrap()
            });
            if !out_json.is_null() {
                unsafe { *out_json = c.into_raw() };
            }
        }
    }
}

fn downcast_panic(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&'static str>() {
        return (*s).to_owned();
    }
    if let Some(s) = p.downcast_ref::<String>() {
        return s.clone();
    }
    "rust panic (no message)".into()
}

/// Free a CString previously returned via `*out_json`. Safe to call with NULL.
///
/// # Safety
///
/// `ptr` must either be NULL or have been produced by one of this module's
/// FFI functions via `CString::into_raw`. Calling with any other pointer is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn stint_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    let _ = CString::from_raw(ptr);
}

// ---- test-only re-exports ---------------------------------------------

/// Test-only helper: exposes the internal envelope writer so unit tests can
/// exercise `write_envelope` without a verb context.
#[doc(hidden)]
pub fn write_envelope_for_test<T: Serialize>(out_json: *mut *mut c_char, result: Result<T, Error>) {
    write_envelope(out_json, result);
}

/// Test-only helper: forces the `ffi_body` panic path so the `catch_unwind`
/// branch is exercised end-to-end.
#[doc(hidden)]
pub fn panic_for_test(out_json: *mut *mut c_char) {
    ffi_body::<_, ()>(out_json, || panic!("test panic"));
}

