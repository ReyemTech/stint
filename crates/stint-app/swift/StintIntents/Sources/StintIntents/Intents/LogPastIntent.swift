import AppIntents
import Foundation

public struct LogPastIntent: AppIntent {
    public static var title: LocalizedStringResource = "Log Past Work"
    public static var description = IntentDescription("Retroactively log a past duration in Stint.")

    @Parameter(title: "Duration")
    public var duration: Measurement<UnitDuration>

    @Parameter(title: "Description", default: "Untitled")
    public var entryDescription: String

    @Parameter(title: "Project")
    public var project: ProjectEntity?

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog {
        let seconds = duration.converted(to: .seconds).value
        let startDate = Date(timeIntervalSinceNow: -seconds)
        let fmt = ISO8601DateFormatter()

        // Stop any running timer first so the backdated entry doesn't overlap.
        if (try? FFIBridge.shared.current()) != nil {
            _ = try? FFIBridge.shared.stop()
        }

        _ = try FFIBridge.shared.start(
            StartParams(
                description: entryDescription,
                projectId: project?.id,
                startAt: fmt.string(from: startDate),
                source: "intent"
            )
        )
        _ = try FFIBridge.shared.stop()

        let mins = Int(duration.converted(to: .minutes).value)
        let projectName = project?.name ?? "no project"
        return .result(dialog: "Logged \(mins) minutes on \(projectName).")
    }
}
