import AppIntents
import Foundation

public struct DeleteEntryIntent: AppIntent {
    public static var title: LocalizedStringResource = "Delete Entry"
    public static var description = IntentDescription("Delete a Stint time entry.")

    @Parameter(title: "Entry")
    public var entry: EntryEntity

    public init() {}

    public func perform() async throws -> some IntentResult & ProvidesDialog {
        try FFIBridge.shared.deleteEntry(localUuid: entry.id)
        return .result(dialog: "Deleted '\(entry.entryDescription)'.")
    }
}
