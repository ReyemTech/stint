import AppIntents
import Foundation

public struct ProjectEntity: AppEntity, Identifiable, Sendable {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Project")
    public static var defaultQuery = ProjectQuery()

    public let id: String
    public let name: String
    public let clientName: String?
    public let archived: Bool

    public var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(
            title: "\(name)",
            subtitle: clientName.map { "Project · \($0)" } ?? "Project"
        )
    }

    public init(from dto: ProjectDTO) {
        self.id = dto.solidtimeId
        self.name = dto.name
        self.clientName = nil  // TODO: pipe through from Solidtime client cache
        self.archived = dto.archived
    }

    public init(id: String, name: String, clientName: String? = nil, archived: Bool = false) {
        self.id = id
        self.name = name
        self.clientName = clientName
        self.archived = archived
    }
}
