//! Qyro device identity.
//!
//! A device identity is an Ed25519 keypair that identifies **a device**. It is
//! not a person, not an account, and not a statement of trust: pairing and
//! deciding to trust a peer are separate, later, explicit steps. It carries no
//! name, email or phone number, and nothing here will ever add one.
//!
//! Scope today is identity, the authenticated handshake and the frame AEAD:
//! Ed25519 keys, domain-separated signatures, fingerprints, the four-message
//! handshake in [`handshake`] built on X25519, HKDF-SHA256 and HMAC-SHA256, and
//! the ChaCha20-Poly1305 sealing of QYRO/1 frames in [`aead`].
//!
//! There is still **no transport**. Nothing here opens a socket, discovers a
//! peer or moves a file: the handshake runs between two values in one process,
//! and a sealed frame is a `Vec<u8>` nothing carries anywhere. Qyro does not
//! transfer files.
//!
//! # Secret handling
//!
//! [`DeviceIdentity`] owns the signing key and deliberately does not implement
//! `Clone`, `Copy` or any serialization. Its `Debug` prints a fixed marker, and
//! the secret is zeroized on drop. There is no accessor that returns the seed or
//! the private key: if you need to persist an identity, that belongs in a secure
//! store, which does not exist yet.
//!
//! # Domain separation
//!
//! Nothing signs a bare message. See [`SignatureDomain`] and ADR-0020.
//!
//! ```
//! use qyro_crypto::{DeviceIdentity, SignatureDomain};
//!
//! let identity = DeviceIdentity::generate()?;
//! let public = identity.public_identity();
//!
//! let signature = identity.try_sign(SignatureDomain::DeviceClaim, b"this device")?;
//! assert!(public.verify(SignatureDomain::DeviceClaim, b"this device", &signature).is_ok());
//!
//! // A signature never verifies under a different domain.
//! assert!(public.verify(SignatureDomain::TestVector, b"this device", &signature).is_err());
//! # Ok::<(), qyro_crypto::IdentityError>(())
//! ```
//!
//! # What this crate will not hand you
//!
//! There is no deterministic constructor in the public API, under any feature
//! flag. Signing is fallible only. A public key that is a low-order point is
//! refused rather than wrapped. Each of those was true in a weaker form before
//! and is now enforced by the type system or by a check, not by a convention.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// `fuzzing` is set by cargo-fuzz on the command line for one build, never in
// Cargo.toml. Declaring it here stops the unexpected-cfg lint from flagging
// every use of it. See the `fuzzing` module for why it is not a feature.
#![allow(unexpected_cfgs, reason = "cargo-fuzz sets --cfg fuzzing")]

pub mod aead;
mod error;
mod fingerprint;
/// Deterministic session construction for fuzz targets. Not a feature; see the
/// module docs for why `--cfg fuzzing` and not `[features]`.
#[cfg(fuzzing)]
pub mod fuzzing;
pub mod handshake;
mod identity;
mod signature;

#[cfg(test)]
mod guards;
#[cfg(test)]
mod schema;
#[cfg(test)]
mod vectors;

pub use error::IdentityError;
pub use fingerprint::{FINGERPRINT_LEN, IdentityFingerprint};
pub use handshake::HandshakeError;
pub use identity::{
    DeviceIdentity, IDENTITY_VERSION, PUBLIC_IDENTITY_WIRE_LEN, PUBLIC_KEY_LEN, PublicIdentity,
};
pub use signature::{IdentitySignature, SIGNATURE_LEN, SignatureDomain};
