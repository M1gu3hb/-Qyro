//! The HKDF-SHA256 key schedule and the HMAC-SHA256 confirmation MACs.

use hkdf::Hkdf;
use hmac::{KeyInit, Mac, SimpleHmac};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

use super::error::HandshakeError;
use super::transcript::TRANSCRIPT_LEN;

/// Bytes in a confirmation MAC.
pub const FINISHED_MAC_LEN: usize = 32;

/// Bytes in a derived key.
pub(crate) const DERIVED_KEY_LEN: usize = 32;

/// Prefix on every `info` string.
const LABEL_PREFIX: &[u8] = b"QYRO-HS-V1/";

/// What a derived key is for. One label, one purpose, no reuse.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Label {
    InitiatorFinished,
    ResponderFinished,
    InitiatorToResponder,
    ResponderToInitiator,
    SessionId,
}

impl Label {
    const fn as_bytes(self) -> &'static [u8] {
        match self {
            Self::InitiatorFinished => b"initiator-finished",
            Self::ResponderFinished => b"responder-finished",
            Self::InitiatorToResponder => b"initiator-to-responder",
            Self::ResponderToInitiator => b"responder-to-initiator",
            Self::SessionId => b"session-id",
        }
    }
}

/// A 32-byte secret produced by the key schedule.
///
/// Not `Clone` and not serializable, for the same reason [`crate::DeviceIdentity`]
/// is not: a secret that copies freely ends up somewhere nobody audited. Its
/// `Debug` prints a fixed marker, and it zeroizes on drop.
pub struct SessionKey([u8; DERIVED_KEY_LEN]);

impl SessionKey {
    /// Crate-internal raw access.
    ///
    /// `cfg(test)` because nothing reads these yet: the AEAD that will consume
    /// them does not exist. It stays crate-private when it does — nothing
    /// outside this crate, and in particular nothing across the FFI boundary,
    /// is ever handed raw key bytes.
    #[cfg(test)]
    pub(crate) const fn as_bytes(&self) -> &[u8; DERIVED_KEY_LEN] {
        &self.0
    }
}

impl Drop for SessionKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl core::fmt::Debug for SessionKey {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("SessionKey(<redacted>)")
    }
}

/// The derived material for one completed handshake.
pub(crate) struct Schedule {
    pub(crate) initiator_finished: Zeroizing<[u8; DERIVED_KEY_LEN]>,
    pub(crate) responder_finished: Zeroizing<[u8; DERIVED_KEY_LEN]>,
    pub(crate) initiator_to_responder: SessionKey,
    pub(crate) responder_to_initiator: SessionKey,
    pub(crate) session_id: [u8; DERIVED_KEY_LEN],
}

impl Schedule {
    /// Derives every key for one handshake.
    ///
    /// `salt` is the base transcript and `shared_secret` the X25519 output. The
    /// auth transcript enters each `info`, so no derived key exists without both
    /// signatures having gone into its derivation: there is no
    /// Diffie-Hellman-only secret to fall back on if authentication fails.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::KeyDerivationFailed`] if HKDF refuses the
    /// requested output length, which cannot happen at 32 bytes but is not
    /// worth an `expect` on a security path.
    pub(crate) fn derive(
        salt: &[u8; TRANSCRIPT_LEN],
        shared_secret: &[u8; 32],
        auth_transcript: &[u8; TRANSCRIPT_LEN],
    ) -> Result<Self, HandshakeError> {
        let hkdf = Hkdf::<Sha256>::new(Some(salt), shared_secret);

        let expand = |label: Label| -> Result<[u8; DERIVED_KEY_LEN], HandshakeError> {
            let mut info = Vec::with_capacity(
                LABEL_PREFIX.len() + label.as_bytes().len() + 1 + TRANSCRIPT_LEN,
            );
            info.extend_from_slice(LABEL_PREFIX);
            info.extend_from_slice(label.as_bytes());
            info.push(0x00);
            info.extend_from_slice(auth_transcript);

            let mut out = [0u8; DERIVED_KEY_LEN];
            hkdf.expand(&info, &mut out)
                .map_err(|_| HandshakeError::KeyDerivationFailed)?;
            Ok(out)
        };

        Ok(Self {
            initiator_finished: Zeroizing::new(expand(Label::InitiatorFinished)?),
            responder_finished: Zeroizing::new(expand(Label::ResponderFinished)?),
            initiator_to_responder: SessionKey(expand(Label::InitiatorToResponder)?),
            responder_to_initiator: SessionKey(expand(Label::ResponderToInitiator)?),
            session_id: expand(Label::SessionId)?,
        })
    }
}

/// Computes HMAC-SHA-256 over `message`.
///
/// `SimpleHmac` rather than `Hmac`: it is the variable-key-length form, which
/// is what RFC 2104 specifies and therefore what an implementation in another
/// language will produce from its standard HMAC-SHA-256. Conformance is checked
/// against the RFC 4231 vectors rather than assumed.
pub(crate) fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; FINISHED_MAC_LEN] {
    let mut mac = <SimpleHmac<Sha256> as KeyInit>::new_from_slice(key)
        .unwrap_or_else(|_| unreachable!("RFC 2104 HMAC accepts a key of any length"));
    mac.update(message);
    let tag = mac.finalize().into_bytes();
    let mut out = [0u8; FINISHED_MAC_LEN];
    out.copy_from_slice(&tag);
    out
}

/// Computes a confirmation MAC over the auth transcript.
pub(crate) fn finished_mac(
    key: &[u8; DERIVED_KEY_LEN],
    auth_transcript: &[u8; TRANSCRIPT_LEN],
) -> [u8; FINISHED_MAC_LEN] {
    hmac_sha256(key, auth_transcript)
}

/// Compares a received MAC against the expected one in constant time.
///
/// Never `==`. A byte-by-byte comparison that returns early leaks, through
/// timing, how many leading bytes the sender got right, which is enough to
/// build the rest of the value one byte at a time.
///
/// # Errors
///
/// Returns [`HandshakeError::FinishedVerificationFailed`] when they differ.
pub(crate) fn verify_finished_mac(
    expected: &[u8; FINISHED_MAC_LEN],
    received: &[u8; FINISHED_MAC_LEN],
) -> Result<(), HandshakeError> {
    if expected.ct_eq(received).into() {
        Ok(())
    } else {
        Err(HandshakeError::FinishedVerificationFailed)
    }
}
