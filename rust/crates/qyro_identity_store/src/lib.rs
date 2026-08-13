//! Versioned, platform-backed persistence for a Qyro device identity.
//!
//! Specifications: `docs/adr/ADR-0024-secure-identity-storage.md` and
//! `docs/adr/ADR-0031-trust-and-pairing.md`.
//!
//! # What exists and what does not
//!
//! The identity and known-peer blob formats and their refusals are implemented
//! and tested. The trust decision is pure and does not make a new peer trusted
//! automatically. **One platform backend exists**: `qyro_win_dpapi`, Windows
//! only. There is none for Android and none for iOS, so on those two platforms
//! nothing here persists anything.
//!
//! There is deliberately no in-memory backend outside `cfg(test)` — a working
//! fake in the public API is one import away from becoming the thing a caller
//! ships, which is the same reasoning that keeps `from_test_seed` crate-private
//! in `qyro_crypto`.
//!
//! # What this crate never does
//!
//! It does not invent cryptography. The wrapper underneath already encrypts and
//! authenticates; this crate lays out bytes, refuses the ones that do not fit,
//! and keeps "there is no identity" apart from "there is one and it cannot be
//! read".

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod blob;
mod error;
mod known_peer_types;
mod known_peers;

#[cfg(test)]
mod guards;
#[cfg(test)]
mod tests;

pub use error::StoreError;
pub use known_peer_types::{KnownPeerStoreError, TrustVerdict};
pub use known_peers::{
    HUMAN_FINGERPRINT_LEN, HumanFingerprint, KnownPeer, KnownPeers, MAX_KNOWN_PEERS,
    MAX_PEER_NAME_LEN, MAX_WRAPPED_KNOWN_PEERS_LEN, PeerCandidate, decide_trust, open_known_peers,
    seal_known_peers,
};

use qyro_crypto::{DeviceIdentity, IdentitySecret, SEED_LEN};
use zeroize::Zeroizing;

/// Additional entropy for the platform wrapper, prepended to the header.
///
/// **This is not a secret.** It is compiled into a binary the user holds, and
/// anyone who reads that binary has it. What it buys is domain separation: it
/// stops another application running as the same user from unwrapping this file
/// by calling the platform API with default arguments. Microsoft's own
/// documentation is explicit that secondary entropy "doesn't strengthen the key
/// used to encrypt the data".
///
/// Versioned in the name so that changing it is a visible format change rather
/// than a silent one that makes every existing blob unreadable.
pub const QYRO_IDENTITY_ENTROPY_V1: &[u8] = b"qyro.identity.store.v1";

/// Wraps and unwraps sensitive stored bytes using whatever the platform provides.
///
/// Split out from [`IdentityStore`] so the byte layout can be tested on any
/// platform while the wrapping is only meaningful on one. Implementors do the
/// cryptography; this crate never does.
pub trait SecretWrapper {
    /// Protects `secret` under `entropy`.
    ///
    /// The input is an identity seed or an encoded known-peer body. The wrapper
    /// treats both as opaque bytes and the caller supplies a distinct entropy
    /// domain so one format cannot be opened as the other.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Unwrap`] carrying the platform's own code.
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError>;

    /// Reverses [`Self::wrap`], and fails if `entropy` is not identical.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Unwrap`] when the platform refuses, which includes
    /// every tampering case: the wrapper authenticates its own output.
    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError>;

    /// The `wrap` byte this implementation writes into the header.
    fn wrap_id(&self) -> u8;
}

/// Somewhere a device identity survives the process that made it.
pub trait IdentityStore {
    /// Stores a newly generated identity.
    ///
    /// # Errors
    ///
    /// Implementations decide whether an existing identity is an error or is
    /// replaced, and say which; silently overwriting one is data loss.
    fn create(&self, identity: &DeviceIdentity) -> Result<(), StoreError>;

    /// Loads the stored identity.
    ///
    /// # Errors
    ///
    /// [`StoreError::IdentityAbsent`] when there is none. Every other variant
    /// means there is one and it could not be read, and the two must not be
    /// treated alike.
    fn load(&self) -> Result<DeviceIdentity, StoreError>;

    /// Removes the stored identity.
    ///
    /// # Errors
    ///
    /// Returns an error only when removal itself fails.
    fn delete(&self) -> Result<(), StoreError>;

    /// Replaces the stored identity with a new one.
    ///
    /// # Errors
    ///
    /// Must leave exactly one identity stored: never none, never two.
    fn rotate(&self) -> Result<DeviceIdentity, StoreError>;
}

/// Builds the byte string a wrapper is handed as additional entropy.
///
/// Twelve header bytes, not sixteen. See [`blob::ENTROPY_HEADER_LEN`] and
/// QYR-0048.
#[must_use]
pub fn entropy_for(version: u8, wrap: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(QYRO_IDENTITY_ENTROPY_V1.len() + blob::ENTROPY_HEADER_LEN);
    out.extend_from_slice(QYRO_IDENTITY_ENTROPY_V1);
    out.extend_from_slice(&blob::BlobHeader::entropy_prefix(version, wrap));
    out
}

/// Serialises an identity into the stored form, using `wrapper` to protect it.
///
/// Steps 1 through 6 of the write order.
///
/// # Errors
///
/// Propagates whatever the wrapper reports, and refuses a wrapper output that
/// does not fit a `u32`.
pub fn seal_identity(
    identity: &DeviceIdentity,
    wrapper: &impl SecretWrapper,
) -> Result<Vec<u8>, StoreError> {
    let secret = identity.export_secret();
    let wrap = wrapper.wrap_id();
    let entropy = entropy_for(blob::VERSION, wrap);
    let wrapped = wrapper.wrap(secret.as_bytes(), &entropy)?;
    blob::encode(blob::VERSION, wrap, &wrapped)
}

/// Parses stored bytes back into an identity, using `wrapper` to unprotect.
///
/// Steps 2 through 9 of the read order. Step 1 belongs to the store, which is
/// the only thing that knows whether a blob exists at all.
///
/// # Errors
///
/// One variant per step; see [`StoreError`].
pub fn open_identity(
    bytes: &[u8],
    wrapper: &impl SecretWrapper,
) -> Result<DeviceIdentity, StoreError> {
    let (header, body) = blob::parse(bytes)?;

    // 7b. The blob belongs to this wrapper.
    //
    // Until sprint 4D.2a nothing compared these, because there was one wrapper
    // and the question could not arise. With two, handing a Windows blob to the
    // Android wrapper would have reached `unwrap` and come back as a platform
    // failure — indistinguishable from a corrupt file, which is the one thing
    // the `wrap` byte exists to distinguish (ADR-0025 §5).
    if header.wrap != wrapper.wrap_id() {
        return Err(StoreError::WrapMismatch {
            blob: header.wrap,
            wrapper: wrapper.wrap_id(),
        });
    }

    let entropy = entropy_for(header.version, header.wrap);
    let seed = wrapper.unwrap(body, &entropy)?;
    let bytes: &[u8; SEED_LEN] = seed
        .as_slice()
        .try_into()
        .map_err(|_| StoreError::MalformedSecret { found: seed.len() })?;
    Ok(DeviceIdentity::from_secret(&IdentitySecret::from_bytes(
        bytes,
    )))
}
