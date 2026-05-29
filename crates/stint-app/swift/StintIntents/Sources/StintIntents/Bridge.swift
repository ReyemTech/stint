import Foundation

// MARK: - C ABI declarations (symbols resolved at app-load time)
//
// We forward-declare the stint-core C functions via @_silgen_name rather
// than importing a clang module. Reason: a single-target Swift Package
// can't import a sibling clang module; introducing a second target would
// complicate framework bundling. The symbols are provided by libstint_core
// (statically linked into the Tauri-built Stint binary). The framework
// itself links with `-undefined dynamic_lookup`.
//
// Signatures MUST stay in sync with:
//   crates/stint-core/include/stint_core.h
//   crates/stint-core/src/ffi.rs

@_silgen_name("stint_free_string")
private func stint_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

@_silgen_name("stint_verb_start")
private func stint_verb_start(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_stop")
private func stint_verb_stop(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_current")
private func stint_verb_current(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_list_entries")
private func stint_verb_list_entries(_ filter: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_list_projects")
private func stint_verb_list_projects(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_list_tasks")
private func stint_verb_list_tasks(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_update_entry")
private func stint_verb_update_entry(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_verb_delete_entry")
private func stint_verb_delete_entry(_ params: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_settings_set")
private func stint_settings_set(_ key: UnsafePointer<CChar>?, _ value: UnsafePointer<CChar>?) -> Int32

@_silgen_name("stint_settings_get")
private func stint_settings_get(_ key: UnsafePointer<CChar>?, _ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

@_silgen_name("stint_settings_clear")
private func stint_settings_clear(_ key: UnsafePointer<CChar>?) -> Int32

@_silgen_name("stint_log_warn")
private func stint_log_warn(_ msg: UnsafePointer<CChar>?)

@_silgen_name("stint_current_focus_id")
private func stint_current_focus_id(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32

// MARK: - Envelope decoding

struct Envelope<T: Decodable>: Decodable {
    let ok: T?
    let err: EnvelopeErr?
}

struct EnvelopeErr: Decodable {
    let code: Int
    let message: String
}

// MARK: - DTOs (Rust shapes in verbs/types.rs, encoded snake_case)

public struct StartParams: Encodable {
    public var description: String
    public var projectId: String?
    public var taskId: String?
    public var billable: Bool
    public var startAt: String?
    public var source: String

    public init(
        description: String,
        projectId: String? = nil,
        taskId: String? = nil,
        billable: Bool = false,
        startAt: String? = nil,
        source: String = "intent"
    ) {
        self.description = description
        self.projectId = projectId
        self.taskId = taskId
        self.billable = billable
        self.startAt = startAt
        self.source = source
    }

    enum CodingKeys: String, CodingKey {
        case description, source, billable
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
    }
}

public struct EntryFilter: Encodable {
    public var since: String?
    public var until: String?
    public var projectId: String?
    public var limit: UInt32?

    public init(
        since: String? = nil,
        until: String? = nil,
        projectId: String? = nil,
        limit: UInt32? = nil
    ) {
        self.since = since
        self.until = until
        self.projectId = projectId
        self.limit = limit
    }

    enum CodingKeys: String, CodingKey {
        case since, until, limit
        case projectId = "project_id"
    }
}

/// 3-way nullable for EntryPatch fields. Encoded as absent / null / value.
public enum NullablePatch<T: Encodable>: Encodable {
    case unchanged
    case clear
    case set(T)

    public func encode(to encoder: Encoder) throws {
        // Container encodes only the value branch; the absent/clear branches
        // are handled by EntryPatch.encode below.
        var c = encoder.singleValueContainer()
        switch self {
        case .unchanged: try c.encodeNil()  // unreachable in practice
        case .clear: try c.encodeNil()
        case .set(let v): try c.encode(v)
        }
    }
}

public struct EntryPatch: Encodable {
    public var description: String?
    public var projectId: NullablePatch<String>
    public var taskId: NullablePatch<String>
    public var billable: Bool?
    public var startAt: String?
    public var endAt: NullablePatch<String>

    public init(
        description: String? = nil,
        projectId: NullablePatch<String> = .unchanged,
        taskId: NullablePatch<String> = .unchanged,
        billable: Bool? = nil,
        startAt: String? = nil,
        endAt: NullablePatch<String> = .unchanged
    ) {
        self.description = description
        self.projectId = projectId
        self.taskId = taskId
        self.billable = billable
        self.startAt = startAt
        self.endAt = endAt
    }

    enum CodingKeys: String, CodingKey {
        case description, billable
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
        case endAt = "end_at"
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        if let d = description { try c.encode(d, forKey: .description) }
        if let b = billable { try c.encode(b, forKey: .billable) }
        if let s = startAt { try c.encode(s, forKey: .startAt) }
        try encodeNullable(into: &c, key: .projectId, value: projectId)
        try encodeNullable(into: &c, key: .taskId, value: taskId)
        try encodeNullable(into: &c, key: .endAt, value: endAt)
    }

    private func encodeNullable<T: Encodable>(
        into c: inout KeyedEncodingContainer<CodingKeys>,
        key: CodingKeys,
        value: NullablePatch<T>
    ) throws {
        switch value {
        case .unchanged: return
        case .clear: try c.encodeNil(forKey: key)
        case .set(let v): try c.encode(v, forKey: key)
        }
    }
}

public struct EntryDTO: Decodable, Equatable, Sendable {
    public let localUuid: String
    public let solidtimeId: String?
    public let description: String
    public let projectId: String?
    public let taskId: String?
    public let billable: Bool
    public let startAt: String
    public let endAt: String?
    public let source: String

    enum CodingKeys: String, CodingKey {
        case description, billable, source
        case localUuid = "local_uuid"
        case solidtimeId = "solidtime_id"
        case projectId = "project_id"
        case taskId = "task_id"
        case startAt = "start_at"
        case endAt = "end_at"
    }
}

public struct ProjectDTO: Decodable, Equatable, Sendable {
    public let solidtimeId: String
    public let name: String
    public let color: String?
    public let clientId: String?
    public let archived: Bool

    enum CodingKeys: String, CodingKey {
        case name, color, archived
        case solidtimeId = "solidtime_id"
        case clientId = "client_id"
    }
}

public struct TaskDTO: Decodable, Equatable, Sendable {
    public let solidtimeId: String
    public let projectId: String
    public let name: String
    public let done: Bool

    enum CodingKeys: String, CodingKey {
        case name, done
        case solidtimeId = "solidtime_id"
        case projectId = "project_id"
    }
}

// MARK: - Bridge protocol (testable seam)

public protocol Bridge: Sendable {
    func start(_ params: StartParams) throws -> EntryDTO
    func stop() throws -> EntryDTO
    func current() throws -> EntryDTO?
    func listEntries(_ filter: EntryFilter) throws -> [EntryDTO]
    func listProjects() throws -> [ProjectDTO]
    func listTasks(projectId: String?) throws -> [TaskDTO]
    func updateEntry(localUuid: String, patch: EntryPatch) throws -> EntryDTO
    func deleteEntry(localUuid: String) throws

    func settingsSet(_ key: String, _ value: String) throws
    func settingsGet(_ key: String) throws -> String?
    func settingsClear(_ key: String) throws

    func logWarn(_ msg: String)
}

// MARK: - Production FFIBridge

/// Calls the C ABI in stint-core. Symbols are resolved at app-load time
/// (the framework is built with `-undefined dynamic_lookup`; the host
/// Stint binary provides the implementations via libstint_core).
public final class FFIBridge: Bridge, @unchecked Sendable {
    public static let shared = FFIBridge()

    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    public init() {}

    // ---- write helpers ----

    private func encodeParams<P: Encodable>(_ params: P) throws -> Data {
        try encoder.encode(params)
    }

    private func callWriting<P: Encodable, T: Decodable>(
        _ verb: (UnsafePointer<CChar>?, UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32,
        _ params: P
    ) throws -> T {
        let data = try encodeParams(params)
        let paramsString = String(decoding: data, as: UTF8.self)
        var out: UnsafeMutablePointer<CChar>?
        paramsString.withCString { ptr in
            _ = verb(ptr, &out)
        }
        return try decodeEnvelope(out)
    }

    private func callReading<T: Decodable>(
        _ verb: (UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?) -> Int32
    ) throws -> T {
        var out: UnsafeMutablePointer<CChar>?
        _ = verb(&out)
        return try decodeEnvelope(out)
    }

    private func decodeEnvelope<T: Decodable>(_ ptr: UnsafeMutablePointer<CChar>?) throws -> T {
        guard let ptr = ptr else {
            throw BridgeError.internal("null envelope pointer")
        }
        defer { stint_free_string(ptr) }
        let data = Data(bytes: ptr, count: strlen(ptr))
        let env = try decoder.decode(Envelope<T>.self, from: data)
        if let e = env.err {
            throw BridgeError.from(code: Int32(e.code), message: e.message)
        }
        guard let ok = env.ok else {
            throw BridgeError.internal("envelope missing both ok and err")
        }
        return ok
    }

    // ---- verbs ----

    public func start(_ params: StartParams) throws -> EntryDTO {
        try callWriting(stint_verb_start, params)
    }

    public func stop() throws -> EntryDTO {
        try callReading(stint_verb_stop)
    }

    public func current() throws -> EntryDTO? {
        // current returns Option<EntryView>; ok branch may legitimately be null.
        var out: UnsafeMutablePointer<CChar>?
        _ = stint_verb_current(&out)
        guard let ptr = out else { return nil }
        defer { stint_free_string(ptr) }
        let data = Data(bytes: ptr, count: strlen(ptr))
        struct OptionalEnvelope: Decodable {
            let ok: EntryDTO?
            let err: EnvelopeErr?
        }
        let env = try decoder.decode(OptionalEnvelope.self, from: data)
        if let e = env.err {
            throw BridgeError.from(code: Int32(e.code), message: e.message)
        }
        return env.ok
    }

    public func listEntries(_ filter: EntryFilter) throws -> [EntryDTO] {
        try callWriting(stint_verb_list_entries, filter)
    }

    public func listProjects() throws -> [ProjectDTO] {
        try callReading(stint_verb_list_projects)
    }

    public func listTasks(projectId: String?) throws -> [TaskDTO] {
        struct P: Encodable {
            let project_id: String?
        }
        return try callWriting(stint_verb_list_tasks, P(project_id: projectId))
    }

    public func updateEntry(localUuid: String, patch: EntryPatch) throws -> EntryDTO {
        struct P: Encodable {
            let local_uuid: String
            let patch: EntryPatch
        }
        return try callWriting(stint_verb_update_entry, P(local_uuid: localUuid, patch: patch))
    }

    public func deleteEntry(localUuid: String) throws {
        struct P: Encodable {
            let local_uuid: String
        }
        let _: [String: String] = try callWriting(stint_verb_delete_entry, P(local_uuid: localUuid))
    }

    // ---- settings ----

    public func settingsSet(_ key: String, _ value: String) throws {
        let rc = key.withCString { k in
            value.withCString { v in
                stint_settings_set(k, v)
            }
        }
        if rc != 0 {
            throw BridgeError.internal("settings_set rc=\(rc)")
        }
    }

    public func settingsGet(_ key: String) throws -> String? {
        var out: UnsafeMutablePointer<CChar>?
        let rc = key.withCString { k in stint_settings_get(k, &out) }
        if rc != 0 {
            throw BridgeError.internal("settings_get rc=\(rc)")
        }
        guard let ptr = out else { return nil }
        defer { stint_free_string(ptr) }
        return String(cString: ptr)
    }

    public func settingsClear(_ key: String) throws {
        let rc = key.withCString { k in stint_settings_clear(k) }
        if rc != 0 {
            throw BridgeError.internal("settings_clear rc=\(rc)")
        }
    }

    // ---- log ----

    public func logWarn(_ msg: String) {
        msg.withCString { stint_log_warn($0) }
    }
}

// Suppress the @unchecked Sendable warning: FFIBridge contains JSONEncoder and
// JSONDecoder which are not Sendable, but they are used in a serial manner by
// callers (each call constructs fresh encoded data, no shared mutable state).
