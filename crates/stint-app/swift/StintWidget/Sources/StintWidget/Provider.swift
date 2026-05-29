import WidgetKit
import AppIntents
import Foundation

struct StintTimelineEntry: TimelineEntry {
    let date: Date
    let snapshot: WidgetSnapshot
}

enum WidgetSnapshot {
    case unavailable
    case runningTimer(description: String, projectName: String?, elapsedSecs: TimeInterval)
    case idleTimer
    case todayTotal(seconds: TimeInterval, byProject: [(name: String, seconds: TimeInterval)])
    case weekProject(projectName: String, seconds: TimeInterval, byDay: [TimeInterval])
}

struct StintProvider: AppIntentTimelineProvider {
    typealias Entry = StintTimelineEntry
    typealias Intent = WidgetConfigIntent

    func placeholder(in context: Context) -> StintTimelineEntry {
        StintTimelineEntry(date: Date(), snapshot: .runningTimer(description: "Loading…", projectName: nil, elapsedSecs: 0))
    }

    func snapshot(for configuration: WidgetConfigIntent, in context: Context) async -> StintTimelineEntry {
        await fetchOne(configuration: configuration)
    }

    func timeline(for configuration: WidgetConfigIntent, in context: Context) async -> Timeline<StintTimelineEntry> {
        let snapshot = await fetchSnapshot(configuration: configuration)
        let now = Date()
        switch snapshot {
        case .runningTimer:
            var entries: [StintTimelineEntry] = []
            for i in 0..<60 {
                entries.append(StintTimelineEntry(date: now.addingTimeInterval(TimeInterval(i * 60)), snapshot: snapshot))
            }
            return Timeline(entries: entries, policy: .atEnd)
        default:
            return Timeline(entries: [StintTimelineEntry(date: now, snapshot: snapshot)], policy: .after(now.addingTimeInterval(300)))
        }
    }

    private func fetchSnapshot(configuration: WidgetConfigIntent) async -> WidgetSnapshot {
        guard let port = try? PortDiscovery.read() else { return .unavailable }
        var request = URLRequest(url: URL(string: "http://127.0.0.1:\(port)/v1/current")!)
        request.timeoutInterval = 2
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return .unavailable
            }
            if data.count <= 4, let str = String(data: data, encoding: .utf8), str.trimmingCharacters(in: .whitespacesAndNewlines) == "null" {
                return .idleTimer
            }
            let entry = try JSONDecoder().decode(EntryDTO.self, from: data)
            let start = ISO8601DateFormatter().date(from: entry.start_at) ?? Date()
            return .runningTimer(
                description: entry.description,
                projectName: entry.project_id,
                elapsedSecs: Date().timeIntervalSince(start)
            )
        } catch {
            return .unavailable
        }
    }

    private func fetchOne(configuration: WidgetConfigIntent) async -> StintTimelineEntry {
        StintTimelineEntry(date: Date(), snapshot: await fetchSnapshot(configuration: configuration))
    }
}
