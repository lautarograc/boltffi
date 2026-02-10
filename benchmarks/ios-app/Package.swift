// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "IOSBench",
    platforms: [.iOS(.v16), .macOS(.v13)],
    products: [
        .library(name: "IOSBench", targets: ["IOSBench"]),
    ],
    dependencies: [
        .package(name: "BenchBoltFFI", path: "../rust-boltffi/BoltFFIPackage"),
        .package(name: "BenchUniffi", path: "../rust-uniffi/UniffiPackage"),
    ],
    targets: [
        .target(
            name: "IOSBench",
            dependencies: [
                .product(name: "BenchBoltFFI", package: "BenchBoltFFI"),
                .product(name: "BenchUniffi", package: "BenchUniffi"),
            ]
        ),
    ]
)
