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
mod closure_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vectors;

use qyro_protocol::SessionId;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::{Zeroize, Zeroizing};

use crate::error::IdentityError;
use crate::identity::{DeviceIdentity, PUBLIC_IDENTITY_WIRE_LEN, PublicIdentity};
use crate::signature::{IdentitySignature, SIGNATURE_LEN, SignatureDomain};

pub use error::HandshakeError;
pub use schedule::FINISHED_MAC_LEN;

use schedule::{Schedule, SessionKey, finished_mac, verify_finished_mac};
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

/// Bytes of the ephemeral X25519 secret drawn per handshake.
const HANDSHAKE_SECRET_LEN: usize = 32;

/// Bytes of entropy one side needs: an ephemeral secret and a nonce.
const HANDSHAKE_ENTROPY_LEN: usize = HANDSHAKE_SECRET_LEN + NONCE_LEN;

/// Message type codes. Inside the message, therefore inside the transcript.
const TYPE_INITIATOR_HELLO: u8 = 1;
const TYPE_RESPONDER_HELLO: u8 = 2;
const TYPE_INITIATOR_FINISH: u8 = 3;
const TYPE_RESPONDER_FINISH: u8 = 4;

// Offsets within a hello.
const OFFSET_EPHEMERAL: usize = PREFIX_LEN;
const OFFSET_NONCE: usize = OFFSET_EPHEMERAL + X25519_PUBLIC_LEN;
const OFFSET_IDENTITY: usize = OFFSET_NONCE + NONCE_LEN;

/// An X25519 secret that exists for exactly one exchange.
///
/// Wraps `StaticSecret`, which is the only x25519-dalek type constructible
/// directly from bytes, and restores the property its name gives up:
/// [`Self::diffie_hellman`] takes `self`, so the compiler rejects a second use.
///
/// Constructing from bytes rather than through `EphemeralSecret::random_from_rng`
/// is deliberate, and it is what makes this path fail closed. That constructor
/// requires a `CryptoRng`, whose `fill_bytes` is *infallible* — an adapter
/// feeding it drawn bytes has no way to report exhaustion, so the first version
/// of this code answered a read past its buffer by zeroing the destination and
/// returning success. That is not a visibly dead key: an all-zero X25519 secret
/// clamps to a valid scalar and completes a working handshake containing no
/// entropy, which is exactly the outcome [`HandshakeError::EntropyUnavailable`]
/// exists to prevent. Removing the adapter removes the failure mode rather than
/// handling it — there is no longer any code that can substitute bytes.
///
/// Clamping and the scalar arithmetic stay inside the library
/// (`mul_clamped` / `mul_base_clamped`); nothing here reimplements the curve.
/// `StaticSecret` is `ZeroizeOnDrop`, and this wrapper is deliberately not
/// `Clone`, so the secret cannot be duplicated into somewhere unaudited.
pub(crate) struct EphemeralKeyPair {
    secret: StaticSecret,
    public: X25519PublicKey,
}

impl EphemeralKeyPair {
    /// Builds a key pair from freshly drawn entropy.
    ///
    /// The caller draws the bytes fallibly; see [`system_entropy`].
    pub(crate) fn from_secret_bytes(mut bytes: [u8; HANDSHAKE_SECRET_LEN]) -> Self {
        let secret = StaticSecret::from(bytes);
        bytes.zeroize();
        let public = X25519PublicKey::from(&secret);
        Self { secret, public }
    }

    /// The public key to put in a hello.
    pub(crate) const fn public(&self) -> &X25519PublicKey {
        &self.public
    }

    /// Performs the exchange and consumes the secret.
    ///
    /// # Errors
    ///
    /// Returns [`HandshakeError::NonContributorySharedSecret`] when the peer
    /// sent a low-order point.
    pub(crate) fn diffie_hellman(
        self,
        peer: &X25519PublicKey,
    ) -> Result<Zeroizing<[u8; 32]>, HandshakeError> {
        let shared = self.secret.diffie_hellman(peer);
        if !shared.was_contributory() {
            // The peer sent a low-order point: the shared secret is all zeros
            // and both sides would "agree" without either having contributed
            // anything.
            return Err(HandshakeError::NonContributorySharedSecret);
        }
        Ok(Zeroizing::new(shared.to_bytes()))
    }
}

impl core::fmt::Debug for EphemeralKeyPair {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("EphemeralKeyPair")
            .field("secret", &"<redacted>")
            .finish()
    }
}

/// Draws handshake entropy, or reports that the system could not supply it.
fn system_entropy() -> Result<[u8; HANDSHAKE_ENTROPY_LEN], HandshakeError> {
    let mut out = [0u8; HANDSHAKE_ENTROPY_LEN];
    getrandom::fill(&mut out).map_err(|_| HandshakeError::EntropyUnavailable)?;
    Ok(out)
}

/// Splits drawn entropy into an ephemeral key pair and a nonce.
///
/// Total, with no error path, because there is nothing left that can fail: the
/// bytes were drawn fallibly by [`system_entropy`], and everything after that
/// is a copy. The only failure the previous design could have had was one it
/// invented for itself by feeding an infallible RNG trait.
fn split_entropy(mut entropy: [u8; HANDSHAKE_ENTROPY_LEN]) -> (EphemeralKeyPair, [u8; NONCE_LEN]) {
    let mut secret_bytes = [0u8; HANDSHAKE_SECRET_LEN];
    secret_bytes.copy_from_slice(&entropy[..HANDSHAKE_SECRET_LEN]);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&entropy[HANDSHAKE_SECRET_LEN..]);
    entropy.zeroize();

    // Infallible by construction: the bytes are already in hand, and the split
    // is a copy. Every way this could have failed was a way of inventing bytes.
    (EphemeralKeyPair::from_secret_bytes(secret_bytes), nonce)
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
            secret.public(),
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
    secret: EphemeralKeyPair,
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
        let shared = self.secret.diffie_hellman(&peer_ephemeral)?;

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
            secret.public(),
            &nonce,
            self.identity.public_identity(),
        );

        let shared = secret.diffie_hellman(&peer_ephemeral)?;

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
    /// Verifies the `InitiatorFinish` and produces the pending `ResponderFinish`.
    ///
    /// Returns [`ResponderFinishPending`], **not** an established session. It
    /// used to return both the bytes and an `EstablishedResponder`, which said
    /// the session was usable while the message that completes the handshake
    /// was still sitting in the caller's hand — and might never reach the peer.
    /// A responder that starts using session keys at that point is talking to
    /// someone who does not yet believe the handshake finished.
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
    ) -> Result<ResponderFinishPending, HandshakeError> {
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

        Ok(ResponderFinishPending {
            finish,
            session: Session {
                session_id: derived.session_id,
                sending: derived.responder_to_initiator,
                receiving: derived.initiator_to_responder,
                peer_identity: self.peer_identity,
            },
        })
    }
}

/// The responder, holding a verified session it may not use yet.
///
/// It has authenticated the initiator and derived every key, but the peer has
/// not seen the `ResponderFinish`. Until it has, the two sides do not agree
/// that a session exists.
///
/// The only thing this state offers is the bytes to send. Nothing here reaches
/// a key, by design: the point of the state is that the secrets exist and are
/// still out of reach.
///
/// Dropping it without confirming destroys the secrets, which is the correct
/// outcome for a handshake whose last message was never delivered.
pub struct ResponderFinishPending {
    finish: [u8; RESPONDER_FINISH_LEN],
    session: Session,
}

impl ResponderFinishPending {
    /// The `ResponderFinish` bytes to put on the wire.
    #[must_use]
    pub const fn encoded_finish(&self) -> &[u8; RESPONDER_FINISH_LEN] {
        &self.finish
    }

    /// The identity that signed the transcript.
    ///
    /// Available here because the peer is already authenticated. Holding an
    /// identity is not trusting it.
    #[must_use]
    pub const fn peer_identity(&self) -> &PublicIdentity {
        &self.session.peer_identity
    }

    /// Records that the transport delivered [`Self::encoded_finish`], and
    /// establishes the session.
    ///
    /// Call this **only** once the transport reports the bytes were actually
    /// handed over. There is no transport yet, so nothing can call it for real;
    /// the tests confirm delivery by hand, and that is exactly the seam a
    /// transport will occupy.
    ///
    /// Consumes `self`: a session is established once, and a second
    /// confirmation cannot be written.
    #[must_use]
    pub fn confirm_sent(self) -> EstablishedResponder {
        EstablishedResponder {
            session: self.session,
        }
    }
}

impl core::fmt::Debug for ResponderFinishPending {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ResponderFinishPending")
            .field("peer", self.session.peer_identity.fingerprint())
            .field("keys", &"<redacted>")
            .finish()
    }
}

// --------------------------------------------------------------- established

/// Which end of the handshake a state belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// Sent the first hello.
    Initiator,
    /// Answered it.
    Responder,
}

/// What both sides end up holding.
///
/// The traffic secrets stay in this struct and never leave the crate. The AEAD
/// that will consume them lives here too, so they have no reason to.
struct Session {
    session_id: SessionId,
    sending: SessionKey,
    receiving: SessionKey,
    peer_identity: PublicIdentity,
}

/// The derived secrets, handed to the AEAD when one exists.
///
/// Crate-private and deliberately opaque. It is the seam the next milestone
/// consumes to build a frame sealer and opener; declaring it now keeps the
/// established states from growing a public key accessor to serve that purpose
/// later.
pub(crate) struct PendingSessionSecrets {
    #[allow(dead_code, reason = "consumed by the AEAD milestone, not by this one")]
    pub(crate) sending: SessionKey,
    #[allow(dead_code, reason = "consumed by the AEAD milestone, not by this one")]
    pub(crate) receiving: SessionKey,
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
    ($type:ty, $name:literal, $role:expr) => {
        impl $type {
            const ROLE: Role = $role;

            /// The session identifier both sides derived.
            ///
            /// The same eight-byte type the QYRO/1 header carries, so it goes
            /// on the wire with no conversion. Derived by the same schedule as
            /// the keys but under its own label, so it reveals nothing about
            /// them: safe to display or log.
            #[must_use]
            pub const fn session_id(&self) -> SessionId {
                self.session.session_id
            }

            /// Which end of the handshake this is.
            #[must_use]
            pub const fn role(&self) -> Role {
                Self::ROLE
            }

            /// The identity that actually signed the transcript.
            ///
            /// Not a statement of trust. Deciding to trust a peer is a separate,
            /// later, explicit step that does not exist yet.
            #[must_use]
            pub const fn peer_identity(&self) -> &PublicIdentity {
                &self.session.peer_identity
            }

            /// The peer's fingerprint, for display and comparison.
            #[must_use]
            pub const fn peer_fingerprint(&self) -> &crate::IdentityFingerprint {
                self.session.peer_identity.fingerprint()
            }

            /// Hands the traffic secrets to the AEAD. Crate-internal.
            ///
            /// The one way out of this type, and it stays inside the crate.
            /// There is no AEAD yet, so nothing calls it.
            #[allow(dead_code, reason = "the AEAD milestone consumes this")]
            pub(crate) fn into_secrets(self) -> PendingSessionSecrets {
                PendingSessionSecrets {
                    sending: self.session.sending,
                    receiving: self.session.receiving,
                }
            }

            /// Raw key inspection for the crate's own tests.
            ///
            /// `cfg(test)`, never a feature: a feature is additive and any
            /// crate in a dependency graph could switch it on for everybody.
            #[cfg(test)]
            pub(crate) const fn sending_key_bytes(&self) -> &[u8; 32] {
                self.session.sending.as_bytes()
            }

            /// Raw key inspection for the crate's own tests.
            #[cfg(test)]
            pub(crate) const fn receiving_key_bytes(&self) -> &[u8; 32] {
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

established_accessors!(
    EstablishedInitiator,
    "EstablishedInitiator",
    Role::Initiator
);
established_accessors!(
    EstablishedResponder,
    "EstablishedResponder",
    Role::Responder
);
