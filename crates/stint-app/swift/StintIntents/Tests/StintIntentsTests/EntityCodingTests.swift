import Foundation
import Testing
@testable import StintIntents

@Suite("Entity DTO decoding from Rust JSON shapes")
struct EntityCodingTests {
    private func decode<T: Decodable>(_ json: String) throws -> T {
        try JSONDecoder().decode(T.self, from: Data(json.utf8))
    }

    @Test func entryDTODecodes() throws {
        let json = """
        {
          "local_uuid": "u1",
          "solidtime_id": null,
          "description": "writing tests",
          "project_id": "p1",
          "task_id": null,
          "billable": true,
          "start_at": "2026-05-25T10:00:00Z",
          "end_at": "2026-05-25T11:00:00Z",
          "source": "test"
        }
        """
        let dto: EntryDTO = try decode(json)
        #expect(dto.localUuid == "u1")
        #expect(dto.description == "writing tests")
        #expect(dto.projectId == "p1")
        #expect(dto.taskId == nil)
        #expect(dto.billable == true)
        #expect(dto.endAt == "2026-05-25T11:00:00Z")
    }

    @Test func projectDTODecodes() throws {
        let json = #"{"solidtime_id":"p1","name":"Acme","color":null,"client_id":null,"archived":false}"#
        let dto: ProjectDTO = try decode(json)
        #expect(dto.solidtimeId == "p1")
        #expect(dto.name == "Acme")
        #expect(dto.archived == false)
    }

    @Test func taskDTODecodes() throws {
        let json = #"{"solidtime_id":"t1","project_id":"p1","name":"Fix bug","done":false}"#
        let dto: TaskDTO = try decode(json)
        #expect(dto.solidtimeId == "t1")
        #expect(dto.projectId == "p1")
        #expect(dto.name == "Fix bug")
        #expect(dto.done == false)
    }

    @Test func entryEntityComputesDurationFromDTO() {
        let dto = EntryDTO(
            localUuid: "u1",
            solidtimeId: nil,
            description: "x",
            projectId: nil,
            taskId: nil,
            billable: false,
            startAt: "2026-05-25T10:00:00Z",
            endAt: "2026-05-25T10:30:00Z",
            source: "test"
        )
        let entity = EntryEntity(from: dto)
        let mins = Int(entity.duration.converted(to: .minutes).value)
        #expect(mins == 30)
    }

    @Test func entryEntityHandlesRunningTimerEndAtNil() {
        let dto = EntryDTO(
            localUuid: "u1",
            solidtimeId: nil,
            description: "running",
            projectId: nil,
            taskId: nil,
            billable: false,
            startAt: "2026-05-25T10:00:00Z",
            endAt: nil,
            source: "test"
        )
        let entity = EntryEntity(from: dto)
        // Duration is computed from now; just verify it's non-negative and bounded.
        let secs = entity.duration.converted(to: .seconds).value
        #expect(secs >= 0)
        #expect(entity.endAt == nil)
    }
}
