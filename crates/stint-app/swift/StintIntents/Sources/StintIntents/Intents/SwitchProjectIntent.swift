import AppIntents
import Foundation

public struct SwitchProjectIntent: AppIntent {
    public static var title: LocalizedStringResource = "Switch Project"
    public static var description = IntentDescription("Stop the current Stint timer and start a new one on a different project.")

    @Parameter(title: "Project")
    public var project: ProjectEntity

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog {
        guard let current = try FFIBridge.shared.current() else {
            throw BridgeError.invariant("No timer to switch from.")
        }
        _ = try FFIBridge.shared.stop()
        _ = try FFIBridge.shared.start(
            StartParams(
                description: current.description,
                projectId: project.id,
                source: "intent"
            )
        )
        return .result(dialog: "Switched to \(project.name).")
    }
}
