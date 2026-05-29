import AppIntents
import Foundation

public struct EntryEntity: AppEntity, Identifiable, Sendable {
    public static var typeDisplayRepresentation = TypeDisplayRepresentation(name: "Time Entry")
    public static var defaultQuery = EntryQuery()

    public let id: String  // local_uuid
    public let entryDescription: String
    public let projectId: String?
    public let taskId: String?
    public let billable: Bool
    public let startAt: Date
    public let endAt: Date?

    public var duration: Measurement<UnitDuration> {
        let end = endAt ?? Date()
        return Measurement(value: end.timeIntervalSince(startAt), unit: .seconds)
    }

    public var displayRepresentation: DisplayRepresentation {
        let fmt = ISO8601DateFormatter()
        let mins = Int(duration.converted(to: .minutes).value)
        return DisplayRepresentation(
            title: "\(entryDescription)",
            subtitle: "\(fmt.string(from: startAt)) · \(mins)m"
        )
    }

    public init(from dto: EntryDTO) {
        self.id = dto.localUuid
        self.entryDescription = dto.description
        self.projectId = dto.projectId
        self.taskId = dto.taskId
        self.billable = dto.billable
        let fmt = ISO8601DateFormatter()
        self.startAt = fmt.date(from: dto.startAt) ?? Date()
        self.endAt = dto.endAt.flatMap { fmt.date(from: $0) }
    }
}
