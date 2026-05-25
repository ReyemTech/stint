import AppIntents
import Foundation

public struct StopTimerIntent: AppIntent {
    public static var title: LocalizedStringResource = "Stop Timer"
    public static var description = IntentDescription("Stop the running Stint timer.")

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<EntryEntity> {
        let entry = try FFIBridge.shared.stop()
        let entity = EntryEntity(from: entry)
        let mins = Int(entity.duration.converted(to: .minutes).value)
        let projectLabel = entry.projectId.map { "project \($0)" } ?? "no project"
        return .result(value: entity, dialog: "Stopped. \(mins) minutes on \(projectLabel).")
    }
}
