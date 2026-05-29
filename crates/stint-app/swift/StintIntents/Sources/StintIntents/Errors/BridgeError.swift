import Foundation

/// Maps the stable error-code contract in stint-core's FFI envelope into a
/// typed Swift error. App Intent `perform()` bodies throw these; App
/// Intents surfaces the `errorDescription` as the spoken dialog.
///
/// Codes (do not renumber — public contract):
///   1 = Invariant       (e.g., "a timer is already running")
///   2 = NotFound        (lookup miss)
///   3 = Conflict        (reserved — no current Error variant maps here)
///   4 = Serialization   (malformed JSON across the C ABI)
///   99 = Internal       (any other typed Error variant)
///   -1 = Panic          (catch_unwind caught a panic across FFI)
public enum BridgeError: LocalizedError {
    case invariant(String)
    case notFound(String)
    case conflict(String)
    case serialization(String)
    case `internal`(String)
    case panic(String)

    public static func from(code: Int32, message: String) -> BridgeError {
        switch code {
        case 1: return .invariant(message)
        case 2: return .notFound(message)
        case 3: return .conflict(message)
        case 4: return .serialization(message)
        case -1: return .panic(message)
        default: return .internal(message)
        }
    }

    public var errorDescription: String? {
        switch self {
        case .invariant(let m), .notFound(let m):
            return m
        case .conflict:
            return "That conflicts with an existing entry."
        case .serialization:
            return "Couldn't read the request."
        case .internal:
            return "Stint hit an internal error. Check the app."
        case .panic:
            return "Stint encountered an unexpected error."
        }
    }
}
