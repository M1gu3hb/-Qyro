# Qyro native libraries for iOS

The Xcode project links `libqyro_ffi.a` from a platform-specific generated directory:

- `iphoneos/libqyro_ffi.a` for unsigned device builds.
- `iphonesimulator/libqyro_ffi.a` for simulator builds and XCTest.

The archives are generated from `rust/crates/qyro_ffi` and are intentionally not committed. The iOS runtime workflow builds both ARM64 targets, verifies the two Qyro ABI symbols, links the device application without code signing, and executes the native protocol test in a simulator.
