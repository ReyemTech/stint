import AppIntents
import Foundation

public struct EntryQuery: EntityQuery, EntityStringQuery {
    public init() {}

    public func entities(for identifiers: [EntryEntity.ID]) async throws -> [EntryEntity] {
        // No direct lookup-by-id verb; fetch a wide window and filter client-side.
        let all = try FFIBridge.shared
            .listEntries(EntryFilter(limit: 500))
            .map(EntryEntity.init(from:))
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [EntryEntity] {
        try FFIBridge.shared
            .listEntries(EntryFilter(limit: 20))
            .map(EntryEntity.init(from:))
    }

    public func entities(matching string: String) async throws -> [EntryEntity] {
        let q = string.lowercased()
        return try FFIBridge.shared
            .listEntries(EntryFilter(limit: 200))
            .map(EntryEntity.init(from:))
            .filter { $0.entryDescription.lowercased().contains(q) }
    }
}
