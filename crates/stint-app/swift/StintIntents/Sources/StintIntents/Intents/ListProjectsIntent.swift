import AppIntents

public struct ListProjectsIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Projects"
    public static var description = IntentDescription("Fetch the list of Stint projects.")

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[ProjectEntity]> {
        let projects = try FFIBridge.shared.listProjects().map(ProjectEntity.init(from:))
        return .result(value: projects)
    }
}
