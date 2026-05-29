import Testing
import Foundation
@testable import StintWidget

@Suite("PortDiscovery")
struct PortDiscoveryTests {
    @Test func readsValidPortFile() throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("port-\(UUID()).txt")
        try "49792\n".write(to: tmp, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(at: tmp) }
        let port = try PortDiscovery.read(from: tmp)
        #expect(port == 49792)
    }

    @Test func errorsWhenFileMissing() {
        let nowhere = URL(fileURLWithPath: "/tmp/does-not-exist-\(UUID()).port")
        #expect(throws: PortDiscoveryError.self) {
            try PortDiscovery.read(from: nowhere)
        }
    }

    @Test func errorsOnGarbledFile() throws {
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("bad-\(UUID()).txt")
        try "not-a-number".write(to: tmp, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(at: tmp) }
        #expect(throws: PortDiscoveryError.self) {
            try PortDiscovery.read(from: tmp)
        }
    }
}
