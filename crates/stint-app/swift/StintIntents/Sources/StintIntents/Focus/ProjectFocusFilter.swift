import AppIntents
import Foundation

/// macOS Focus filter that sets a default project for new Stint timers
/// while a Focus mode is active.
///
/// `perform()` is called by the OS on every focus activation that has
/// this filter configured. It does NOT fire on deactivation, so we store
/// a (focus_id, project_id) tuple and let `verbs::start` reconcile against
/// the currently-active focus at read time — see the spec's §6.3.
public struct ProjectFocusFilter: SetFocusFilterIntent {
    public static var title: LocalizedStringResource = "Default Project"
    public static var description = IntentDescription(
        "Set a default project for new Stint timers while this focus is on."
    )

    // SetFocusFilterIntent requires all parameters to be optional (Apple's
    // contract). If the user leaves the project unset, the filter no-ops.
    @Parameter(title: "Project")
    public var project: ProjectEntity?

    public init() {}

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "Default project: \(project?.name ?? "—")")
    }

    public func perform() async throws -> some IntentResult {
        // If the user activated this filter without selecting a project,
        // clear any previously-stored default. Otherwise persist a fresh
        // (focus_id, project_id) tuple — Rust's verbs::start fallback
        // reconciles against focus.last_seen_id at read time.
        guard let project = project else {
            try? FFIBridge.shared.settingsClear("focus.default_project")
            try? FFIBridge.shared.settingsClear("focus.last_seen_id")
            return .result()
        }
        let focusId = UUID().uuidString
        let payload = "\(focusId)\t\(project.id)"
        try FFIBridge.shared.settingsSet("focus.default_project", payload)
        try FFIBridge.shared.settingsSet("focus.last_seen_id", focusId)
        return .result()
    }
}
