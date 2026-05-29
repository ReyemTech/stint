import AppIntents
import Foundation

public struct UpdateEntryIntent: AppIntent {
    public static var title: LocalizedStringResource = "Update Entry"
    public static var description = IntentDescription("Update fields on a Stint time entry.")

    @Parameter(title: "Entry")
    public var entry: EntryEntity

    @Parameter(title: "Description")
    public var entryDescription: String?

    @Parameter(title: "Project")
    public var project: ProjectEntity?

    @Parameter(title: "Billable")
    public var billable: Bool?

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<EntryEntity> {
        var patch = EntryPatch()
        if let d = entryDescription { patch.description = d }
        if let p = project { patch.projectId = .set(p.id) }
        if let b = billable { patch.billable = b }
        let updated = try FFIBridge.shared.updateEntry(localUuid: entry.id, patch: patch)
        return .result(value: EntryEntity(from: updated))
    }
}
