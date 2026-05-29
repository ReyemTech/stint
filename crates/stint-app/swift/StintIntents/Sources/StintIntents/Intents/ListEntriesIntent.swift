import AppIntents
import Foundation

public struct ListEntriesIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Entries"
    public static var description = IntentDescription("Fetch Stint time entries.")

    @Parameter(title: "Since")
    public var since: Date?

    @Parameter(title: "Until")
    public var until: Date?

    @Parameter(title: "Project")
    public var project: ProjectEntity?

    @Parameter(title: "Limit", default: 100)
    public var limit: Int

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[EntryEntity]> {
        let fmt = ISO8601DateFormatter()
        let filter = EntryFilter(
            since: since.map { fmt.string(from: $0) },
            until: until.map { fmt.string(from: $0) },
            projectId: project?.id,
            limit: UInt32(max(0, limit))
        )
        let entries = try FFIBridge.shared.listEntries(filter).map(EntryEntity.init(from:))
        return .result(value: entries)
    }
}
