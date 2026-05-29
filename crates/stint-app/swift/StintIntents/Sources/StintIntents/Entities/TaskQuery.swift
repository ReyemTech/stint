import AppIntents
import Foundation

public struct TaskQuery: EntityQuery, EntityStringQuery {
    public init() {}

    public func entities(for identifiers: [TaskEntity.ID]) async throws -> [TaskEntity] {
        let all = try FFIBridge.shared.listTasks(projectId: nil).map(TaskEntity.init(from:))
        return all.filter { identifiers.contains($0.id) }
    }

    public func suggestedEntities() async throws -> [TaskEntity] {
        try FFIBridge.shared
            .listTasks(projectId: nil)
            .filter { !$0.done }
            .map(TaskEntity.init(from:))
    }

    public func entities(matching string: String) async throws -> [TaskEntity] {
        let q = string.lowercased()
        return try FFIBridge.shared
            .listTasks(projectId: nil)
            .filter { !$0.done && $0.name.lowercased().contains(q) }
            .map(TaskEntity.init(from:))
    }
}
