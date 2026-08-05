//! The four-message authenticated handshake, in memory.
//!
//! Wire format, transcript and key schedule are frozen by
//! `docs/adr/ADR-0021-authenticated-handshake.md`. That document is the
//! specification; this module is one implementation of it. Where they disagree,
//! the ADR is right.
//!
//! # What this establishes
//!
//! Two devices end up holding the same session identifier, the same pair of
//! directional keys, and each other's [`PublicIdentity`] — bound to the
//! transcript, so it is the identity that actually signed rather than the one
//! either side hoped for. **Holding a peer's identity is not trusting it.**
//! Pairing and trust are separate, later, explicit steps.
//!
//! # What this does not do
//!
//! No AEAD: the derived keys encrypt nothing yet. No transport of any kind —
//! there is no socket, no discovery, no framing integration. No replay window,
//! no rekey, no persistence. The handshake runs entirely between two values in
//! one process.
//!
//! # States are consumed
//!
//! Every transition takes `self` by value, so a state cannot be driven twice
//! and the steps cannot be reordered. A handshake with a mutable `step` field
//! permits exactly the mistakes a reviewer does not see while reading the happy
//! path; here they do not compile.

mod error;
mod schedule;
mod transcript;

#[cfg(test)]
mod tests;

use rand_core::{TryCryptoRng, TryRng};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroizing;

use crate::error::IdentityError;
use crate::identity::{DeviceIdentity, PUBLIC_IDENTITY_WIRE_LEN, PublicIdentity};
use crate::signature::{IdentitySignature, SIGNATURE_LEN, SignatureDomain};

pub use error::HandshakeError;
pub use schedule::{FINISHED_MAC_LEN, SessionKey};

use schedule::{DERIVED_KEY_LEN, Schedule, finished_mac, verify_finished_mac};
use transcript::{
    TRANSCRIPT_LEN, auth_transcript, base_transcript, initiator_signing_message,
    responder_signing_message,
};

/// Handshake format version. A different value is refused, never downgraded.
pub const HANDSHAKE_VERSION: u8 = 1;

/// The one cryptographic suite this build implements: X25519, Ed25519,
/// SHA-256, HKDF-SHA256, HMAC-SHA256.
pub const CRYPTO_SUITE_ID: u8 = 1;

/// Bytes in a handshake nonce.
pub const NONCE_LEN: usize = 32;

/// Bytes in an X25519 public key.
pub const X25519_PUBLIC_LEN: usize = 32;

/// Bytes every message begins with: version, suite, type.
const PREFIX_LEN: usize = 3;

/// Bytes in the unsigned part of either hello.
const HELLO_UNSIGNED_LEN: usize =
    PREFIX_LEN + X25519_PUBLIC_LEN + NONCE_LEN + PUBLIC_IDENTITY_WIRE_LEN;

/// Bytes in an `InitiatorHello`.
pub const INITIATOR_HELLO_LEN: usize = HELLO_UNSIGNED_LEN;

/// Bytes in a `ResponderHello`.
pub const RESPONDER_HELLO_LEN: usize = HELLO_UNSIGNED_LEN + SIGNATURE_LEN;

/// Bytes in an `InitiatorFinish`.
pub const INITIATOR_FINISH_LEN: usize = PREFIX_LEN + SIGNATURE_LEN + FINISHED_MAC_LEN;

/// Bytes in a `ResponderFinish`.
pub const RESPONDER_FINISH_LEN: usize = PREFIX_LEN + FINISHED_MAC_LEN;

/// Bytes of entropy one side needs: an ephemeral secret and a nonce.
const HANDSHAKE_ENTROPY_LEN: usize = 32 + NONCE_LEN;

/// Message type codes. Inside the message, therefore inside the transcript.
const TYPE_INITIATOR_HELLO: u8 = 1;
const TYPE_RESPONDER_HELLO: u8 = 2;
const TYPE_INITIATOR_FINISH: u8 = 3;
const TYPE_RESPONDER_FINISH: u8 = 4;

// Offsets within a hello.
const OFFSET_EPHEMERAL: usize = PREFIX_LEN;
const OFFSET_NONCE: usize = OFFSET_EPHEMERAL + X25519_PUBLIC_LEN;
const OFFSET_IDENTITY: usize = OFFSET_NONCE + NONCE_LEN;

/// Replays a fixed buffer as a CSPRNG.
///
/// `EphemeralSecret::random()` calls `getrandom` and panics if it fails, which
/// is not acceptable on this path: [`HandshakeError::EntropyUnavailable`] exists
/// precisely so entropy failure is a decision the caller makes rather than a
/// crash. Drawing the bytes here first and replaying them keeps the failure
/// fallible, and lets the tests drive a handshake deterministically through the
/// same code path production uses.
struct FixedRng {
    bytes: Zeroizing<[u8; 32]>,
    used: bool,
}

impl TryRng for FixedRng {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        let mut out = [0u8; 4];
        self.try_fill_bytes(&mut out)?;
        Ok(u32::from_le_bytes(out))
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        let mut out = [0u8; 8];
        self.try_fill_bytes(&mut out)?;
        Ok(u64::from_le_bytes(out))
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        // The buffer serves exactly one 32-byte draw, which is what
        // `EphemeralSecret::random_from_rng` performs. Anything beyond that
        // would silently reuse the same bytes, so it is zeroed instead: a
        // handshake that quietly reused entropy would be far worse than one
        // that produces an obviously dead key.
        if self.used || dst.len() > self.bytes.len() {
            dst.fill(0);
            return Ok(());
        }
        dst.copy_from_slice(&self.bytes[..dst.len()]);
        self.used = true;
        Ok(())
    }
}

impl TryCryptoRng for FixedRng {}

/// Draws handshake entropy, or reports that the system could not supply it.
fn system_entropy() -> Result<[u8; HANDSHAKE_ENTROPY_LEN], HandshakeError> {
    let mut out = [0u8; HANDSHAKE_ENTROPY_LEN];
    getrandom::fill(&mut out).map_err(|_| HandshakeError::EntropyUnavailable)?;
    Ok(out)
}

/// Splits entropy into an ephemeral X25519 secret and a nonce.
fn split_entropy(entropy: [u8; HANDSHAKE_ENTROPY_LEN]) -> (EphemeralSecret, [u8; NONCE_LEN]) {
    let mut secret_bytes = Zeroizing::new([0u8; 32]);
    secret_bytes.copy_from_slice(&entropy[..32]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&entropy[32..]);

    let mut rng = FixedRng {
        bytes: secret_bytes,
        used: false,
    };
    (EphemeralSecret::random_from_rng(&mut rng), nonce)
}

/// Checks the fixed prefix of any handshake message.
fn check_prefix(
    message: &[u8],
    expected_type: u8,
    expected_len: usize,
) -> Result<(), HandshakeError> {
    if message.len() != expected_len {
        return Err(HandshakeError::InvalidMessageLength {
            found: message.len(),
            expected: expected_len,
        });
    }
    if message[0] != HANDSHAKE_VERSION {
        return Err(HandshakeError::UnsupportedHandshakeVersion {
            found: message[0],
            supported: HANDSHAKE_VERSION,
        });
    }
    if message[1] != CRYPTO_SUITE_ID {
        return Err(HandshakeError::UnsupportedCryptoSuite {
            found: message[1],
            supported: CRYPTO_SUITE_ID,
        });
    }
    if message[2] != expected_type {
        return Err(HandshakeError::UnexpectedMessage {
            found: message[2],
            expected: expected_type,
        });
    }
    Ok(())
}

/// Reads the ephemeral key and identity out of a hello's unsigned part.
fn parse_hello_body(
    message: &[u8],
) -> Result<(X25519PublicKey, [u8; NONCE_LEN], PublicIdentity), HandshakeError> {
    let mut ephemeral = [0u8; X25519_PUBLIC_LEN];
    ephemeral.copy_from_slice(&message[OFFSET_EPHEMERAL..OFFSET_EPHEMERAL + X25519_PUBLIC_LEN]);

    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&message[OFFSET_NONCE..OFFSET_NONCE + NONCE_LEN]);

    let identity = PublicIdentity::decode(
        &message[OFFSET_IDENTITY..OFFSET_IDENTITY + PUBLIC_IDENTITY_WIRE_LEN],
    )
    .map_err(|error| match error {
        // A low-order identity is its own answer: it is well-formed, and that
        // is exactly the problem.
        IdentityError::WeakPublicKey => HandshakeError::WeakPublicIdentity,
        _ => HandshakeError::InvalidPublicIdentity,
    })?;

    Ok((X25519PublicKey::from(ephemeral), nonce, identity))
}

/// Writes the prefix and body shared by both hellos.
fn write_hello_unsigned(
    message_type: u8,
    ephemeral: &X25519PublicKey,
    nonce: &[u8; NONCE_LEN],
    identity: &PublicIdentity,
) -> [u8; HELLO_UNSIGNED_LEN] {
    let mut out = [0u8; HELLO_UNSIGNED_LEN];
    out[0] = HANDSHAKE_VERSION;
    out[1] = CRYPTO_SUITE_ID;
    out[2] = message_type;
    out[OFFSET_EPHEMERAL..OFFSET_EPHEMERAL + X25519_PUBLIC_LEN]
        .copy_from_slice(ephemeral.as_bytes());
    out[OFFSET_NONCE..OFFSET_NONCE + NONCE_LEN].copy_from_slice(nonce);
    out[OFFSET_IDENTITY..OFFSET_IDENTITY + PUBLIC_IDENTITY_WIRE_LEN]
        .copy_from_slice(&identity.encode());
    out
}

/// Performs the X25519 exchange, refusing a non-contributory result.
fn exchange(
    secret: EphemeralSecret,
    peer: &X25519PublicKey,
) -> Result<Zeroizing<[u8; 32]>, HandshakeError> {
    let shared = secret.diffie_hellman(peer);
    if !shared.was_contributory() {
        // The peer sent a low-order point: the shared secret is all zeros and
        // both sides would "agree" without either having contributed anything.
        return Err(HandshakeError::NonContributorySharedSecret);
    }
    Ok(Zeroizing::new(shared.to_bytes()))
}

fn signature_at(message: &[u8], offset: usize) -> IdentitySignature {
    let mut bytes = [0u8; SIGNATURE_LEN];
    bytes.copy_from_slice(&message[offset..offset + SIGNATURE_LEN]);
    IdentitySignature::from_bytes(bytes)
}

fn mac_at(message: &[u8], offset: usize) -> [u8; FINISHED_MAC_LEN] {
    let mut bytes = [0u8; FINISHED_MAC_LEN];
    bytes.copy_from_slice(&message[offset..offset + FINISHED_MAC_LEN]);
    bytes
}

fn sign_transcript(
    identity: &DeviceIdentity,
    message: &[u8],
) -> Result<IdentitySignature, HandshakeError> {
    // Not mapped to `SignatureVerificationFailed`: this is *our* signature, and
    // a failure here means this build cannot sign in the handshake domain at
    // all. Reporting it as a verification failure would blame the peer for a
    // local defect — which is exactly what it did while the domain was still
    // reserved, turning a one-line fix into a hunt through the transcript.
    identity
        .try_sign(SignatureDomain::HandshakeTranscript, message)
        .map_err(|_| HandshakeError::InvalidState)
}

fn verify_transcript(
    identity: &PublicIdentity,
    message: &[u8],
    signature: &IdentitySignature,
) -> Result<(), HandshakeError> {
    identity
        .verify(SignatureDomain::HandshakeTranscript, message, signature)
        .map_err(|_| HandshakeError::SignatureVerificationFailed)
}

// ----------------------------------------------------------------- initiator

/// The initiator, before anything has been sent.
pub struct InitiatorStart<'identity> {
    identity: &'identity DeviceIdentity,
}

impl<'identity> InitiatorStart<'identity> {
    /// Binds a handshake to one device identity.
    ///
    /// The identity is fixed here rather than passed at each step, so a caller
    /// cannot change which device it claims to be halfway through.
    #[must_use]
    pub const fn new(identity: &'identity DeviceIdentity) -> Self {
        Self { identity }
    }

    /// Produces the `InitiatorHello`.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::EntropyUnavailable`] when the system CSPRNG
    /// cannot supply randomness. There is no weaker fallback.
    pub fn send_hello(
        self,
    ) -> Result<
        (
            [u8; INITIATOR_HELLO_LEN],
            InitiatorAwaitResponder<'identity>,
        ),
        HandshakeError,
    > {
        let entropy = system_entropy()?;
        self.send_hello_with_entropy(entropy)
    }

    /// Deterministic variant. **Tests only.**
    ///
    /// Crate-private and `cfg(test)` for the same reason
    /// `DeviceIdentity::from_test_seed` is: a handshake whose entropy a caller
    /// chooses is a handshake with no forward secrecy, and a feature flag would
    /// not keep it out of a release build.
    #[cfg(test)]
    pub(crate) fn send_hello_with_entropy(
        self,
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<
        (
            [u8; INITIATOR_HELLO_LEN],
            InitiatorAwaitResponder<'identity>,
        ),
        HandshakeError,
    > {
        self.build_hello(entropy)
    }

    #[cfg(not(test))]
    fn send_hello_with_entropy(
        self,
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<
        (
            [u8; INITIATOR_HELLO_LEN],
            InitiatorAwaitResponder<'identity>,
        ),
        HandshakeError,
    > {
        self.build_hello(entropy)
    }

    fn build_hello(
        self,
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<
        (
            [u8; INITIATOR_HELLO_LEN],
            InitiatorAwaitResponder<'identity>,
        ),
        HandshakeError,
    > {
        let (secret, nonce) = split_entropy(entropy);
        let hello = write_hello_unsigned(
            TYPE_INITIATOR_HELLO,
            &X25519PublicKey::from(&secret),
            &nonce,
            self.identity.public_identity(),
        );

        Ok((
            hello,
            InitiatorAwaitResponder {
                identity: self.identity,
                secret,
                hello,
            },
        ))
    }
}

/// The initiator, waiting for the responder's hello.
pub struct InitiatorAwaitResponder<'identity> {
    identity: &'identity DeviceIdentity,
    secret: EphemeralSecret,
    hello: [u8; INITIATOR_HELLO_LEN],
}

impl InitiatorAwaitResponder<'_> {
    /// Verifies the `ResponderHello` and produces the `InitiatorFinish`.
    ///
    /// # Errors
    ///
    /// Returns the [`HandshakeError`] describing the first violated rule: a
    /// wrong length, version, suite or type; an unusable or low-order identity;
    /// a non-contributory exchange; or a signature that does not verify.
    pub fn receive_responder_hello(
        self,
        message: &[u8],
    ) -> Result<([u8; INITIATOR_FINISH_LEN], InitiatorAwaitResponderFinish), HandshakeError> {
        check_prefix(message, TYPE_RESPONDER_HELLO, RESPONDER_HELLO_LEN)?;

        let unsigned = &message[..HELLO_UNSIGNED_LEN];
        let (peer_ephemeral, _peer_nonce, peer_identity) = parse_hello_body(unsigned)?;
        let responder_signature = signature_at(message, HELLO_UNSIGNED_LEN);

        let base = base_transcript(&self.hello, unsigned);
        verify_transcript(
            &peer_identity,
            &responder_signing_message(&base),
            &responder_signature,
        )?;

        // Only after the responder is authenticated. An exchange performed
        // before verification would produce a secret derived from an
        // unauthenticated key, and something would eventually be tempted to use
        // it.
        let shared = exchange(self.secret, &peer_ephemeral)?;

        let initiator_signature = sign_transcript(
            self.identity,
            &initiator_signing_message(&base, responder_signature.as_bytes()),
        )?;
        let auth = auth_transcript(
            &base,
            responder_signature.as_bytes(),
            initiator_signature.as_bytes(),
        );

        let derived = Schedule::derive(&base, &shared, &auth)?;
        let initiator_mac = finished_mac(&derived.initiator_finished, &auth);

        let mut finish = [0u8; INITIATOR_FINISH_LEN];
        finish[0] = HANDSHAKE_VERSION;
        finish[1] = CRYPTO_SUITE_ID;
        finish[2] = TYPE_INITIATOR_FINISH;
        finish[PREFIX_LEN..PREFIX_LEN + SIGNATURE_LEN]
            .copy_from_slice(initiator_signature.as_bytes());
        finish[PREFIX_LEN + SIGNATURE_LEN..].copy_from_slice(&initiator_mac);

        Ok((
            finish,
            InitiatorAwaitResponderFinish {
                auth,
                derived,
                peer_identity,
            },
        ))
    }
}

/// The initiator, waiting for the responder's confirmation.
pub struct InitiatorAwaitResponderFinish {
    auth: [u8; TRANSCRIPT_LEN],
    derived: Schedule,
    peer_identity: PublicIdentity,
}

impl InitiatorAwaitResponderFinish {
    /// The identity that actually signed the transcript.
    ///
    /// Available before the handshake completes because the responder is
    /// already authenticated at this point. Holding it is not trusting it.
    #[must_use]
    pub const fn peer_identity(&self) -> &PublicIdentity {
        &self.peer_identity
    }

    /// Verifies the `ResponderFinish` and completes the handshake.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::FinishedVerificationFailed`] when the MAC does
    /// not match, which means the peer proved its identity but derived
    /// different keys.
    pub fn receive_responder_finish(
        self,
        message: &[u8],
    ) -> Result<EstablishedInitiator, HandshakeError> {
        check_prefix(message, TYPE_RESPONDER_FINISH, RESPONDER_FINISH_LEN)?;

        let received = mac_at(message, PREFIX_LEN);
        let expected = finished_mac(&self.derived.responder_finished, &self.auth);
        verify_finished_mac(&expected, &received)?;

        Ok(EstablishedInitiator {
            session: Session {
                session_id: self.derived.session_id,
                sending: self.derived.initiator_to_responder,
                receiving: self.derived.responder_to_initiator,
                peer_identity: self.peer_identity,
            },
        })
    }
}

// ----------------------------------------------------------------- responder

/// The responder, before anything has been received.
pub struct ResponderStart<'identity> {
    identity: &'identity DeviceIdentity,
}

impl<'identity> ResponderStart<'identity> {
    /// Binds a handshake to one device identity.
    #[must_use]
    pub const fn new(identity: &'identity DeviceIdentity) -> Self {
        Self { identity }
    }

    /// Verifies the `InitiatorHello` and produces the signed `ResponderHello`.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::EntropyUnavailable`] when the system CSPRNG
    /// cannot supply randomness, or the error describing the first rule the
    /// hello violated.
    pub fn receive_initiator_hello_from_system(
        self,
        message: &[u8],
    ) -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> {
        let entropy = system_entropy()?;
        self.receive_initiator_hello(message, entropy)
    }

    /// Verifies the `InitiatorHello` using supplied entropy.
    ///
    /// `pub(crate)` outside tests: production callers use
    /// [`Self::receive_initiator_hello_from_system`], which draws its own.
    #[cfg(test)]
    pub(crate) fn receive_initiator_hello(
        self,
        message: &[u8],
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> {
        self.answer_hello(message, entropy)
    }

    #[cfg(not(test))]
    fn receive_initiator_hello(
        self,
        message: &[u8],
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> {
        self.answer_hello(message, entropy)
    }

    fn answer_hello(
        self,
        message: &[u8],
        entropy: [u8; HANDSHAKE_ENTROPY_LEN],
    ) -> Result<([u8; RESPONDER_HELLO_LEN], ResponderAwaitInitiatorFinish), HandshakeError> {
        check_prefix(message, TYPE_INITIATOR_HELLO, INITIATOR_HELLO_LEN)?;
        let (peer_ephemeral, _peer_nonce, peer_identity) = parse_hello_body(message)?;

        let (secret, nonce) = split_entropy(entropy);
        let unsigned = write_hello_unsigned(
            TYPE_RESPONDER_HELLO,
            &X25519PublicKey::from(&secret),
            &nonce,
            self.identity.public_identity(),
        );

        let shared = exchange(secret, &peer_ephemeral)?;

        let base = base_transcript(message, &unsigned);
        let responder_signature =
            sign_transcript(self.identity, &responder_signing_message(&base))?;

        let mut hello = [0u8; RESPONDER_HELLO_LEN];
        hello[..HELLO_UNSIGNED_LEN].copy_from_slice(&unsigned);
        hello[HELLO_UNSIGNED_LEN..].copy_from_slice(responder_signature.as_bytes());

        Ok((
            hello,
            ResponderAwaitInitiatorFinish {
                base,
                shared,
                responder_signature,
                peer_identity,
            },
        ))
    }
}

/// The responder, waiting for the initiator's signature and confirmation.
pub struct ResponderAwaitInitiatorFinish {
    base: [u8; TRANSCRIPT_LEN],
    shared: Zeroizing<[u8; 32]>,
    responder_signature: IdentitySignature,
    peer_identity: PublicIdentity,
}

impl ResponderAwaitInitiatorFinish {
    /// Verifies the `InitiatorFinish` and produces the `ResponderFinish`.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::SignatureVerificationFailed`] when the
    /// initiator's signature does not verify over this transcript, or
    /// [`HandshakeError::FinishedVerificationFailed`] when its MAC does not
    /// match.
    pub fn receive_initiator_finish(
        self,
        message: &[u8],
    ) -> Result<([u8; RESPONDER_FINISH_LEN], EstablishedResponder), HandshakeError> {
        check_prefix(message, TYPE_INITIATOR_FINISH, INITIATOR_FINISH_LEN)?;

        let initiator_signature = signature_at(message, PREFIX_LEN);
        verify_transcript(
            &self.peer_identity,
            &initiator_signing_message(&self.base, self.responder_signature.as_bytes()),
            &initiator_signature,
        )?;

        let auth = auth_transcript(
            &self.base,
            self.responder_signature.as_bytes(),
            initiator_signature.as_bytes(),
        );
        let derived = Schedule::derive(&self.base, &self.shared, &auth)?;

        let received = mac_at(message, PREFIX_LEN + SIGNATURE_LEN);
        let expected = finished_mac(&derived.initiator_finished, &auth);
        verify_finished_mac(&expected, &received)?;

        let responder_mac = finished_mac(&derived.responder_finished, &auth);
        let mut finish = [0u8; RESPONDER_FINISH_LEN];
        finish[0] = HANDSHAKE_VERSION;
        finish[1] = CRYPTO_SUITE_ID;
        finish[2] = TYPE_RESPONDER_FINISH;
        finish[PREFIX_LEN..].copy_from_slice(&responder_mac);

        Ok((
            finish,
            EstablishedResponder {
                session: Session {
                    session_id: derived.session_id,
                    sending: derived.responder_to_initiator,
                    receiving: derived.initiator_to_responder,
                    peer_identity: self.peer_identity,
                },
            },
        ))
    }
}

// --------------------------------------------------------------- established

/// What both sides end up holding.
struct Session {
    session_id: [u8; DERIVED_KEY_LEN],
    sending: SessionKey,
    receiving: SessionKey,
    peer_identity: PublicIdentity,
}

/// A completed handshake, from the initiator's side.
pub struct EstablishedInitiator {
    session: Session,
}

/// A completed handshake, from the responder's side.
pub struct EstablishedResponder {
    session: Session,
}

macro_rules! established_accessors {
    ($type:ty, $name:literal) => {
        impl $type {
            /// The session identifier both sides derived.
            ///
            /// Derived by the same schedule as the keys but from its own label,
            /// so it reveals nothing about them. Safe to display or log.
            #[must_use]
            pub const fn session_id(&self) -> &[u8; DERIVED_KEY_LEN] {
                &self.session.session_id
            }

            /// The key this side writes with.
            #[must_use]
            pub const fn sending_key(&self) -> &SessionKey {
                &self.session.sending
            }

            /// The key this side reads with.
            #[must_use]
            pub const fn receiving_key(&self) -> &SessionKey {
                &self.session.receiving
            }

            /// The identity that actually signed the transcript.
            ///
            /// Not a statement of trust. Deciding to trust a peer is a separate,
            /// later, explicit step that does not exist yet.
            #[must_use]
            pub const fn peer_identity(&self) -> &PublicIdentity {
                &self.session.peer_identity
            }

            /// Crate-internal raw key access, for the AEAD that will consume it.
            #[cfg(test)]
            pub(crate) const fn sending_key_bytes(&self) -> &[u8; DERIVED_KEY_LEN] {
                self.session.sending.as_bytes()
            }

            /// Crate-internal raw key access, for the AEAD that will consume it.
            #[cfg(test)]
            pub(crate) const fn receiving_key_bytes(&self) -> &[u8; DERIVED_KEY_LEN] {
                self.session.receiving.as_bytes()
            }
        }

        impl core::fmt::Debug for $type {
            /// Prints the peer fingerprint and a fixed marker; never key material.
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter
                    .debug_struct($name)
                    .field("peer", self.session.peer_identity.fingerprint())
                    .field("keys", &"<redacted>")
                    .finish()
            }
        }
    };
}

established_accessors!(EstablishedInitiator, "EstablishedInitiator");
established_accessors!(EstablishedResponder, "EstablishedResponder");
