// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintIntents",
    platforms: [.macOS(.v13)],
    products: [
        // Dynamic framework — static linking into the Tauri-built stint-app
        // binary clashes with WebKit's Swift runtime expectations (executor
        // lookups SIGSEGV at startup). The framework approach keeps Swift
        // isolated and works for Spotlight indexing. Siri/Shortcuts.app
        // discovery remains a separate undocumented gap; see spec §1.5.
        .library(name: "StintIntents", type: .dynamic, targets: ["StintIntents"]),
    ],
    targets: [
        .target(
            name: "StintIntents",
            path: "Sources/StintIntents",
            exclude: ["Shortcuts/PhraseStrings.xcstrings"],
            resources: [
                .process("Shortcuts/PhraseStrings.xcstrings"),
            ],
            linkerSettings: [
                // The C symbols (stint_verb_*, stint_settings_*, ...) live
                // in stint-core which is statically linked into the Tauri-
                // built Stint binary, not into this framework. Defer symbol
                // resolution until load time when the framework is loaded
                // into the host process and the host's symbols become
                // visible via flat-namespace dlsym.
                .unsafeFlags([
                    "-Xlinker", "-undefined",
                    "-Xlinker", "dynamic_lookup",
                ]),
            ]
        ),
        .testTarget(
            name: "StintIntentsTests",
            dependencies: ["StintIntents"],
            path: "Tests/StintIntentsTests"
        ),
    ]
)
