// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintIntents",
    platforms: [.macOS(.v13)],
    products: [
        // Static so the Swift types end up in stint-app's main Mach-O,
        // where Apple's App Intents indexer expects to find them. A
        // dynamic framework would leave the types in a sub-binary that
        // the indexer doesn't walk into.
        .library(name: "StintIntents", type: .static, targets: ["StintIntents"]),
    ],
    targets: [
        .target(
            name: "StintIntents",
            path: "Sources/StintIntents",
            exclude: ["Shortcuts/PhraseStrings.xcstrings"],
            resources: [
                .process("Shortcuts/PhraseStrings.xcstrings"),
            ]
            // No -undefined dynamic_lookup here: the C symbols
            // (stint_verb_*, stint_settings_*, ...) live in stint-core
            // which is statically linked into the same final binary, so
            // they're resolvable at link time.
        ),
        .testTarget(
            name: "StintIntentsTests",
            dependencies: ["StintIntents"],
            path: "Tests/StintIntentsTests"
        ),
    ]
)
