import AppIntents

public struct ListTasksIntent: AppIntent {
    public static var title: LocalizedStringResource = "List Tasks"
    public static var description = IntentDescription("Fetch Stint tasks for a project.")

    @Parameter(title: "Project")
    public var project: ProjectEntity

    public init() {}

    public func perform() async throws -> some IntentResult & ReturnsValue<[TaskEntity]> {
        let tasks = try FFIBridge.shared
            .listTasks(projectId: project.id)
            .map(TaskEntity.init(from:))
        return .result(value: tasks)
    }
}
