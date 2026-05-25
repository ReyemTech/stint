import Foundation

/// Called once by Rust during Tauri's `setup()` hook. First call to a Swift
/// symbol forces the framework's lazy dylib load; this fn kicks off
/// Spotlight bulk refresh and NSUserActivity boot.
@_cdecl("stint_intents_init")
public func stint_intents_init() -> Int32 {
    SpotlightIndexer.shared.bulkRefresh()
    ActivityTracker.shared.boot()
    return 0
}

/// Called from Rust on every verb mutation + after pull-worker success.
///
/// Side effects:
/// - Updates the Spotlight index (entry upsert / delete; full project or
///   task refresh).
/// - For entry start/stop/update, also updates the NSUserActivity tracker
///   on the main actor.
@_cdecl("swift_indexer_notify")
public func swift_indexer_notify(_ kind: Int32, _ payloadPtr: UnsafePointer<CChar>?) {
    guard let payloadPtr = payloadPtr else { return }
    guard let k = IndexerKind(rawValue: kind) else { return }
    let payload = String(cString: payloadPtr)

    switch k {
    case .entryStarted:
        if let entry = decodeEntry(payload) {
            Task { @MainActor in
                ActivityTracker.shared.activate(entry: entry)
            }
        }
    case .entryStopped:
        Task { @MainActor in
            ActivityTracker.shared.invalidate()
        }
    case .entryUpdated:
        if let entry = decodeEntry(payload) {
            Task { @MainActor in
                ActivityTracker.shared.update(description: entry.entryDescription)
            }
        }
    default:
        break
    }

    SpotlightIndexer.shared.delta(kind: k, payload: payload)
}

/// Best-effort macOS Focus identifier accessor. Reads back the
/// `focus.last_seen_id` settings key that `ProjectFocusFilter.perform()`
/// writes — Apple doesn't expose a public "current focus id" API on macOS,
/// so this is our pragmatic proxy. Rust's `verbs::start` fallback
/// reconciles against the same key when picking up the focus default.
@_cdecl("stint_current_focus_id_swift")
public func stint_current_focus_id_swift(
    _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let out = out else { return -2 }
    out.pointee = nil
    do {
        if let id = try FFIBridge.shared.settingsGet("focus.last_seen_id"),
           !id.isEmpty {
            out.pointee = strdup(id)
        }
    } catch {
        // Best-effort — silently leave nil on lookup failure.
    }
    return 0
}

private func decodeEntry(_ payload: String) -> EntryEntity? {
    guard let data = payload.data(using: .utf8),
          let dto = try? JSONDecoder().decode(EntryDTO.self, from: data) else {
        return nil
    }
    return EntryEntity(from: dto)
}
