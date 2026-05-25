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
            path: "Sources/StintIntents"
        ),
    ]
)
