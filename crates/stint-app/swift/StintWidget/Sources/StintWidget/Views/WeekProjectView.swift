import SwiftUI
import WidgetKit

struct WeekProjectView: View {
    let snapshot: WidgetSnapshot
    let size: WidgetFamily

    var body: some View {
        switch snapshot {
        case .weekProject(let projectName, let total, let byDay):
            VStack(alignment: .leading, spacing: 6) {
                Text(projectName).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                Text(timeString(total))
                    .font(.system(size: 28, weight: .semibold, design: .rounded))
                    .monospacedDigit()
                if size == .systemMedium {
                    BarChart(values: byDay)
                        .frame(height: 40)
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

struct BarChart: View {
    let values: [TimeInterval]
    var body: some View {
        GeometryReader { geo in
            let maxVal = values.max() ?? 1
            HStack(alignment: .bottom, spacing: 2) {
                ForEach(values.indices, id: \.self) { i in
                    Rectangle()
                        .fill(Color.accentColor)
                        .frame(width: (geo.size.width - CGFloat(values.count - 1) * 2) / CGFloat(values.count),
                               height: max(2, geo.size.height * CGFloat(values[i] / maxVal)))
                }
            }
        }
    }
}
