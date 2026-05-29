import AppIntents

/// The 5 curated App Shortcuts. Each phrase MUST contain
/// `\(.applicationName)` — appintentsmetadataprocessor rejects the build
/// otherwise. Phrases are a public contract: renaming them breaks any
/// voice shortcuts users have recorded.
///
/// **First-party-app collision avoidance.** Siri's NLU resolves voice
/// phrases against first-party app shortcuts (Clock, Reminders, …) before
/// reaching third-party ones. "Start timer" and "Stop timer" both belong
/// to Clock and will hijack the request even when the user says "in Stint"
/// after. Our phrases deliberately:
///   - Use "tracking" instead of "timer" in the verb position
///   - Lead with the app name when the alternative phrasing isn't unique
///     ("Stint start", "Stint stop") so Siri's first-token match wins
public struct StintAppShortcutsProvider: AppShortcutsProvider {
    public static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: StartTimerIntent(),
            phrases: [
                "Start tracking in \(.applicationName)",
                "Track time in \(.applicationName)",
                "\(.applicationName) start tracking",
                "Track \(\.$project) in \(.applicationName)",
            ],
            shortTitle: "Start Tracking",
            systemImageName: "play.circle.fill"
        )
        AppShortcut(
            intent: StopTimerIntent(),
            phrases: [
                "Stop tracking in \(.applicationName)",
                "\(.applicationName) stop tracking",
                "End \(.applicationName) tracking",
            ],
            shortTitle: "Stop Tracking",
            systemImageName: "stop.circle.fill"
        )
        AppShortcut(
            intent: GetCurrentIntent(),
            phrases: [
                "What am I tracking in \(.applicationName)",
                "\(.applicationName) current tracking",
                "Show \(.applicationName) status",
            ],
            shortTitle: "Current Tracking",
            systemImageName: "clock"
        )
        AppShortcut(
            intent: SwitchProjectIntent(),
            phrases: [
                "Switch to \(\.$project) in \(.applicationName)",
                "\(.applicationName) switch to \(\.$project)",
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
                "\(.applicationName) log past work",
                "Log last meeting in \(.applicationName)",
            ],
            shortTitle: "Log Past Work",
            systemImageName: "backward.circle"
        )
    }
}
