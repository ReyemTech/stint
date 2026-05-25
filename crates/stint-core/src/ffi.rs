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

use crate::config::Settings;
use crate::store::Store;
use crate::{paths, verbs, Error};
use serde::{Deserialize, Serialize};
use std::ffi::{c_char, CStr, CString};
use std::panic;
use std::ptr;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

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

// ---- shared runtime + store ------------------------------------------

/// Lazy multi-threaded Tokio runtime used to `block_on` async verbs from
/// the synchronous FFI surface. One process-wide runtime; Swift callers
/// never see tokio.
fn runtime() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("ffi: failed to build tokio runtime")
    })
}

/// Open the user-default `Store` for the current process and cache it.
///
/// The cache key is the DB path resolved via `paths::database_path()`. If
/// `STINT_DATA_DIR` changes between calls (tests do this), the cache
/// re-opens against the new path. Production opens once.
fn store() -> Result<Store, Error> {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Store>>> = OnceLock::new();

    let path = paths::database_path()?;
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let guard = cache.lock().unwrap();
        if let Some(s) = guard.get(&path) {
            return Ok(s.clone());
        }
    }
    let s = runtime().block_on(Store::connect(&path))?;
    cache.lock().unwrap().insert(path, s.clone());
    Ok(s)
}

/// Parse a JSON-encoded `*const c_char` into a Deserialize. NULL maps to
/// `Error::Invariant` so the caller sees an `err.code = 1` envelope.
unsafe fn parse_params<'a, T: Deserialize<'a>>(ptr: *const c_char) -> Result<T, Error> {
    if ptr.is_null() {
        return Err(Error::Invariant("null params pointer".into()));
    }
    let cstr = unsafe { CStr::from_ptr(ptr) };
    let s = cstr
        .to_str()
        .map_err(|e| Error::Invariant(format!("non-utf8 params: {e}")))?;
    serde_json::from_str(s).map_err(Error::Serde)
}

// ---- verbs -----------------------------------------------------------

/// Start a new running entry. JSON params match `verbs::StartParams`.
///
/// # Safety
/// `params_json` is a NUL-terminated C string. `out_json` must point at a
/// valid `*mut c_char` slot to receive the envelope (must be freed by the
/// caller via [`stint_free_string`]).
#[no_mangle]
pub unsafe extern "C" fn stint_verb_start(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let params: verbs::StartParams = unsafe { parse_params(params_json) }?;
        let store = store()?;
        runtime().block_on(verbs::start(&store, params))
    });
    0
}

/// Stop the running entry. No params.
///
/// # Safety
/// `out_json` must point at a valid `*mut c_char` slot.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_stop(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = store()?;
        runtime().block_on(verbs::stop(&store))
    });
    0
}

/// Return the currently-running entry as `Option<EntryView>` (null if idle).
///
/// # Safety
/// `out_json` must point at a valid `*mut c_char` slot.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_current(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = store()?;
        runtime().block_on(verbs::current(&store))
    });
    0
}

/// List entries matching the given `EntryFilter` (JSON-encoded).
///
/// # Safety
/// `filter_json` is a NUL-terminated JSON string (use `"{}"` for no filter).
#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_entries(
    filter_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let filter: verbs::EntryFilter = unsafe { parse_params(filter_json) }?;
        let store = store()?;
        runtime().block_on(verbs::list_entries(&store, filter))
    });
    0
}

/// List all known projects.
///
/// # Safety
/// `out_json` must point at a valid `*mut c_char` slot.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_projects(out_json: *mut *mut c_char) -> i32 {
    ffi_body(out_json, || {
        let store = store()?;
        runtime().block_on(verbs::list_projects(&store))
    });
    0
}

/// JSON shape for the `stint_verb_list_tasks` param: `{"project_id": "..."}` or `{}`.
#[derive(Deserialize)]
struct ListTasksParams {
    #[serde(default)]
    project_id: Option<String>,
}

/// List tasks for the given project, or all tasks if `project_id` is omitted.
///
/// # Safety
/// `params_json` is a NUL-terminated JSON string.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_list_tasks(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: ListTasksParams = unsafe { parse_params(params_json) }?;
        let store = store()?;
        runtime().block_on(verbs::list_tasks(&store, p.project_id))
    });
    0
}

/// JSON shape: `{"local_uuid": "...", "patch": <EntryPatch>}`.
#[derive(Deserialize)]
struct UpdateEntryParams {
    local_uuid: String,
    patch: verbs::EntryPatch,
}

/// Apply an `EntryPatch` to the entry identified by `local_uuid`.
///
/// # Safety
/// `params_json` is a NUL-terminated JSON string.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_update_entry(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: UpdateEntryParams = unsafe { parse_params(params_json) }?;
        let store = store()?;
        runtime().block_on(verbs::update_entry(&store, &p.local_uuid, p.patch))
    });
    0
}

/// JSON shape: `{"local_uuid": "..."}`.
#[derive(Deserialize)]
struct DeleteEntryParams {
    local_uuid: String,
}

/// Delete the entry identified by `local_uuid`. Envelope `ok` is `{}` on success.
///
/// # Safety
/// `params_json` is a NUL-terminated JSON string.
#[no_mangle]
pub unsafe extern "C" fn stint_verb_delete_entry(
    params_json: *const c_char,
    out_json: *mut *mut c_char,
) -> i32 {
    ffi_body(out_json, || {
        let p: DeleteEntryParams = unsafe { parse_params(params_json) }?;
        let store = store()?;
        runtime().block_on(verbs::delete_entry(&store, &p.local_uuid))?;
        Ok::<_, Error>(serde_json::json!({}))
    });
    0
}

// ---- settings -------------------------------------------------------

/// Opaque key/value setter. Returns 0 on success, non-zero on failure
/// (most commonly a closed DB or malformed UTF-8).
///
/// # Safety
/// Both pointers must reference NUL-terminated UTF-8 strings.
#[no_mangle]
pub unsafe extern "C" fn stint_settings_set(key: *const c_char, value: *const c_char) -> i32 {
    if key.is_null() || value.is_null() {
        return -2;
    }
    let result = panic::catch_unwind(|| -> Result<(), Error> {
        let key = unsafe { CStr::from_ptr(key) }
            .to_str()
            .map_err(|e| Error::Invariant(format!("non-utf8 key: {e}")))?;
        let value = unsafe { CStr::from_ptr(value) }
            .to_str()
            .map_err(|e| Error::Invariant(format!("non-utf8 value: {e}")))?;
        let store = store()?;
        let settings = Settings::new(store);
        runtime().block_on(settings.set(key, value))
    });
    match result {
        Ok(Ok(())) => 0,
        _ => 1,
    }
}

/// Opaque value getter. Returns 0 with `*out_json` set to a malloc'd
/// CString on success, or to NULL if the key is absent. Returns non-zero
/// on internal failure.
///
/// # Safety
/// `key` must be NUL-terminated UTF-8. `out_json` must be a valid pointer.
/// Caller must free `*out_json` via [`stint_free_string`].
#[no_mangle]
pub unsafe extern "C" fn stint_settings_get(key: *const c_char, out_json: *mut *mut c_char) -> i32 {
    if key.is_null() || out_json.is_null() {
        return -2;
    }
    unsafe { *out_json = ptr::null_mut() };
    let result = panic::catch_unwind(|| -> Result<Option<String>, Error> {
        let key = unsafe { CStr::from_ptr(key) }
            .to_str()
            .map_err(|e| Error::Invariant(format!("non-utf8 key: {e}")))?;
        let store = store()?;
        let settings = Settings::new(store);
        runtime().block_on(settings.get(key))
    });
    match result {
        Ok(Ok(Some(v))) => {
            if let Ok(c) = CString::new(v) {
                unsafe { *out_json = c.into_raw() };
            }
            0
        }
        Ok(Ok(None)) => 0,
        _ => 1,
    }
}

/// Delete a settings key. Idempotent — returns 0 even if the key didn't exist.
///
/// # Safety
/// `key` must be NUL-terminated UTF-8.
#[no_mangle]
pub unsafe extern "C" fn stint_settings_clear(key: *const c_char) -> i32 {
    if key.is_null() {
        return -2;
    }
    let result = panic::catch_unwind(|| -> Result<(), Error> {
        let key = unsafe { CStr::from_ptr(key) }
            .to_str()
            .map_err(|e| Error::Invariant(format!("non-utf8 key: {e}")))?;
        let store = store()?;
        let settings = Settings::new(store);
        runtime().block_on(settings.delete(key))
    });
    match result {
        Ok(Ok(())) => 0,
        _ => 1,
    }
}

// ---- log forwarder --------------------------------------------------

/// Forward a UTF-8 warning message into stint's tracing subscriber under
/// the `stint_intents` target. Best-effort — silently drops malformed
/// strings.
///
/// # Safety
/// `msg` must be NUL-terminated UTF-8, or NULL (no-op).
#[no_mangle]
pub unsafe extern "C" fn stint_log_warn(msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    if let Ok(s) = unsafe { CStr::from_ptr(msg) }.to_str() {
        tracing::warn!(target: "stint_intents", "{}", s);
    }
}

// ---- focus id (dlsym'd from Swift; null when framework absent) ------

type FocusIdFn = unsafe extern "C" fn(*mut *mut c_char) -> i32;
static FOCUS_ID_SYMBOL: OnceLock<Option<FocusIdFn>> = OnceLock::new();

unsafe fn lookup_focus_id() -> Option<FocusIdFn> {
    *FOCUS_ID_SYMBOL.get_or_init(|| {
        // Use RTLD_DEFAULT (null) so dlsym walks the global symbol table —
        // it will find swift_indexer_notify / stint_current_focus_id_swift
        // when the Swift framework is loaded into the same process, and
        // return null otherwise (CLI binaries, headless tests).
        let name = std::ffi::CString::new("stint_current_focus_id_swift").unwrap();
        let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
        if sym.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute::<*mut libc::c_void, FocusIdFn>(sym) })
        }
    })
}

/// Return the currently-active macOS Focus identifier via Swift bridge.
/// `*out_json` is set to a malloc'd CString on success, or NULL if no focus
/// is active (or the Swift framework isn't loaded). Returns 0 in both cases.
///
/// # Safety
/// `out_json` must be a valid `*mut c_char` slot. Caller frees via
/// [`stint_free_string`].
#[no_mangle]
pub unsafe extern "C" fn stint_current_focus_id(out_json: *mut *mut c_char) -> i32 {
    if out_json.is_null() {
        return -2;
    }
    unsafe { *out_json = ptr::null_mut() };
    if let Some(f) = unsafe { lookup_focus_id() } {
        unsafe { f(out_json) }
    } else {
        0
    }
}

// ---- indexer notify (Rust → Swift via dlsym) ------------------------

/// Categorizes the payload Rust hands to the Swift indexer. Numeric values
/// are a stable contract — see the spec's "Indexer lifecycle" section.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum IndexerKind {
    EntryStarted = 1,
    EntryStopped = 2,
    EntryUpdated = 3,
    EntryDeleted = 4,
    ProjectsReplaced = 5,
    TasksReplaced = 6,
}

type IndexerNotifyFn = unsafe extern "C" fn(i32, *const c_char);
static INDEXER_NOTIFY_SYMBOL: OnceLock<Option<IndexerNotifyFn>> = OnceLock::new();

unsafe fn lookup_indexer_notify() -> Option<IndexerNotifyFn> {
    *INDEXER_NOTIFY_SYMBOL.get_or_init(|| {
        let name = std::ffi::CString::new("swift_indexer_notify").unwrap();
        let sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
        if sym.is_null() {
            None
        } else {
            Some(unsafe { std::mem::transmute::<*mut libc::c_void, IndexerNotifyFn>(sym) })
        }
    })
}

/// Notify the Swift Spotlight indexer about a mutation. No-op when the
/// Swift framework isn't loaded (CLI builds, headless tests).
///
/// Call this from verb call sites and the pull worker after a successful
/// write. The payload is verb-specific JSON; see Swift's `Spotlight/SpotlightIndexer.swift`
/// `delta(kind:payload:)` for the decoders.
pub fn notify_indexer(kind: IndexerKind, payload_json: &str) {
    let Some(f) = (unsafe { lookup_indexer_notify() }) else {
        return;
    };
    let Ok(c) = CString::new(payload_json) else {
        return;
    };
    unsafe { f(kind as i32, c.as_ptr()) };
}
