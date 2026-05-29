/*
 * stint_core.h
 *
 * C ABI declarations for the StintIntents Swift framework. Mirrors the
 * extern "C" surface defined in `crates/stint-core/src/ffi.rs`.
 *
 * Every verb fn returns 0 (success — see `out_json` for the JSON envelope)
 * or -2 on misuse (null pointer where one was required). Envelopes are
 * either `{"ok": <T>}` or `{"err": {"code": <int>, "message": "<str>"}}`.
 * Error codes: 1=Invariant, 2=NotFound, 4=Serialization, 99=Internal,
 * -1=Panic (caught across the C ABI by `catch_unwind`).
 *
 * Memory ownership: every non-NULL `*out_json` was malloc'd by Rust via
 * `CString::into_raw` and MUST be freed by the caller via
 * `stint_free_string`. Passing NULL to `stint_free_string` is safe.
 */

#ifndef STINT_CORE_H
#define STINT_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- string lifecycle ---- */
void stint_free_string(char *ptr);

/* ---- verbs ---- */
int32_t stint_verb_start(const char *params_json, char **out_json);
int32_t stint_verb_stop(char **out_json);
int32_t stint_verb_current(char **out_json);
int32_t stint_verb_list_entries(const char *filter_json, char **out_json);
int32_t stint_verb_list_projects(char **out_json);
int32_t stint_verb_list_tasks(const char *params_json, char **out_json);
int32_t stint_verb_update_entry(const char *params_json, char **out_json);
int32_t stint_verb_delete_entry(const char *params_json, char **out_json);

/* ---- settings (opaque key/value strings) ---- */
int32_t stint_settings_set(const char *key, const char *value);
int32_t stint_settings_get(const char *key, char **out_json);
int32_t stint_settings_clear(const char *key);

/* ---- log forwarder (Swift → tracing) ---- */
void stint_log_warn(const char *msg);

/* ---- focus id (resolved via dlsym to Swift's stint_current_focus_id_swift) ---- */
/* `*out_json` is NULL when no focus is active (or framework not loaded). */
int32_t stint_current_focus_id(char **out_json);

/*
 * Swift exports the following symbols; Rust looks them up via dlsym
 * (RTLD_DEFAULT walks the global symbol table when both Rust and Swift
 * are loaded in the same process). They are listed here for reference.
 */
/*
 * int32_t stint_intents_init(void);
 *     Called once from Tauri's setup() hook. Triggers the framework load
 *     (first FFI symbol reference), kicks off Spotlight bulk refresh, and
 *     activates NSUserActivity for any currently running entry.
 *
 * void swift_indexer_notify(int32_t kind, const char *payload_json);
 *     IndexerKind values (stable contract):
 *         1 = EntryStarted     payload = EntryView JSON
 *         2 = EntryStopped     payload = EntryView JSON
 *         3 = EntryUpdated     payload = EntryView JSON
 *         4 = EntryDeleted     payload = {"local_uuid": "..."}
 *         5 = ProjectsReplaced payload = [ProjectView, ...]
 *         6 = TasksReplaced    payload = [TaskView, ...]
 *
 * int32_t stint_current_focus_id_swift(char **out_json);
 *     Backs `stint_current_focus_id`. Returns 0 with *out_json malloc'd
 *     (or NULL if no focus is active).
 */

#ifdef __cplusplus
}
#endif

#endif /* STINT_CORE_H */
