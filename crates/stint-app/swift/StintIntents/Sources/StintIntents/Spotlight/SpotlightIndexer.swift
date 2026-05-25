import CoreSpotlight
import Foundation
import UniformTypeIdentifiers

/// Mirrors the Rust IndexerKind contract — see stint_core::ffi::IndexerKind.
public enum IndexerKind: Int32 {
    case entryStarted = 1
    case entryStopped = 2
    case entryUpdated = 3
    case entryDeleted = 4
    case projectsReplaced = 5
    case tasksReplaced = 6
}

/// Maintains the CSSearchableIndex for entries / projects / tasks.
///
/// - **Bulk refresh** on app launch (uses `indexSearchableItems` which has
///   upsert semantics on `uniqueIdentifier` — no delete-first needed).
/// - **Delta updates** triggered by Rust verb call sites via the
///   `swift_indexer_notify` @_cdecl symbol. Each update dispatches to a
///   background queue so the Rust caller isn't blocked.
public final class SpotlightIndexer: @unchecked Sendable {
    public static let shared = SpotlightIndexer()

    private static let entryDomain = "tech.reyem.stint.entry"
    private static let projectDomain = "tech.reyem.stint.project"
    private static let taskDomain = "tech.reyem.stint.task"

    private let bridge: Bridge

    public init(bridge: Bridge = FFIBridge.shared) {
        self.bridge = bridge
    }

    // MARK: - Public API

    /// Re-fetch every entry/project/task from stint-core and reindex.
    /// Idempotent: existing items with matching uniqueIdentifier are upserted.
    public func bulkRefresh() {
        Task.detached(priority: .background) { [self] in
            refreshEntries()
            refreshProjects()
            refreshTasks()
        }
    }

    /// Apply a delta the Rust side pushed in. Decodes the payload per kind
    /// and dispatches the index/delete call to a background queue.
    public func delta(kind: IndexerKind, payload: String) {
        Task.detached(priority: .background) { [self] in
            do {
                switch kind {
                case .entryStarted, .entryStopped, .entryUpdated:
                    let dto = try JSONDecoder().decode(EntryDTO.self, from: Data(payload.utf8))
                    upsertEntry(EntryEntity(from: dto))
                case .entryDeleted:
                    struct P: Decodable { let local_uuid: String }
                    let p = try JSONDecoder().decode(P.self, from: Data(payload.utf8))
                    deleteEntry(localUuid: p.local_uuid)
                case .projectsReplaced:
                    refreshProjects()
                case .tasksReplaced:
                    refreshTasks()
                }
            } catch {
                bridge.logWarn("spotlight delta decode failed: \(error)")
            }
        }
    }

    // MARK: - Entries

    private func refreshEntries() {
        do {
            let entries = try bridge.listEntries(EntryFilter(limit: nil))
                .map(EntryEntity.init(from:))
            let items = entries.map(makeEntryItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error {
                    bridge.logWarn("spotlight refreshEntries failed: \(error)")
                }
            }
        } catch {
            bridge.logWarn("spotlight refreshEntries fetch failed: \(error)")
        }
    }

    public func upsertEntry(_ entry: EntryEntity) {
        let item = makeEntryItem(entry)
        CSSearchableIndex.default().indexSearchableItems([item]) { [bridge] error in
            if let error = error {
                bridge.logWarn("spotlight upsertEntry failed: \(error)")
            }
        }
    }

    public func deleteEntry(localUuid: String) {
        CSSearchableIndex.default()
            .deleteSearchableItems(withIdentifiers: [localUuid]) { [bridge] error in
                if let error = error {
                    bridge.logWarn("spotlight deleteEntry failed: \(error)")
                }
            }
    }

    public func makeEntryItem(_ entry: EntryEntity) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = entry.entryDescription
        let mins = Int(entry.duration.converted(to: .minutes).value)
        let fmt = DateFormatter()
        fmt.dateStyle = .medium
        fmt.timeStyle = .short
        attrs.contentDescription = "\(fmt.string(from: entry.startAt)) · \(mins)m"
        attrs.keywords = ["stint", "timer"]
        if let projectId = entry.projectId {
            attrs.containerIdentifier = projectId
        }
        return CSSearchableItem(
            uniqueIdentifier: entry.id,
            domainIdentifier: Self.entryDomain,
            attributeSet: attrs
        )
    }

    // MARK: - Projects

    private func refreshProjects() {
        do {
            let projects = try bridge.listProjects()
            let items = projects.map(makeProjectItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error {
                    bridge.logWarn("spotlight refreshProjects failed: \(error)")
                }
            }
        } catch {
            bridge.logWarn("spotlight refreshProjects fetch failed: \(error)")
        }
    }

    public func makeProjectItem(_ project: ProjectDTO) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = project.name
        attrs.contentDescription = "Project"
        attrs.keywords = ["stint", "project", project.name]
        return CSSearchableItem(
            uniqueIdentifier: project.solidtimeId,
            domainIdentifier: Self.projectDomain,
            attributeSet: attrs
        )
    }

    // MARK: - Tasks

    private func refreshTasks() {
        do {
            let tasks = try bridge.listTasks(projectId: nil)
            let items = tasks.map(makeTaskItem)
            CSSearchableIndex.default().indexSearchableItems(items) { [bridge] error in
                if let error = error {
                    bridge.logWarn("spotlight refreshTasks failed: \(error)")
                }
            }
        } catch {
            bridge.logWarn("spotlight refreshTasks fetch failed: \(error)")
        }
    }

    public func makeTaskItem(_ task: TaskDTO) -> CSSearchableItem {
        let attrs = CSSearchableItemAttributeSet(contentType: UTType.text)
        attrs.title = task.name
        attrs.contentDescription = "Task in project \(task.projectId)"
        attrs.keywords = ["stint", "task", task.name]
        return CSSearchableItem(
            uniqueIdentifier: task.solidtimeId,
            domainIdentifier: Self.taskDomain,
            attributeSet: attrs
        )
    }
}
