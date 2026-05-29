import CoreSpotlight
import Foundation
import Testing
@testable import StintIntents

@Suite("CSSearchableItem schema for Spotlight indexing")
struct SpotlightSchemaTests {
    @Test func entryItemHasCorrectDomainAndIdentifiers() {
        let dto = EntryDTO(
            localUuid: "u1",
            solidtimeId: nil,
            description: "client meeting",
            projectId: "p1",
            taskId: nil,
            billable: true,
            startAt: "2026-05-25T10:00:00Z",
            endAt: "2026-05-25T11:00:00Z",
            source: "test"
        )
        let item = SpotlightIndexer.shared.makeEntryItem(EntryEntity(from: dto))
        #expect(item.uniqueIdentifier == "u1")
        #expect(item.domainIdentifier == "tech.reyem.stint.entry")
        #expect(item.attributeSet.title == "client meeting")
        #expect(item.attributeSet.keywords?.contains("stint") == true)
    }

    @Test func projectItemHasCorrectDomainAndIdentifiers() {
        let dto = ProjectDTO(
            solidtimeId: "p1",
            name: "Acme",
            color: nil,
            clientId: nil,
            archived: false
        )
        let item = SpotlightIndexer.shared.makeProjectItem(dto)
        #expect(item.uniqueIdentifier == "p1")
        #expect(item.domainIdentifier == "tech.reyem.stint.project")
        #expect(item.attributeSet.title == "Acme")
        #expect(item.attributeSet.keywords?.contains("project") == true)
        #expect(item.attributeSet.keywords?.contains("Acme") == true)
    }

    @Test func taskItemHasCorrectDomainAndIdentifiers() {
        let dto = TaskDTO(
            solidtimeId: "t1",
            projectId: "p1",
            name: "Fix bug",
            done: false
        )
        let item = SpotlightIndexer.shared.makeTaskItem(dto)
        #expect(item.uniqueIdentifier == "t1")
        #expect(item.domainIdentifier == "tech.reyem.stint.task")
        #expect(item.attributeSet.title == "Fix bug")
        #expect(item.attributeSet.keywords?.contains("task") == true)
    }
}
