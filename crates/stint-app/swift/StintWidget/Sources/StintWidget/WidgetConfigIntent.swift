import AppIntents
import WidgetKit

enum WidgetKind: String, AppEnum, CaseIterable {
    case runningTimer
    case todayTotal
    case weekProject

    static var typeDisplayRepresentation: TypeDisplayRepresentation = "Stint widget type"

    static var caseDisplayRepresentations: [WidgetKind : DisplayRepresentation] = [
        .runningTimer: "Running Timer",
        .todayTotal:   "Today Total",
        .weekProject:  "This-Week Project",
    ]
}

struct WidgetProjectEntity: AppEntity {
    static var typeDisplayRepresentation: TypeDisplayRepresentation = "Project"
    static var defaultQuery = WidgetProjectQuery()

    let id: String
    let name: String

    var displayRepresentation: DisplayRepresentation {
        DisplayRepresentation(title: "\(name)")
    }
}

struct WidgetProjectQuery: EntityQuery {
    func entities(for identifiers: [String]) async throws -> [WidgetProjectEntity] {
        let all = try await fetchProjects()
        return all.filter { identifiers.contains($0.id) }
    }
    func suggestedEntities() async throws -> [WidgetProjectEntity] {
        try await fetchProjects()
    }

    private func fetchProjects() async throws -> [WidgetProjectEntity] {
        let port = try PortDiscovery.read()
        let url = URL(string: "http://127.0.0.1:\(port)/v1/projects")!
        let (data, _) = try await URLSession.shared.data(from: url)
        return try JSONDecoder().decode([ProjectDTO].self, from: data)
            .filter { !$0.archived }
            .map { WidgetProjectEntity(id: $0.solidtime_id, name: $0.name) }
    }
}

struct WidgetConfigIntent: WidgetConfigurationIntent {
    static var title: LocalizedStringResource = "Configure Stint Widget"

    @Parameter(title: "Show", default: .runningTimer)
    var kind: WidgetKind

    @Parameter(title: "Project")
    var project: WidgetProjectEntity?
}
