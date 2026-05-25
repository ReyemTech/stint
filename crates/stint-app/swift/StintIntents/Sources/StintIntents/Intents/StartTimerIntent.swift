import AppIntents
import Foundation

public struct StartTimerIntent: AppIntent {
    public static var title: LocalizedStringResource = "Start Timer"
    public static var description = IntentDescription("Start tracking time in Stint.")

    @Parameter(title: "Description", requestValueDialog: "What are you working on?")
    public var entryDescription: String

    @Parameter(title: "Project")
    public var project: ProjectEntity?

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<EntryEntity> {
        let entry = try FFIBridge.shared.start(
            StartParams(
                description: entryDescription,
                projectId: project?.id,
                source: "intent"
            )
        )
        let entity = EntryEntity(from: entry)
        let projectName = project?.name ?? "no project"
        return .result(value: entity, dialog: "Tracking '\(entryDescription)' on \(projectName).")
    }
}
