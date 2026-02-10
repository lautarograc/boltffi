#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../.."

usage() {
    echo "Usage: $0 [--platform <apple|android>] [--skip-bench] [--release|--debug]"
    echo ""
    echo "Options:"
    echo "  --platform <apple|android>  Target platform (default: apple)"
    echo "  --skip-bench                Skip running benchmarks"
    echo "  --release                   Build in release mode"
    echo "  --debug                     Build in debug mode (default)"
    echo "  -h, --help                  Show this help message"
    exit 0
}

PLATFORM="apple"
SKIP_BENCH=false
BUILD_MODE="debug"

while [[ $# -gt 0 ]]; do
    case $1 in
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        --platform=*)
            PLATFORM="${1#*=}"
            shift
            ;;
        --skip-bench)
            SKIP_BENCH=true
            shift
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

if [[ "$PLATFORM" == "ios" ]]; then
    PLATFORM="apple"
fi

if [[ "$PLATFORM" != "apple" && "$PLATFORM" != "android" ]]; then
    echo "Error: Invalid platform '$PLATFORM'. Must be 'apple' or 'android'."
    exit 1
fi

BOLTFFI_CLI="$ROOT_DIR/target/$BUILD_MODE/boltffi"

cd "$SCRIPT_DIR"

echo "=== Building riff CLI ($BUILD_MODE) ==="
if [[ "$BUILD_MODE" == "release" ]]; then
    cargo build --release -p boltffi_cli --manifest-path "$ROOT_DIR/Cargo.toml"
else
    cargo build -p boltffi_cli --manifest-path "$ROOT_DIR/Cargo.toml"
fi

if [[ "$PLATFORM" == "apple" ]]; then
    echo "=== Building for Apple ==="
    if [[ "$BUILD_MODE" == "release" ]]; then
        "$BOLTFFI_CLI" build apple --release
    else
        "$BOLTFFI_CLI" build apple
    fi

    echo "=== Generating Swift bindings ==="
    "$BOLTFFI_CLI" generate swift

    echo "=== Packaging Apple artifacts ==="
    if [[ "$BUILD_MODE" == "release" ]]; then
        "$BOLTFFI_CLI" pack apple --release
    else
        "$BOLTFFI_CLI" pack apple
    fi

    echo "=== Updating BoltFFIPackage ==="
    rm -rf ./BoltFFIPackage/BenchBoltffi.xcframework
    rm -rf ./BoltFFIPackage/.build
    cp -r ./dist/apple/BenchBoltffi.xcframework ./BoltFFIPackage/
    cp ./dist/apple/Sources/BoltFFI/Bench_boltffiBoltFFI.swift ./BoltFFIPackage/Sources/BenchBoltFFI.swift

    if [[ "$SKIP_BENCH" == false ]]; then
        echo "=== Building & Running Swift Bench ==="
        cd ../swift-macos-bench
        rm -rf .build
        if [[ "$BUILD_MODE" == "release" ]]; then
            swift build -c release
            .build/release/SwiftBench
        else
            swift build
            .build/debug/SwiftBench --allow-debug-build
        fi
    fi

elif [[ "$PLATFORM" == "android" ]]; then
    echo "=== Building Rust for host ==="
    cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

    echo "=== Generating Kotlin + JNI bindings ==="
    "$BOLTFFI_CLI" generate kotlin
    "$BOLTFFI_CLI" generate header

    echo "=== Building for Android targets ==="
    if [[ "$BUILD_MODE" == "release" ]]; then
        "$BOLTFFI_CLI" build android --release
    else
        "$BOLTFFI_CLI" build android
    fi

    echo "=== Packaging Android jniLibs ==="
    if [[ "$BUILD_MODE" == "release" ]]; then
        "$BOLTFFI_CLI" pack android --release
    else
        "$BOLTFFI_CLI" pack android
    fi

    if [[ "$SKIP_BENCH" == false ]]; then
        echo "=== Running Kotlin bench ==="
        cd ../kotlin-jvm-bench
        ./gradlew test
    fi
fi

echo "=== Done ($PLATFORM) ==="
