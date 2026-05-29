//! Envelope shape contract — every FFI fn must produce {ok:T} or {err:{code,message}}.

use serde_json::Value;
use std::ffi::{c_char, CStr};
use std::ptr;

#[test]
fn envelope_ok_shape() {
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::write_envelope_for_test::<Value>(&mut out, Ok(serde_json::json!({ "a": 1 })));
    assert!(!out.is_null());
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["ok"]["a"], 1);
    assert!(v.get("err").is_none());
    unsafe { stint_core::ffi::stint_free_string(out) };
}

#[test]
fn envelope_err_invariant_shape() {
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::write_envelope_for_test::<Value>(
        &mut out,
        Err(stint_core::Error::Invariant("nope".into())),
    );
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], 1);
    assert_eq!(v["err"]["message"], "invariant violation: nope");
    unsafe { stint_core::ffi::stint_free_string(out) };
}

#[test]
fn envelope_err_not_found_shape() {
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::write_envelope_for_test::<Value>(
        &mut out,
        Err(stint_core::Error::NotFound("missing-uuid".into())),
    );
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], 2);
    unsafe { stint_core::ffi::stint_free_string(out) };
}

#[test]
fn envelope_err_serialization_maps_to_code_4() {
    // Synthesize a serde_json::Error and confirm it maps to code 4.
    let bad: serde_json::Error = serde_json::from_str::<i32>("not a number").unwrap_err();
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::write_envelope_for_test::<Value>(&mut out, Err(stint_core::Error::Serde(bad)));
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], 4);
    unsafe { stint_core::ffi::stint_free_string(out) };
}

#[test]
fn envelope_err_other_maps_to_internal() {
    let mut out: *mut c_char = ptr::null_mut();
    stint_core::ffi::write_envelope_for_test::<Value>(
        &mut out,
        Err(stint_core::Error::SolidtimeAuth),
    );
    let s = unsafe { CStr::from_ptr(out).to_str().unwrap().to_owned() };
    let v: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["err"]["code"], 99);
    unsafe { stint_core::ffi::stint_free_string(out) };
}

#[test]
fn free_string_handles_null() {
    // Must not segfault.
    unsafe { stint_core::ffi::stint_free_string(ptr::null_mut()) };
}

#[test]
fn write_envelope_handles_null_out_param() {
    // Should be a no-op, not a crash.
    stint_core::ffi::write_envelope_for_test::<Value>(
        ptr::null_mut(),
        Ok(serde_json::json!({"a": 1})),
    );
}
