import AppIntents
import Foundation

public struct ProjectQuery: EntityQuery, EntityStringQuery {
    public init() {}

    public func entities(for identifiers: [ProjectEntity.ID]) async throws -> [ProjectEntity] {
        let all = try FFIBridge.shared.listProjects().map(ProjectEntity.init(from:))
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [ProjectEntity] {
        try FFIBridge.shared
            .listProjects()
            .filter { !$0.archived }
            .map(ProjectEntity.init(from:))
    }

    public func entities(matching string: String) async throws -> [ProjectEntity] {
        let q = string.lowercased()
        return try FFIBridge.shared
            .listProjects()
            .filter { !$0.archived && $0.name.lowercased().contains(q) }
            .map(ProjectEntity.init(from:))
    }
}
