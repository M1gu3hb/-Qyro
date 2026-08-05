//! Qyro device identity.
//!
//! A device identity is an Ed25519 keypair that identifies **a device**. It is
//! not a person, not an account, and not a statement of trust: pairing and
//! deciding to trust a peer are separate, later, explicit steps. It carries no
//! name, email or phone number, and nothing here will ever add one.
//!
//! Scope of this crate today is identity only — keys, domain-separated
//! signatures and fingerprints. There is **no handshake, no X25519, no HKDF and
//! no AEAD**; those arrive in a later sprint. Nothing here can encrypt anything.
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

mod error;
mod fingerprint;
pub mod handshake;
mod identity;
mod signature;

#[cfg(test)]
mod vectors;

pub use error::IdentityError;
pub use fingerprint::{FINGERPRINT_LEN, IdentityFingerprint};
pub use handshake::HandshakeError;
pub use identity::{
    DeviceIdentity, IDENTITY_VERSION, PUBLIC_IDENTITY_WIRE_LEN, PUBLIC_KEY_LEN, PublicIdentity,
};
pub use signature::{IdentitySignature, SIGNATURE_LEN, SignatureDomain};
