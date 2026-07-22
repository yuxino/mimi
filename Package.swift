// swift-tools-version: 6.1

import PackageDescription

let package = Package(
    name: "mimi",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(name: "MimiCore", targets: ["MimiCore"]),
        .executable(name: "mimi", targets: ["MimiApp"]),
        .executable(name: "mimi-core-tests", targets: ["MimiCoreTests"])
    ],
    targets: [
        .target(name: "MimiCore"),
        .executableTarget(
            name: "MimiApp",
            dependencies: ["MimiCore"],
            linkerSettings: [
                .linkedFramework("AppKit"),
                .linkedFramework("AVFoundation"),
                .linkedFramework("ScreenCaptureKit"),
                .linkedFramework("Security"),
                .linkedFramework("SwiftUI")
            ]
        ),
        .executableTarget(
            name: "MimiCoreTests",
            dependencies: ["MimiCore"],
            path: "Tests/MimiCoreTests"
        )
    ]
)
