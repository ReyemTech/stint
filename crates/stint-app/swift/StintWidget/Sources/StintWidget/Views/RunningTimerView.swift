import SwiftUI
import WidgetKit

struct RunningTimerView: View {
    let snapshot: WidgetSnapshot
    let size: WidgetFamily

    var body: some View {
        switch snapshot {
        case .runningTimer(let desc, let proj, let elapsed):
            VStack(alignment: .leading, spacing: 4) {
                Text(timeString(elapsed))
                    .font(.system(size: size == .systemSmall ? 28 : 36, weight: .semibold, design: .rounded))
                    .monospacedDigit()
                Text(desc).font(.callout).lineLimit(size == .systemSmall ? 1 : 2)
                if let p = proj {
                    Text(p).font(.caption).foregroundStyle(.secondary)
                }
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

        case .idleTimer:
            VStack(alignment: .leading, spacing: 4) {
                Text("No active timer").font(.callout)
                Text("Tap to open Stint").font(.caption).foregroundStyle(.secondary)
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

        case .unavailable:
            VStack(alignment: .leading, spacing: 4) {
                Text("Stint not running").font(.callout)
                Text("Launch the app and re-try").font(.caption).foregroundStyle(.secondary)
            }
            .padding()
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)

        default:
            EmptyView()
        }
    }

    private func timeString(_ secs: TimeInterval) -> String {
        let total = Int(secs)
        let h = total / 3600
        let m = (total % 3600) / 60
        return String(format: "%d:%02d", h, m)
    }
}
