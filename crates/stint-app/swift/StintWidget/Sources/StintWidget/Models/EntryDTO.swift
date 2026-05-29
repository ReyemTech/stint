import Foundation

struct EntryDTO: Codable {
    let local_uuid: String
    let solidtime_id: String?
    let description: String
    let project_id: String?
    let task_id: String?
    let billable: Bool
    let start_at: String
    let end_at: String?
    let source: String
}
