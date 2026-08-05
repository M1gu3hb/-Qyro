//! Stable, versioned identity fingerprints.

use core::fmt;

use sha2::{Digest, Sha256};

use crate::error::IdentityError;

/// Bytes in a fingerprint.
pub const FINGERPRINT_LEN: usize = 32;

/// Domain string that scopes the fingerprint hash.
const FINGERPRINT_PREFIX: &[u8] = b"QYRO-DEVICE-IDENTITY-V1";

/// A device's canonical fingerprint.
///
/// Derived as:
///
/// ```text
/// SHA-256( "QYRO-DEVICE-IDENTITY-V1" || 0x00 || version (u8) || public_key )
/// ```
///
/// The version is inside the hash, so changing the identity format necessarily
/// changes every fingerprint rather than silently producing the same value for
/// a different meaning.
///
/// The canonical value is all 32 bytes and is **never truncated**. A shorter
/// display form may be added later for the interface, but comparison always uses
/// the full value.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IdentityFingerprint([u8; FINGERPRINT_LEN]);

impl IdentityFingerprint {
    /// Computes the fingerprint of a public key.
    pub(crate) fn compute(version: u8, public_key: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(FINGERPRINT_PREFIX);
        hasher.update([0x00]);
        hasher.update([version]);
        hasher.update(public_key);
        let digest = hasher.finalize();
        let mut out = [0u8; FINGERPRINT_LEN];
        out.copy_from_slice(&digest);
        Self(out)
    }

    /// Returns the full 32 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; FINGERPRINT_LEN] {
        &self.0
    }

    /// Returns the lowercase hex form, without separators.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Returns the grouped form people read aloud when comparing two devices.
    ///
    /// Lowercase hex in eight-character groups separated by `-`. Grouping only
    /// affects presentation; the canonical value is still all 32 bytes.
    #[must_use]
    pub fn to_grouped_hex(&self) -> String {
        let hex = self.to_hex();
        hex.as_bytes()
            .chunks(8)
            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
            .collect::<Vec<String>>()
            .join("-")
    }

    /// Parses either the plain or the grouped hex form.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::MalformedFingerprint`] for a wrong length, a
    /// non-hex character, or uppercase input. Accepting uppercase would give one
    /// fingerprint two spellings, which is exactly the ambiguity the canonical
    /// form exists to avoid.
    pub fn parse(text: &str) -> Result<Self, IdentityError> {
        let compact: String = text.chars().filter(|c| *c != '-').collect();
        if compact.len() != FINGERPRINT_LEN * 2 {
            return Err(IdentityError::MalformedFingerprint);
        }
        if compact
            .chars()
            .any(|c| !c.is_ascii_hexdigit() || c.is_ascii_uppercase())
        {
            return Err(IdentityError::MalformedFingerprint);
        }

        let mut out = [0u8; FINGERPRINT_LEN];
        for (index, slot) in out.iter_mut().enumerate() {
            let start = index * 2;
            *slot = u8::from_str_radix(&compact[start..start + 2], 16)
                .map_err(|_| IdentityError::MalformedFingerprint)?;
        }
        Ok(Self(out))
    }
}

impl fmt::Debug for IdentityFingerprint {
    /// A fingerprint is public by design; printing it leaks nothing.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "IdentityFingerprint({})", self.to_grouped_hex())
    }
}

impl fmt::Display for IdentityFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_grouped_hex())
    }
}
