// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "StintWidget",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "StintWidget", targets: ["StintWidget"]),
    ],
    targets: [
        .executableTarget(
            name: "StintWidget",
            path: "Sources/StintWidget"
        ),
        .testTarget(
            name: "StintWidgetTests",
            dependencies: ["StintWidget"],
            path: "Tests/StintWidgetTests"
        ),
    ]
)
