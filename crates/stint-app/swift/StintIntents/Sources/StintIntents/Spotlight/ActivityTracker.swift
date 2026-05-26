import Foundation

/// Maintains an NSUserActivity for the currently-running timer so Spotlight
/// shows it as a "live" tile and handoff is eligible.
public final class ActivityTracker: @unchecked Sendable {
    public static let shared = ActivityTracker()

    private static let activityType = "tech.reyem.stint.tracking"

    private var current: NSUserActivity?

    public init() {}

    /// Called once at framework init. Queries stint-core for any
    /// currently-running entry and activates an NSUserActivity for it.
    public func boot() {
        Task.detached(priority: .background) {
            do {
                if let entry = try FFIBridge.shared.current() {
                    let entity = EntryEntity(from: entry)
                    await MainActor.run {
                        Self.shared.activate(entry: entity)
                    }
                }
            } catch {
                FFIBridge.shared.logWarn("activitytracker boot failed: \(error)")
            }
        }
    }

    @MainActor
    public func activate(entry: EntryEntity) {
        let activity = NSUserActivity(activityType: Self.activityType)
        activity.title = "Tracking: \(entry.entryDescription)"
        activity.userInfo = ["uuid": entry.id]
        // webpageURL is what Spotlight + Handoff dispatch when the activity
        // is selected. Setting it to our deep-link scheme makes the existing
        // tauri-plugin-deep-link handler in stint-app/src/main.rs route to
        // the entry. Without this, macOS just launches the app with the
        // activity attached but no obvious dispatch target on the Tauri/Rust
        // side.
        activity.webpageURL = URL(string: "stint://entry/\(entry.id)")
        activity.isEligibleForSearch = true
        activity.isEligibleForHandoff = true
        // NSUserActivity.isEligibleForPrediction is iOS-only; no macOS equivalent.
        activity.becomeCurrent()
        self.current = activity
    }

    @MainActor
    public func update(description: String) {
        current?.title = "Tracking: \(description)"
    }

    @MainActor
    public func invalidate() {
        current?.invalidate()
        current = nil
    }
}
