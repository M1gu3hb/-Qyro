//! Cross-platform smoke test for the Qyro cryptographic core. **Never shipped.**
//!
//! Four workflows were green before this crate existed and none of them proved
//! anything about `qyro_crypto` away from x86_64 Linux. They all build and run
//! `qyro_ffi`, which deliberately cannot reach `qyro_crypto`, so a green Android
//! job was evidence about a two-symbol ABI and nothing else. See
//! `docs/adr/ADR-0023-crypto-platform-test-harness.md`.
//!
//! # What it runs
//!
//! One complete session, through public API only: two fresh identities, the
//! four-message handshake, the frame AEAD in both directions, a round trip
//! through the ordinary wire decoder, a replay attempt and a tamper attempt.
//!
//! # What it does not do
//!
//! It implements no cryptography. It has no deterministic constructor — the
//! entropy is the system CSPRNG, the same source production uses, because a
//! harness with a fixed key would prove the platform can reproduce a constant
//! rather than that its CSPRNG works.
//!
//! It exposes exactly one C symbol, returning `int32_t`. Nothing here hands out
//! a key, a seed, a nonce, a traffic secret or plaintext: a function that
//! returned bytes would have to document who owns them and who frees them, and
//! for key material the right answer is that they do not cross at all.

#![warn(missing_docs)]
// One deliberate `no_mangle`, so the iOS XCTest target and the Android runner
// can call in by name. `forbid(unsafe_code)` is not used here because
// `#[unsafe(no_mangle)]` is itself an unsafe attribute
// under edition 2024: naming an exported symbol is a promise about the whole
// link, not just about this crate. `deny` instead, so the exception is this one
// attribute rather than the whole file.
#![deny(unsafe_code)]
#![allow(
    unsafe_code,
    reason = "the single C export below is the entire reason this crate exists"
)]

use qyro_crypto::handshake::{InitiatorStart, ResponderStart};
use qyro_crypto::{DeviceIdentity, aead::AeadError};
use qyro_protocol::{DecodedFrame, Flags, Frame, FrameDecoder, MessageType};

#[cfg(test)]
mod guards;
#[cfg(test)]
mod tests;

/// What the smoke run reports.
///
/// Stable numbers: a CI runner reads them as a process exit code, and a code
/// that moved between runs would make an old log unreadable. Each one names the
/// step that failed and nothing about why beyond that.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum SmokeOutcome {
    /// Every step passed.
    Success = 0,
    /// A device identity could not be generated, so the CSPRNG is unavailable.
    IdentityGeneration = 1,
    /// The four-message handshake did not complete.
    Handshake = 2,
    /// The two sides did not agree on a session identifier.
    SessionMismatch = 3,
    /// The traffic secrets could not be expanded into frame keys.
    FrameCryptoDerivation = 4,
    /// A frame could not be sealed.
    Seal = 5,
    /// A sealed frame did not survive encode and decode.
    WireRoundTrip = 6,
    /// The peer could not open a frame this side sealed.
    Open = 7,
    /// The plaintext or the authenticated metadata came back wrong.
    PayloadMismatch = 8,
    /// Replaying a frame was accepted, or rejected for the wrong reason.
    ReplayNotDetected = 9,
    /// A frame with an altered tag authenticated.
    TamperNotDetected = 10,
    /// The responder-to-initiator direction failed where the other succeeded.
    ReverseDirection = 11,
}

impl SmokeOutcome {
    /// The exit code a runner sees.
    #[must_use]
    pub const fn code(self) -> i32 {
        self as i32
    }
}

impl core::fmt::Display for SmokeOutcome {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::Success => "success",
            Self::IdentityGeneration => "device identity generation failed",
            Self::Handshake => "the four-message handshake did not complete",
            Self::SessionMismatch => "the two sides disagree on the session identifier",
            Self::FrameCryptoDerivation => "frame key derivation failed",
            Self::Seal => "sealing a frame failed",
            Self::WireRoundTrip => "a sealed frame did not survive the wire",
            Self::Open => "opening a sealed frame failed",
            Self::PayloadMismatch => "the plaintext or the metadata came back wrong",
            Self::ReplayNotDetected => "a replayed frame was not rejected as a replay",
            Self::TamperNotDetected => "a frame with an altered tag authenticated",
            Self::ReverseDirection => "the responder-to-initiator direction failed",
        };
        formatter.write_str(text)
    }
}

/// The payload the smoke seals. Not a secret; it is in this source file.
const PAYLOAD: &[u8] = b"qyro crypto platform smoke";
/// What the reverse direction sends back.
const REPLY: &[u8] = b"qyro crypto platform smoke reply";

const TRANSFER_ID: u64 = 0x0102_0304_0506_0708;
const STREAM_ID: u32 = 0x090a_0b0c;
const ITEM_ID: u32 = 0x0d0e_0f10;

/// Runs the whole flow and reports the first step that failed.
///
/// # Panics
///
/// It does not. Every step is a `Result` or an `Option`, and this crate has no
/// `unwrap`. It does **not** build with `forbid(unsafe_code)` — see the note at
/// the top of this file — and saying otherwise here contradicted that note
/// sixty lines further down (QYR-0054).
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one linear flow reads better in one place than split across helpers"
)]
pub fn run() -> SmokeOutcome {
    // Real identities from the system CSPRNG. A fixed seed would prove the
    // platform can reproduce a constant, which is not the question.
    let Ok(initiator_identity) = DeviceIdentity::generate() else {
        return SmokeOutcome::IdentityGeneration;
    };
    let Ok(responder_identity) = DeviceIdentity::generate() else {
        return SmokeOutcome::IdentityGeneration;
    };

    // --- handshake ----------------------------------------------------------
    let Ok((initiator_hello, awaiting_responder)) =
        InitiatorStart::new(&initiator_identity).send_hello()
    else {
        return SmokeOutcome::Handshake;
    };
    let Ok((responder_hello, awaiting_initiator_finish)) = ResponderStart::new(&responder_identity)
        .receive_initiator_hello_from_system(&initiator_hello)
    else {
        return SmokeOutcome::Handshake;
    };
    let Ok((initiator_finish, awaiting_responder_finish)) =
        awaiting_responder.receive_responder_hello(&responder_hello)
    else {
        return SmokeOutcome::Handshake;
    };
    let Ok(pending) = awaiting_initiator_finish.receive_initiator_finish(&initiator_finish) else {
        return SmokeOutcome::Handshake;
    };

    // The responder holds a session it may not use until the transport reports
    // the last message was delivered. Here the "transport" is this line.
    let responder_finish = *pending.encoded_finish();
    let established_responder = pending.confirm_sent();

    let Ok(established_initiator) =
        awaiting_responder_finish.receive_responder_finish(&responder_finish)
    else {
        return SmokeOutcome::Handshake;
    };

    if established_initiator.session_id() != established_responder.session_id() {
        return SmokeOutcome::SessionMismatch;
    }

    // --- frame crypto -------------------------------------------------------
    let session_id = established_initiator.session_id();
    let Ok((mut initiator_sealer, mut initiator_opener)) =
        established_initiator.into_frame_crypto()
    else {
        return SmokeOutcome::FrameCryptoDerivation;
    };
    let Ok((mut responder_sealer, mut responder_opener)) =
        established_responder.into_frame_crypto()
    else {
        return SmokeOutcome::FrameCryptoDerivation;
    };

    // --- seal ---------------------------------------------------------------
    let Ok(frame) = Frame::new(MessageType::DataChunk, PAYLOAD.to_vec()) else {
        return SmokeOutcome::Seal;
    };
    let Ok(frame) = frame.with_flags(Flags::END_OF_ITEM) else {
        return SmokeOutcome::Seal;
    };
    let frame = frame.with_identifiers(session_id, TRANSFER_ID, STREAM_ID, ITEM_ID);

    let Ok(sealed) = initiator_sealer.seal(&frame) else {
        return SmokeOutcome::Seal;
    };

    // --- wire ---------------------------------------------------------------
    let bytes = sealed.encode();
    let Some(received) = decode_one(&bytes) else {
        return SmokeOutcome::WireRoundTrip;
    };
    if received.encode() != bytes {
        return SmokeOutcome::WireRoundTrip;
    }

    // --- open ---------------------------------------------------------------
    let Ok(opened) = responder_opener.open(&received) else {
        return SmokeOutcome::Open;
    };
    if opened.payload() != PAYLOAD
        || opened.message_type() != MessageType::DataChunk
        || opened.session_id() != session_id
        || opened.transfer_id() != TRANSFER_ID
        || opened.stream_id() != STREAM_ID
        || opened.item_id() != ITEM_ID
        || opened.sequence() != 0
        || !opened.flags().contains(Flags::END_OF_ITEM)
    {
        return SmokeOutcome::PayloadMismatch;
    }

    // --- replay -------------------------------------------------------------
    if responder_opener.open(&received).err() != Some(AeadError::ReplayDetected { sequence: 0 }) {
        return SmokeOutcome::ReplayNotDetected;
    }

    // --- tamper -------------------------------------------------------------
    // A new sequence, so the replay window cannot be what rejects it: this must
    // fail on the tag and nothing else.
    let Ok(second) = initiator_sealer.seal(&frame) else {
        return SmokeOutcome::Seal;
    };
    let mut altered = second.encode();
    let Some(last) = altered.last_mut() else {
        return SmokeOutcome::TamperNotDetected;
    };
    *last ^= 0xFF;
    let Some(forged) = decode_one(&altered) else {
        return SmokeOutcome::TamperNotDetected;
    };
    if responder_opener.open(&forged).err() != Some(AeadError::AuthenticationFailed) {
        return SmokeOutcome::TamperNotDetected;
    }
    // And the genuine frame with that sequence still opens, which is the whole
    // point of updating the replay window only after the tag verifies.
    if responder_opener.open(second.envelope()).is_err() {
        return SmokeOutcome::TamperNotDetected;
    }

    // --- the other direction ------------------------------------------------
    let Ok(reply) = Frame::new(MessageType::ChunkAck, REPLY.to_vec()) else {
        return SmokeOutcome::ReverseDirection;
    };
    let Ok(sealed_reply) = responder_sealer.seal(&reply) else {
        return SmokeOutcome::ReverseDirection;
    };
    let reply_bytes = sealed_reply.encode();
    let Some(received_reply) = decode_one(&reply_bytes) else {
        return SmokeOutcome::ReverseDirection;
    };
    let Ok(opened_reply) = initiator_opener.open(&received_reply) else {
        return SmokeOutcome::ReverseDirection;
    };
    if opened_reply.payload() != REPLY || opened_reply.sequence() != 0 {
        return SmokeOutcome::ReverseDirection;
    }

    SmokeOutcome::Success
}

/// Decodes exactly one encrypted envelope from `bytes`.
fn decode_one(bytes: &[u8]) -> Option<qyro_protocol::EncryptedEnvelope> {
    let mut decoder = FrameDecoder::new();
    decoder.push(bytes).ok()?;
    match decoder.next_frame() {
        Ok(Some(DecodedFrame::Encrypted(envelope))) => Some(envelope),
        _ => None,
    }
}

/// C entry point for the Android and iOS runners. **Test harness only.**
///
/// Returns 0 on success and a stable non-zero [`SmokeOutcome`] code otherwise.
/// It takes nothing and returns an integer: no pointer crosses this boundary in
/// either direction, so there is nothing to own, free or leak.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_crypto_smoke_run() -> i32 {
    run().code()
}
