import AppIntents
import Foundation

public struct TaskEntity: AppEntity, Identifiable, Sendable {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Task")
    public static var defaultQuery = TaskQuery()

    public let id: String
    public let projectId: String
    public let name: String
    public let done: Bool

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(name)", subtitle: "Task")
    }

    public init(from dto: TaskDTO) {
        self.id = dto.solidtimeId
        self.projectId = dto.projectId
        self.name = dto.name
        self.done = dto.done
    }

    public init(id: String, projectId: String, name: String, done: Bool = false) {
        self.id = id
        self.projectId = projectId
        self.name = name
        self.done = done
    }
}
