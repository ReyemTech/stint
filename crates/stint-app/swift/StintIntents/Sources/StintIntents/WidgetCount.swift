import Foundation
import WidgetKit

/// Returns the number of configured Stint widgets, or -1 on error.
/// Called from Rust via dlsym at GUI startup to decide whether to
/// auto-enable the loopback HTTP API.
///
/// Lives in the StintIntents framework (loaded by stint-app at launch)
/// rather than the StintWidget.appex (separate process) — only the
/// framework path is dlsym-reachable from the main binary.
@_cdecl("stint_widget_count")
public func stint_widget_count() -> Int32 {
    let kindFilter = "tech.reyem.stint.widget"
    let semaphore = DispatchSemaphore(value: 0)
    var result: Int32 = -1
    WidgetCenter.shared.getCurrentConfigurations { res in
        if case .success(let widgets) = res {
            result = Int32(widgets.filter { $0.kind == kindFilter }.count)
        }
        semaphore.signal()
    }
    _ = semaphore.wait(timeout: .now() + .seconds(2))
    return result
}
