//! Deterministic session construction for fuzz targets. **Not a feature.**
//!
//! A fuzz target for [`crate::aead::FrameOpener`] is useless without a session
//! whose keys match the frames it is fed. With a random session, essentially
//! every input dies at `WrongSession` before the AEAD runs, and the target
//! spends its whole budget exercising an eight-byte comparison.
//!
//! # Why `cfg(fuzzing)` and not a feature
//!
//! Cargo features are **additive**: any crate anywhere in a dependency graph
//! can switch one on for everybody, and nothing warns the crate that holds the
//! keys. A public `test-vectors` feature would be one `Cargo.toml` line away
//! from putting a deterministic session constructor into a release build.
//!
//! `--cfg fuzzing` is set by `cargo-fuzz` on the command line for one build. It
//! cannot be requested by a dependency, it does not appear in `Cargo.toml`, and
//! it is absent from every ordinary `cargo build`, `cargo test` and `cargo
//! install`. This module does not exist in any of them.
//!
//! # It is still a command-line flag, not a lock
//!
//! Worth saying plainly, because "cannot be requested by a dependency" is not
//! the same as "cannot be turned on": anyone building this workspace can set
//! `RUSTFLAGS='--cfg fuzzing'` and compile the whole of it with this module
//! present and the deterministic constructors reachable. That is deliberate —
//! it is how `cargo-fuzz` works — and it is the reason the flag is the right
//! mechanism and a Cargo feature is the wrong one. A feature can be switched on
//! *for you*, by a crate you have never read, and nothing tells you. A
//! `RUSTFLAGS` entry is a decision the person running the build makes, in their
//! own shell, for that build.
//!
//! No release process here sets it. If one ever does, the deterministic seeds
//! below are in the binary.
//!
//! # What it still refuses to do
//!
//! It exposes a session, not key material. There is no accessor here for a
//! traffic secret, an AEAD key or a nonce prefix, because a fuzz target has no
//! use for one and every additional exit is another thing to get wrong.

use qyro_protocol::{Frame, MessageType, SessionId};

use crate::aead::{FrameOpener, FrameSealer};
use crate::handshake::{InitiatorStart, ResponderStart};
use crate::identity::DeviceIdentity;

// TEST ONLY — NEVER PRODUCTION. The same seeds the committed vectors use, so a
// crash found by fuzzing can be reproduced against `aead-v1.json` by hand.
const INITIATOR_SEED: [u8; 32] = [0x11; 32];
const RESPONDER_SEED: [u8; 32] = [0x22; 32];

/// Deterministic handshake entropy. TEST ONLY — NEVER PRODUCTION.
fn entropy(tag: u8) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = tag ^ (index as u8).wrapping_mul(31).wrapping_add(7);
    }
    out
}

/// One side's frame crypto, plus a genuine sealed frame to mutate.
pub struct FuzzSession {
    /// Seals frames the opener below will accept.
    pub sealer: FrameSealer,
    /// Opens what that sealer produced.
    pub opener: FrameOpener,
    /// The session identifier both sides derived.
    pub session_id: SessionId,
}

/// Builds a fixed session by running the real handshake.
///
/// Runs the state machine rather than assembling keys by hand, so a fuzz target
/// exercises the same objects production builds. Returns `None` if any step
/// fails, which cannot happen with fixed entropy but is not worth a panic in a
/// module whose whole job is to feed a panic-hunting tool.
#[must_use]
pub fn deterministic_session() -> Option<FuzzSession> {
    let initiator_identity = DeviceIdentity::from_test_seed(&INITIATOR_SEED);
    let responder_identity = DeviceIdentity::from_test_seed(&RESPONDER_SEED);

    let (initiator_hello, awaiting_responder) = InitiatorStart::new(&initiator_identity)
        .send_hello_with_entropy(entropy(0xA0))
        .ok()?;
    let (responder_hello, awaiting_initiator_finish) = ResponderStart::new(&responder_identity)
        .receive_initiator_hello(&initiator_hello, entropy(0xB0))
        .ok()?;
    let (initiator_finish, awaiting_responder_finish) = awaiting_responder
        .receive_responder_hello(&responder_hello)
        .ok()?;
    let pending = awaiting_initiator_finish
        .receive_initiator_finish(&initiator_finish)
        .ok()?;
    let responder_finish = *pending.encoded_finish();
    let established_responder = pending.confirm_sent();
    let established_initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .ok()?;

    let session_id = established_initiator.session_id();
    let (sealer, _) = established_initiator.into_frame_crypto().ok()?;
    let (_, opener) = established_responder.into_frame_crypto().ok()?;

    Some(FuzzSession {
        sealer,
        opener,
        session_id,
    })
}

/// A plain frame carrying `payload`, for a target to seal.
#[must_use]
pub fn plain_frame(payload: Vec<u8>) -> Option<Frame> {
    Frame::new(MessageType::DataChunk, payload).ok()
}

/// A fresh replay window, so a target can drive its transitions directly.
///
/// The window is `pub(crate)`, and this is the one door out of that — open only
/// under `--cfg fuzzing`, and only wide enough for `check` and `record`.
#[must_use]
pub fn replay_window() -> crate::aead::FuzzReplayWindow {
    crate::aead::FuzzReplayWindow::new()
}
