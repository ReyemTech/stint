import Foundation

enum PortDiscoveryError: Error {
    case fileNotFound
    case unreadable
    case parseError
}

struct PortDiscovery {
    static var defaultPath: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        return base.appendingPathComponent("stint/api.port")
    }

    static func read(from url: URL = defaultPath) throws -> UInt16 {
        guard FileManager.default.fileExists(atPath: url.path) else { throw PortDiscoveryError.fileNotFound }
        guard let data = try? Data(contentsOf: url),
              let s = String(data: data, encoding: .utf8) else { throw PortDiscoveryError.unreadable }
        guard let port = UInt16(s.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            throw PortDiscoveryError.parseError
        }
        return port
    }
}
