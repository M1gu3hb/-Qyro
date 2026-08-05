// swift-tools-version:5.9
//
// Test harness only. Never shipped, and deliberately not part of the Qyro
// application's Xcode project: keeping it in a separate package is what makes
// "the harness cannot end up in Runner.app" a structural fact rather than a
// promise. See docs/adr/ADR-0023-crypto-platform-test-harness.md.
//
// The XCFramework this references is built in CI from the Rust static library
// and is not committed. `ios-crypto` in .github/workflows/crypto-platform.yml
// creates it immediately before running these tests.

import PackageDescription

let package = Package(
    name: "QyroCryptoSmoke",
    platforms: [.iOS(.v13)],
    targets: [
        .binaryTarget(
            name: "CQyroCryptoSmoke",
            path: "CQyroCryptoSmoke.xcframework"
        ),
        .testTarget(
            name: "QyroCryptoSmokeTests",
            dependencies: ["CQyroCryptoSmoke"]
        ),
    ]
)
