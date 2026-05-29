import Testing
@testable import StintIntents

@Suite("BridgeError envelope code mapping")
struct BridgeErrorTests {
    @Test func invariantMapsToCode1() {
        let err = BridgeError.from(code: 1, message: "timer already running")
        if case .invariant(let m) = err {
            #expect(m == "timer already running")
        } else {
            Issue.record("expected .invariant, got \(err)")
        }
        #expect(err.errorDescription == "timer already running")
    }

    @Test func notFoundMapsToCode2() {
        let err = BridgeError.from(code: 2, message: "no such uuid")
        if case .notFound(let m) = err {
            #expect(m == "no such uuid")
        } else {
            Issue.record("expected .notFound")
        }
        #expect(err.errorDescription == "no such uuid")
    }

    @Test func conflictMapsToCode3() {
        let err = BridgeError.from(code: 3, message: "overlap")
        if case .conflict = err {
        } else {
            Issue.record("expected .conflict")
        }
        #expect(err.errorDescription == "That conflicts with an existing entry.")
    }

    @Test func serializationMapsToCode4() {
        let err = BridgeError.from(code: 4, message: "bad json")
        if case .serialization = err {
        } else {
            Issue.record("expected .serialization")
        }
        #expect(err.errorDescription == "Couldn't read the request.")
    }

    @Test func panicMapsToNegative1() {
        let err = BridgeError.from(code: -1, message: "rust panic")
        if case .panic = err {
        } else {
            Issue.record("expected .panic")
        }
        #expect(err.errorDescription == "Stint encountered an unexpected error.")
    }

    @Test func unknownCodeMapsToInternal() {
        let err = BridgeError.from(code: 99, message: "unknown")
        if case .internal = err {
        } else {
            Issue.record("expected .internal")
        }
        let err2 = BridgeError.from(code: 7777, message: "other")
        if case .internal = err2 {
        } else {
            Issue.record("expected .internal for unknown code")
        }
    }
}
