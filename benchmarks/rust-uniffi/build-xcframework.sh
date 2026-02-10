#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PACKAGE="bench_uniffi"
TARGET_DIR="target"
DIST_DIR="UniffiPackage"
STAGING_DIR="${TARGET_DIR}/uniffi-staging"

SIMULATOR_X86_64="x86_64-apple-ios"
SIMULATOR_AARCH64="aarch64-apple-ios-sim"
DEVICE_AARCH64="aarch64-apple-ios"
MACOS_X86_64="x86_64-apple-darwin"
MACOS_AARCH64="aarch64-apple-darwin"

echo "=== Building Rust targets ==="
for target in "$SIMULATOR_X86_64" "$SIMULATOR_AARCH64" "$DEVICE_AARCH64" "$MACOS_X86_64" "$MACOS_AARCH64"; do
    echo "Building for $target..."
    cargo build --lib --release --target "$target"
done

echo "=== Generating UniFFI bindings ==="
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

cargo run --bin uniffi-bindgen generate \
    --library "${TARGET_DIR}/${DEVICE_AARCH64}/release/lib${PACKAGE}.dylib" \
    --language swift \
    --out-dir "$STAGING_DIR"

echo "=== Creating fat libraries ==="
FAT_SIM_DIR="${TARGET_DIR}/ios-simulator-fat"
FAT_MAC_DIR="${TARGET_DIR}/macos-fat"
mkdir -p "$FAT_SIM_DIR" "$FAT_MAC_DIR"

lipo -create \
    "${TARGET_DIR}/${SIMULATOR_X86_64}/release/lib${PACKAGE}.a" \
    "${TARGET_DIR}/${SIMULATOR_AARCH64}/release/lib${PACKAGE}.a" \
    -output "${FAT_SIM_DIR}/lib${PACKAGE}.a"

lipo -create \
    "${TARGET_DIR}/${MACOS_X86_64}/release/lib${PACKAGE}.a" \
    "${TARGET_DIR}/${MACOS_AARCH64}/release/lib${PACKAGE}.a" \
    -output "${FAT_MAC_DIR}/lib${PACKAGE}.a"

echo "=== Preparing headers ==="
HEADERS_DIR="${STAGING_DIR}/bench_uniffi_headers/bench_uniffi"
mkdir -p "$HEADERS_DIR"
mv "${STAGING_DIR}"/*.h "$HEADERS_DIR/"
mv "${STAGING_DIR}"/*.modulemap "${HEADERS_DIR}/module.modulemap"

echo "=== Building XCFramework ==="
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

XCFRAMEWORK_PATH="${DIST_DIR}/BenchUniffi.xcframework"

HEADERS_PATH="${STAGING_DIR}/bench_uniffi_headers"
xcodebuild -create-xcframework \
    -library "${TARGET_DIR}/${DEVICE_AARCH64}/release/lib${PACKAGE}.a" -headers "$HEADERS_PATH" \
    -library "${FAT_SIM_DIR}/lib${PACKAGE}.a" -headers "$HEADERS_PATH" \
    -library "${FAT_MAC_DIR}/lib${PACKAGE}.a" -headers "$HEADERS_PATH" \
    -output "$XCFRAMEWORK_PATH"

echo "=== Copying Swift sources ==="
mkdir -p "${DIST_DIR}/Sources"
cp "${STAGING_DIR}"/*.swift "${DIST_DIR}/Sources/"

echo "=== Creating Package.swift ==="
cat > "${DIST_DIR}/Package.swift" << 'EOF'
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "BenchUniffi",
    platforms: [
        .iOS(.v16),
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "BenchUniffi",
            targets: ["BenchUniffi"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "BenchUniffiFFI",
            path: "BenchUniffi.xcframework"
        ),
        .target(
            name: "BenchUniffi",
            dependencies: ["BenchUniffiFFI"],
            path: "Sources"
        ),
    ]
)
EOF

echo "=== Done ==="
echo "Output: $DIST_DIR"
ls -la "$DIST_DIR"
