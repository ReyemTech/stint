import AppIntents

/// The 5 curated App Shortcuts. Each phrase MUST contain
/// `\(.applicationName)` — appintentsmetadataprocessor rejects the build
/// otherwise. Phrases are a public contract: renaming them breaks any
/// voice shortcuts users have recorded.
public struct StintAppShortcutsProvider: AppShortcutsProvider {
    public static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: StartTimerIntent(),
            phrases: [
                "Start timer in \(.applicationName)",
                "Start tracking in \(.applicationName)",
                "Start \(\.$project) in \(.applicationName)",
            ],
            shortTitle: "Start Timer",
            systemImageName: "play.circle.fill"
        )
        AppShortcut(
            intent: StopTimerIntent(),
            phrases: [
                "Stop \(.applicationName) timer",
                "Stop tracking in \(.applicationName)",
            ],
            shortTitle: "Stop Timer",
            systemImageName: "stop.circle.fill"
        )
        AppShortcut(
            intent: GetCurrentIntent(),
            phrases: [
                "What am I tracking in \(.applicationName)",
                "Show current \(.applicationName) timer",
            ],
            shortTitle: "Current Timer",
            systemImageName: "clock"
        )
        AppShortcut(
            intent: SwitchProjectIntent(),
            phrases: [
                "Switch to \(\.$project) in \(.applicationName)",
            ],
            shortTitle: "Switch Project",
            systemImageName: "arrow.triangle.swap"
        )
        // LogPastIntent's `duration` parameter is a Measurement<UnitDuration>;
        // App Shortcut phrases only allow AppEntity / AppEnum placeholders,
        // not Measurement. So this App Shortcut just opens the intent's
        // configuration dialog where the user fills in duration manually.
        AppShortcut(
            intent: LogPastIntent(),
            phrases: [
                "Log past work in \(.applicationName)",
                "Log last meeting in \(.applicationName)",
            ],
            shortTitle: "Log Past Work",
            systemImageName: "backward.circle"
        )
    }
}
