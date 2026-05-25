//! Look up the currently active macOS Focus identifier.
//!
//! Production path: dlsym into Swift's `stint_current_focus_id_swift`
//! (exported by the StintIntents framework). When the framework isn't
//! loaded (CLI binary, headless tests, non-macOS), the helper returns
//! `None` and the [`verbs::start`] fallback treats it as "no current
//! focus" — the focus default is ignored.
//!
//! Test path: the `STINT_TEST_FOCUS_ID` env var, if set and non-empty,
//! short-circuits the dlsym lookup. This lets integration tests exercise
//! the focus fallback without a real Swift runtime.

use std::ffi::CStr;
use std::os::raw::c_char;

pub fn current_id() -> Option<String> {
    if let Ok(v) = std::env::var("STINT_TEST_FOCUS_ID") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    let mut out: *mut c_char = std::ptr::null_mut();
    let rc = unsafe { crate::ffi::stint_current_focus_id(&mut out) };
    if rc != 0 || out.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(out).to_str().ok()?.to_owned() };
    unsafe { crate::ffi::stint_free_string(out) };
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
