//! The two transcript hashes, exactly as ADR-0021 freezes them.
//!
//! Nothing here is a policy decision; it is a transcription of the ADR. If the
//! two disagree, the ADR is right and this file is a bug.

use sha2::{Digest, Sha256};

use super::HELLO_UNSIGNED_LEN;

/// Domain string for the pre-authentication transcript.
const BASE_PREFIX: &[u8] = b"QYRO-HANDSHAKE-BASE-V1";

/// Domain string for the post-authentication transcript.
const AUTH_PREFIX: &[u8] = b"QYRO-HANDSHAKE-AUTH-V1";

/// Bytes in either transcript hash.
pub(crate) const TRANSCRIPT_LEN: usize = 32;

/// Hashes the two hellos into the pre-authentication transcript.
///
/// ```text
/// SHA-256( "QYRO-HANDSHAKE-BASE-V1" || 0x00
///          || len(initiator_hello) u32 BE          || initiator_hello
///          || len(responder_hello_unsigned) u32 BE || responder_hello_unsigned )
/// ```
///
/// The lengths are written even though both messages are fixed-size. Eight
/// bytes buy the guarantee that no two different pairs of messages can hash the
/// same, which stops being free the moment a later version makes any part
/// variable — and by then the omission would be invisible.
pub(crate) fn base_transcript(
    initiator_hello: &[u8; HELLO_UNSIGNED_LEN],
    responder_hello_unsigned: &[u8; HELLO_UNSIGNED_LEN],
) -> [u8; TRANSCRIPT_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(BASE_PREFIX);
    hasher.update([0x00]);
    update_with_length(&mut hasher, initiator_hello);
    update_with_length(&mut hasher, responder_hello_unsigned);
    finish(hasher)
}

/// Hashes the base transcript together with both signatures.
///
/// ```text
/// SHA-256( "QYRO-HANDSHAKE-AUTH-V1" || 0x00
///          || base_transcript || responder_signature || initiator_signature )
/// ```
///
/// No lengths here: all three inputs are fixed-size by construction, and unlike
/// the hellos they are produced by this code rather than parsed from a peer.
pub(crate) fn auth_transcript(
    base: &[u8; TRANSCRIPT_LEN],
    responder_signature: &[u8; 64],
    initiator_signature: &[u8; 64],
) -> [u8; TRANSCRIPT_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(AUTH_PREFIX);
    hasher.update([0x00]);
    hasher.update(base);
    hasher.update(responder_signature);
    hasher.update(initiator_signature);
    finish(hasher)
}

/// The bytes the responder signs: the base transcript alone.
pub(crate) fn responder_signing_message(base: &[u8; TRANSCRIPT_LEN]) -> [u8; TRANSCRIPT_LEN] {
    *base
}

/// The bytes the initiator signs: the base transcript and the responder's
/// signature.
///
/// Including the responder's signature binds the initiator's to this particular
/// answer, so it cannot be lifted into another session. The two signing
/// messages are 32 and 96 bytes, and the identity signing input carries the
/// message length, so a responder signature can never verify as an initiator
/// one — role separation without a hand-added prefix.
pub(crate) fn initiator_signing_message(
    base: &[u8; TRANSCRIPT_LEN],
    responder_signature: &[u8; 64],
) -> [u8; TRANSCRIPT_LEN + 64] {
    let mut out = [0u8; TRANSCRIPT_LEN + 64];
    out[..TRANSCRIPT_LEN].copy_from_slice(base);
    out[TRANSCRIPT_LEN..].copy_from_slice(responder_signature);
    out
}

/// The `u32` big-endian length prefix a hello carries into the transcript.
///
/// A constant, not a measurement. Both hellos are `HELLO_UNSIGNED_LEN` bytes by
/// type, so there is nothing to convert at runtime and nothing that can fail to
/// fit. This used to be `u32::try_from(bytes.len()).expect(...)` on a slice: an
/// argument made in a comment and enforced by ending the process, on a path
/// driven by bytes a peer chose.
const HELLO_LEN_PREFIX: [u8; 4] = (HELLO_UNSIGNED_LEN as u32).to_be_bytes();

/// The width the cast above relies on, checked during const evaluation. A hello
/// that outgrew a `u32` would stop the build, not the process.
const _: () = assert!(HELLO_UNSIGNED_LEN <= u32::MAX as usize);

fn update_with_length(hasher: &mut Sha256, bytes: &[u8; HELLO_UNSIGNED_LEN]) {
    hasher.update(HELLO_LEN_PREFIX);
    hasher.update(bytes);
}

fn finish(hasher: Sha256) -> [u8; TRANSCRIPT_LEN] {
    let digest = hasher.finalize();
    let mut out = [0u8; TRANSCRIPT_LEN];
    out.copy_from_slice(&digest);
    out
}
