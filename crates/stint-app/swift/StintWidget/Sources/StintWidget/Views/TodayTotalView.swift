import SwiftUI
import WidgetKit

struct TodayTotalView: View {
    let snapshot: WidgetSnapshot
    let size: WidgetFamily

    var body: some View {
        switch snapshot {
        case .todayTotal(let total, let byProject):
            VStack(alignment: .leading, spacing: 6) {
                Text(timeString(total))
                    .font(.system(size: 32, weight: .semibold, design: .rounded))
                    .monospacedDigit()
                Text("Today").font(.caption).foregroundStyle(.secondary)
                if size == .systemMedium {
                    ForEach(byProject.prefix(3), id: \.name) { item in
                        HStack {
                            Text(item.name).font(.caption).lineLimit(1)
                            Spacer()
                            Text(timeString(item.seconds)).font(.caption).monospacedDigit()
                        }
                    }
                }
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
        return "\(h)h \(m)m"
    }
}
