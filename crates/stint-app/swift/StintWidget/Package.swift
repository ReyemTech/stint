// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintWidget",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "StintWidget", type: .dynamic, targets: ["StintWidget"]),
    ],
    targets: [
        .target(
            name: "StintWidget",
            path: "Sources/StintWidget"
        ),
    ]
)
