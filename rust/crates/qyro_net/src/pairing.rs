//! The pairing string: where a peer is, and which key it had better hold.
//!
//! Specification: `docs/adr/ADR-0035-discovery-and-pairing.md` §2.
//!
//! ```text
//! QYRO1|<socket-addr>|<32 lowercase hex>
//! ```
//!
//! This is the path that works everywhere. Client isolation on the router,
//! networks that filter multicast, a person who denied the permission, an
//! emulator — automatic discovery fails all four and a typed-in string does not.
//! It is also what the QR encodes: there is no second format, scanning is
//! reading this.
//!
//! # The fingerprint here is an expectation, not a credential
//!
//! Scanning a code establishes **no trust by itself**. What it does is fix which
//! fingerprint has to come out of the handshake. The trust decision is still
//! `decide_trust` against the peer store, and the identity it is given is the
//! **authenticated** one — never the one that arrived in this string.
//!
//! What that buys is the rule in ADR-0035 §2.1: if the string carried a
//! fingerprint and it does not match the authenticated one, the session is
//! refused **without asking anybody**. Someone who scanned a code already
//! answered the question, and asking again is how people learn to say yes.

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr as _;

/// The prefix every pairing string starts with.
///
/// Present so a loose string is recognisable as ours, and so a later version can
/// change everything after it without ambiguity.
pub const PAIRING_PREFIX: &str = "QYRO1";

/// The separator. Chosen because it appears in neither half.
///
/// A socket address is digits, dots, colons and brackets; a lowercase hex digest
/// is `0-9a-f`. Neither can contain `|`, so splitting on it is exact and nothing
/// ever needs escaping — which is the whole reason this is not a URL.
pub const PAIRING_SEPARATOR: char = '|';

/// Bytes of fingerprint that travel. Matches `HumanFingerprint`'s 128 bits.
pub const PAIRING_FINGERPRINT_LEN: usize = 16;

/// Why a pairing string was refused.
///
/// One variant per way it can be wrong, because «invalid pairing code» tells a
/// person nothing about which half of the code they mistyped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PairingError {
    /// The string does not begin with [`PAIRING_PREFIX`].
    NotAPairingString,
    /// Not exactly three fields.
    WrongFieldCount { found: usize },
    /// The address half is not a socket address.
    UnreadableAddress,
    /// The address is `0.0.0.0` or `::`, which names no host to dial.
    UnspecifiedAddress,
    /// The port is zero, which is a request for a port and never an answer.
    ZeroPort,
    /// The fingerprint half is not 32 characters.
    FingerprintWrongLength { found: usize },
    /// The fingerprint half has a character that is not lowercase hex.
    FingerprintNotLowercaseHex,
}

impl fmt::Display for PairingError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NotAPairingString => write!(out, "not a Qyro pairing string"),
            Self::WrongFieldCount { found } => {
                write!(out, "a pairing string has three fields, found {found}")
            }
            Self::UnreadableAddress => write!(out, "the address is not readable"),
            Self::UnspecifiedAddress => write!(out, "the address names no host"),
            Self::ZeroPort => write!(out, "port 0 is a request, never an answer"),
            Self::FingerprintWrongLength { found } => {
                write!(out, "a fingerprint is 32 characters, found {found}")
            }
            Self::FingerprintNotLowercaseHex => {
                write!(out, "the fingerprint is not lowercase hexadecimal")
            }
        }
    }
}

impl std::error::Error for PairingError {}

/// Where a peer is, and the fingerprint it is expected to authenticate as.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PairingEndpoint {
    address: SocketAddr,
    fingerprint: [u8; PAIRING_FINGERPRINT_LEN],
}

impl PairingEndpoint {
    /// Builds one, refusing an address nothing could dial.
    ///
    /// # Errors
    ///
    /// [`PairingError::UnspecifiedAddress`] for `0.0.0.0` or `::`, and
    /// [`PairingError::ZeroPort`] for port zero. Both are addresses a listener
    /// legitimately binds and **no** dialler can use, so letting one into a
    /// pairing string would put a failure three layers away from its cause.
    pub const fn new(
        address: SocketAddr,
        fingerprint: [u8; PAIRING_FINGERPRINT_LEN],
    ) -> Result<Self, PairingError> {
        if address.ip().is_unspecified() {
            return Err(PairingError::UnspecifiedAddress);
        }
        if address.port() == 0 {
            return Err(PairingError::ZeroPort);
        }
        Ok(Self {
            address,
            fingerprint,
        })
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &[u8; PAIRING_FINGERPRINT_LEN] {
        &self.fingerprint
    }

    /// Reads one back.
    ///
    /// # Errors
    ///
    /// One [`PairingError`] per way the string can be wrong.
    pub fn parse(text: &str) -> Result<Self, PairingError> {
        let fields: Vec<&str> = text.trim().split(PAIRING_SEPARATOR).collect();
        if fields.len() != 3 {
            return Err(PairingError::WrongFieldCount {
                found: fields.len(),
            });
        }
        // `first`/`get` rather than `fields[0]`: this crate denies indexing, and
        // the length check above would still leave a panic the compiler cannot
        // see.
        let (Some(prefix), Some(address), Some(fingerprint)) =
            (fields.first(), fields.get(1), fields.get(2))
        else {
            return Err(PairingError::WrongFieldCount {
                found: fields.len(),
            });
        };
        if *prefix != PAIRING_PREFIX {
            return Err(PairingError::NotAPairingString);
        }

        let address = SocketAddr::from_str(address).map_err(|_| PairingError::UnreadableAddress)?;
        let fingerprint = fingerprint_from_hex(fingerprint)?;
        Self::new(address, fingerprint)
    }
}

impl fmt::Display for PairingEndpoint {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `SocketAddr`'s own Display, which already brackets IPv6 and is what
        // `FromStr` reads back. Writing the two halves apart here is how a
        // round trip stops being a round trip.
        write!(
            out,
            "{PAIRING_PREFIX}{PAIRING_SEPARATOR}{}{PAIRING_SEPARATOR}",
            self.address
        )?;
        for byte in self.fingerprint {
            write!(out, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// One lowercase hex digit, or nothing.
///
/// Lowercase only, deliberately. Two spellings of the same fingerprint is the
/// ambiguity ADR-0031 removed from the human fingerprint, and accepting both
/// here would put it back for the price of being forgiving about something no
/// human types by hand — a QR scanner does not change case.
const fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn fingerprint_from_hex(text: &str) -> Result<[u8; PAIRING_FINGERPRINT_LEN], PairingError> {
    let bytes = text.as_bytes();
    if bytes.len() != PAIRING_FINGERPRINT_LEN * 2 {
        return Err(PairingError::FingerprintWrongLength {
            found: text.chars().count(),
        });
    }
    let mut out = [0_u8; PAIRING_FINGERPRINT_LEN];
    // `chunks_exact` and `iter_mut` rather than an index: the length is already
    // known to be right, and this way the compiler knows it too.
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let (Some(high), Some(low)) = (pair.first(), pair.get(1)) else {
            return Err(PairingError::FingerprintNotLowercaseHex);
        };
        let (Some(high), Some(low)) = (nibble(*high), nibble(*low)) else {
            return Err(PairingError::FingerprintNotLowercaseHex);
        };
        *slot = (high << 4) | low;
    }
    Ok(out)
}
