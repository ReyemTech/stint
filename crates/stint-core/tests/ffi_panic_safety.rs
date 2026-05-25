//! A Rust panic across the FFI boundary must be caught by `catch_unwind`
//! and turned into a `code = -1` Panic envelope — never undefined behavior.

use serde_json::Value;
use std::ffi::{c_char, CStr};
use std::ptr;

#[test]
fn panic_in_ffi_body_returns_envelope_not_segfault() {
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::panic_for_test(&mut out);

    assert!(!out.is_null(), "envelope must be written even on panic");
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], -1);
    assert!(
        v["err"]["message"].as_str().unwrap().contains("test panic"),
        "panic message should be surfaced; got: {}",
        v["err"]["message"]
    );
    unsafe { stint_core::ffi::stint_free_string(out) };
}
