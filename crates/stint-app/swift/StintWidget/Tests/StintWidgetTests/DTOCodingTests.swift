import Testing
import Foundation
@testable import StintWidget

@Suite("DTO Coding")
struct DTOCodingTests {
    @Test func entryDecodes() throws {
        let json = #"{"local_uuid":"u1","solidtime_id":null,"description":"x","project_id":"p1","task_id":null,"billable":false,"start_at":"2026-05-27T10:00:00Z","end_at":null,"source":"test"}"#
        let dto = try JSONDecoder().decode(EntryDTO.self, from: Data(json.utf8))
        #expect(dto.local_uuid == "u1")
        #expect(dto.description == "x")
    }

    @Test func projectDecodes() throws {
        let json = #"{"solidtime_id":"p1","name":"Acme","color":null,"client_id":null,"archived":false}"#
        let dto = try JSONDecoder().decode(ProjectDTO.self, from: Data(json.utf8))
        #expect(dto.name == "Acme")
    }
}
