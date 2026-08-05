//! The canonical QYRO/1 session identifier.

use core::fmt;

/// Bytes in a session identifier, fixed by the QYRO/1 header layout.
pub const SESSION_ID_LEN: usize = 8;

/// Identifies one session on the wire.
///
/// Eight bytes, big-endian, and the *only* representation. The header used to
/// store a bare `u64` while `qyro_crypto`'s key schedule derived a 32-byte
/// identifier under its `session-id` label. Nothing converted between them, so
/// the first code to put a handshake's identifier into a frame would have had
/// to pick a truncation — a decision about a frozen wire format, taken at a
/// call site, by whoever happened to be wiring up the transport.
///
/// Endianness is the format's, never the host's. A `to_ne_bytes` on this path
/// would pass every test on x86 and produce a different identifier on a
/// big-endian peer, which is the kind of defect that only appears once two
/// architectures actually talk to each other.
///
/// Not a secret: it correlates frames and may be logged or displayed.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SessionId([u8; SESSION_ID_LEN]);

impl SessionId {
    /// The identifier a header carries before a session exists.
    ///
    /// Distinguished from a derived one only by value; a real session
    /// identifier comes out of the handshake key schedule.
    pub const ZERO: Self = Self([0u8; SESSION_ID_LEN]);

    /// Builds an identifier from its wire bytes.
    #[must_use]
    pub const fn from_be_bytes(bytes: [u8; SESSION_ID_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns the wire bytes.
    #[must_use]
    pub const fn to_be_bytes(self) -> [u8; SESSION_ID_LEN] {
        self.0
    }

    /// Borrows the wire bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; SESSION_ID_LEN] {
        &self.0
    }

    /// Builds an identifier from a `u64`, big-endian.
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value.to_be_bytes())
    }

    /// Returns the big-endian `u64` view.
    ///
    /// A convenience for callers that index or log by number. The bytes are
    /// canonical; this is a view of them, not a second representation.
    #[must_use]
    pub const fn to_u64(self) -> u64 {
        u64::from_be_bytes(self.0)
    }
}

impl fmt::Debug for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SessionId(")?;
        for byte in &self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        formatter.write_str(")")
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl From<u64> for SessionId {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

impl From<[u8; SESSION_ID_LEN]> for SessionId {
    fn from(bytes: [u8; SESSION_ID_LEN]) -> Self {
        Self::from_be_bytes(bytes)
    }
}
