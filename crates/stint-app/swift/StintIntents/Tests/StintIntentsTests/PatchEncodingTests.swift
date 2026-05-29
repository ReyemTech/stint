import Foundation
import Testing
@testable import StintIntents

@Suite("EntryPatch 3-way nullable encoding")
struct PatchEncodingTests {
    private func encodeJSON<T: Encodable>(_ value: T) throws -> String {
        let data = try JSONEncoder().encode(value)
        return String(decoding: data, as: UTF8.self)
    }

    @Test func unchangedFieldIsAbsent() throws {
        let patch = EntryPatch()
        let json = try encodeJSON(patch)
        #expect(!json.contains("project_id"))
        #expect(!json.contains("task_id"))
        #expect(!json.contains("end_at"))
    }

    @Test func clearProjectIdEncodesAsNull() throws {
        let patch = EntryPatch(projectId: .clear)
        let json = try encodeJSON(patch)
        #expect(json.contains("\"project_id\":null"))
    }

    @Test func setProjectIdEncodesAsValue() throws {
        let patch = EntryPatch(projectId: .set("p1"))
        let json = try encodeJSON(patch)
        #expect(json.contains("\"project_id\":\"p1\""))
    }

    @Test func descriptionSetEncodesPlain() throws {
        let patch = EntryPatch(description: "new desc")
        let json = try encodeJSON(patch)
        #expect(json.contains("\"description\":\"new desc\""))
    }

    @Test func multipleFieldsCombine() throws {
        let patch = EntryPatch(
            description: "d",
            projectId: .set("p1"),
            taskId: .clear,
            billable: true,
            endAt: .set("2026-05-25T11:00:00Z")
        )
        let json = try encodeJSON(patch)
        #expect(json.contains("\"description\":\"d\""))
        #expect(json.contains("\"project_id\":\"p1\""))
        #expect(json.contains("\"task_id\":null"))
        #expect(json.contains("\"billable\":true"))
        #expect(json.contains("\"end_at\":\"2026-05-25T11:00:00Z\""))
    }
}
