import Foundation

// Stub implementations of the stint-core C ABI for unit tests. The
// production framework uses @_silgen_name forward declarations resolved
// at app-load time against libstint_core; in the test bundle there is
// no host process providing those symbols, so the test target ships
// these no-op stubs to satisfy the dynamic loader.
//
// Tests that exercise actual FFI behavior would need to be integration
// tests linking against real libstint_core — that's outside the scope
// of these unit tests, which focus on pure-Swift logic (envelope
// decoding, entity coding, Spotlight schema construction).

@_cdecl("stint_free_string")
func stub_stint_free_string(_ ptr: UnsafeMutablePointer<CChar>?) {
    // No-op: production frees via CString::from_raw; stubs don't allocate.
    _ = ptr
}

@_cdecl("stint_verb_start")
func stub_stint_verb_start(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_stop")
func stub_stint_verb_stop(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_current")
func stub_stint_verb_current(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_list_entries")
func stub_stint_verb_list_entries(_ filter: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_list_projects")
func stub_stint_verb_list_projects(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_list_tasks")
func stub_stint_verb_list_tasks(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_update_entry")
func stub_stint_verb_update_entry(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_verb_delete_entry")
func stub_stint_verb_delete_entry(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_settings_set")
func stub_stint_settings_set(_ key: UnsafePointer<CChar>?, _ value: UnsafePointer<CChar>?) -> Int32 {
    return -2
}

@_cdecl("stint_settings_get")
func stub_stint_settings_get(_ key: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return -2
}

@_cdecl("stint_settings_clear")
func stub_stint_settings_clear(_ key: UnsafePointer<CChar>?) -> Int32 {
    return -2
}

@_cdecl("stint_log_warn")
func stub_stint_log_warn(_ msg: UnsafePointer<CChar>?) {
    // No-op
    _ = msg
}

@_cdecl("stint_current_focus_id")
func stub_stint_current_focus_id(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32 {
    if let out = out { out.pointee = nil }
    return 0
}
