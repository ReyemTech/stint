import Foundation

struct ProjectDTO: Codable {
    let solidtime_id: String
    let name: String
    let color: String?
    let client_id: String?
    let archived: Bool
}
