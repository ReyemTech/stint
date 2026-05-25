// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintIntents",
    platforms: [.macOS(.v13)],
    products: [
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
            publicHeadersPath: "include",
            linkerSettings: [
                // The C symbols (stint_verb_*, stint_settings_*, ...) are
                // provided by libstint_core which is statically linked into
                // the Tauri-built Stint binary, not into this framework.
                // Defer symbol resolution until load time.
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
