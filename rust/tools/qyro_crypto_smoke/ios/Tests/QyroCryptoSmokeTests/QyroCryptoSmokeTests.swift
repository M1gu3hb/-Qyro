// Runs the Qyro cryptographic core inside an iOS simulator.
//
// This is the only place iOS gets real evidence about qyro_crypto. The four
// workflows that existed before this one build and run qyro_ffi, which cannot
// reach qyro_crypto at all, so a green iOS job used to say nothing about the
// handshake, the AEAD or the replay window on an Apple toolchain.
//
// Test harness only. Never shipped.

import XCTest
import CQyroCryptoSmoke

final class QyroCryptoSmokeTests: XCTestCase {
    /// Exit codes from `qyro_crypto_smoke_run`, mirrored from the C header so a
    /// failure names the step instead of printing a bare integer.
    private static let stepNames: [Int32: String] = [
        1: "device identity generation failed (system CSPRNG unavailable)",
        2: "the four-message handshake did not complete",
        3: "the two sides disagree on the session identifier",
        4: "frame key derivation failed",
        5: "sealing a frame failed",
        6: "a sealed frame did not survive encode and decode",
        7: "opening a sealed frame failed",
        8: "the plaintext or the authenticated metadata came back wrong",
        9: "a replayed frame was not rejected as a replay",
        10: "a frame with an altered tag authenticated",
        11: "the responder-to-initiator direction failed",
    ]

    func testCryptoSmokeRunsOnThisPlatform() {
        let result = qyro_crypto_smoke_run()
        let reason = Self.stepNames[result] ?? "unknown step"
        XCTAssertEqual(
            result, 0,
            "qyro crypto smoke failed on iOS with code \(result): \(reason)"
        )
    }

    /// Running twice must also pass: each call builds its own identities and its
    /// own session, so a second run sharing state with the first would show up
    /// here as a replay or a nonce failure rather than silently.
    func testCryptoSmokeIsRepeatable() {
        XCTAssertEqual(qyro_crypto_smoke_run(), 0)
        XCTAssertEqual(qyro_crypto_smoke_run(), 0)
    }
}
