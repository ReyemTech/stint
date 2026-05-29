import WidgetKit
import SwiftUI

struct RunningTimerWidget: Widget {
    let kind: String = "tech.reyem.stint.widget"

    var body: some WidgetConfiguration {
        AppIntentConfiguration(kind: kind, intent: WidgetConfigIntent.self, provider: StintProvider()) { entry in
            WidgetRenderer(snapshot: entry.snapshot)
        }
        .configurationDisplayName("Stint")
        .description("Time-tracking dashboard for stint.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}

struct WidgetRenderer: View {
    let snapshot: WidgetSnapshot
    @Environment(\.widgetFamily) var family

    var body: some View {
        switch snapshot {
        case .runningTimer, .idleTimer, .unavailable:
            RunningTimerView(snapshot: snapshot, size: family)
        case .todayTotal:
            TodayTotalView(snapshot: snapshot, size: family)
        case .weekProject:
            WeekProjectView(snapshot: snapshot, size: family)
        }
    }
}
