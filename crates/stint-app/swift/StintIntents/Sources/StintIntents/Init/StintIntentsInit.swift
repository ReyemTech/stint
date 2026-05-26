import Foundation

/// Called once by Rust during Tauri's `setup()` hook.
///
/// Two responsibilities:
///
/// 1. **Keep the intent types alive.** Release LTO would otherwise dead-
///    strip the Swift type metadata records for our intent/entity types
///    because no Rust code reaches them — but Apple's App Intents indexer
///    scans the main binary's Mach-O for those exact records to discover
///    discoverable intents. Holding a reference to `.self` of each forces
///    the linker to keep them.
/// 2. Kick off Spotlight bulk refresh + NSUserActivity boot.
@_cdecl("stint_intents_init")
public func stint_intents_init() -> Int32 {
    // Anchor the intent + provider + filter type metadata so LTO doesn't
    // strip them. We don't actually use the array — just having the
    // expression in code that's reachable from an @_cdecl entry point is
    // enough.
    let anchors: [Any.Type] = [
        StartTimerIntent.self,
        StopTimerIntent.self,
        GetCurrentIntent.self,
        SwitchProjectIntent.self,
        LogPastIntent.self,
        ListEntriesIntent.self,
        ListProjectsIntent.self,
        ListTasksIntent.self,
        UpdateEntryIntent.self,
        DeleteEntryIntent.self,
        ProjectFocusFilter.self,
        StintAppShortcutsProvider.self,
        ProjectEntity.self,
        TaskEntity.self,
        EntryEntity.self,
        ProjectQuery.self,
        TaskQuery.self,
        EntryQuery.self,
    ]
    // Force a side effect the compiler can't elide so the anchor array is
    // really materialized. Without this `_ = anchors.count` would get
    // const-folded and the metadata records dropped again under LTO.
    NSLog("StintIntents: anchored %d types", anchors.count)

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
