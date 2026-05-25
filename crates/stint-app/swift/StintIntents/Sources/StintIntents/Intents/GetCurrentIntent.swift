import AppIntents
import Foundation

public struct GetCurrentIntent: AppIntent {
    public static var title: LocalizedStringResource = "Current Timer"
    public static var description = IntentDescription("Show the currently running Stint timer.")

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog & ReturnsValue<EntryEntity?> {
        guard let entry = try FFIBridge.shared.current() else {
            return .result(value: nil, dialog: "No active timer.")
        }
        let entity = EntryEntity(from: entry)
        return .result(value: entity, dialog: "You're tracking '\(entry.description)'.")
    }
}
