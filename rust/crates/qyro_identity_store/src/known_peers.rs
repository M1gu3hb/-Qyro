//! Typed trust decisions and the ADR-0031 known-peer store.
//!
//! Network names never reach this module. A [`PeerCandidate`] carries the local
//! name by which the caller selected a record plus the public identity proved by
//! the completed handshake. That distinction is what makes a changed key
//! observable instead of classifying every replacement as a new peer.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::fmt;

use qyro_crypto::{IdentityFingerprint, PublicIdentity};
use zeroize::Zeroizing;

use crate::{KnownPeerStoreError, SecretWrapper, TrustVerdict};

/// Bytes shown to a person during pairing: 128 bits.
pub const HUMAN_FINGERPRINT_LEN: usize = 16;

/// Maximum UTF-8 bytes in a local peer name.
pub const MAX_PEER_NAME_LEN: usize = 255;

/// Maximum records accepted in one store.
pub const MAX_KNOWN_PEERS: usize = 4096;

/// Maximum wrapped bytes accepted before calling a platform wrapper.
pub const MAX_WRAPPED_KNOWN_PEERS_LEN: usize = 2_097_152;

const MAGIC: [u8; 8] = *b"QYRO-KPS";
const VERSION: u8 = 1;
const HEADER_PREFIX_LEN: usize = 12;
const HEADER_LEN: usize = 16;
const PUBLIC_IDENTITY_LEN: usize = 33;
const RECORD_FIXED_LEN: usize = 51;
const MIN_RECORD_LEN: usize = 52;
const MAX_RECORD_LEN: usize = 306;
const MAX_CLEAR_STORE_LEN: usize = 1_269_764;
const ENTROPY_DOMAIN: &[u8] = b"qyro.known-peers.store.v1";

/// A 128-bit display derived from the canonical 256-bit identity fingerprint.
///
/// This type is presentation evidence only. [`decide_trust`] compares the full
/// public identity and never this prefix.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HumanFingerprint([u8; HUMAN_FINGERPRINT_LEN]);

impl HumanFingerprint {
    /// Derives the display from the first 128 bits of a canonical fingerprint.
    #[must_use]
    pub fn from_fingerprint(fingerprint: &IdentityFingerprint) -> Self {
        let mut bytes = [0u8; HUMAN_FINGERPRINT_LEN];
        for (destination, source) in bytes.iter_mut().zip(fingerprint.as_bytes()) {
            *destination = *source;
        }
        Self(bytes)
    }

    /// Returns the exact 16 displayed bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; HUMAN_FINGERPRINT_LEN] {
        &self.0
    }

    /// Returns four groups of eight lowercase hexadecimal characters.
    #[must_use]
    pub fn to_grouped_hex(&self) -> String {
        self.to_string()
    }
}

impl From<&PublicIdentity> for HumanFingerprint {
    fn from(identity: &PublicIdentity) -> Self {
        Self::from_fingerprint(identity.fingerprint())
    }
}

impl fmt::Debug for HumanFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "HumanFingerprint({self})")
    }
}

impl fmt::Display for HumanFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, byte) in self.0.iter().enumerate() {
            if matches!(index, 4 | 8 | 12) {
                formatter.write_str("-")?;
            }
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// One accepted peer, and no connection metadata beyond ADR-0031.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnownPeer {
    name: String,
    identity: PublicIdentity,
    first_seen: i64,
    last_seen: i64,
}

impl KnownPeer {
    /// Validates and constructs one peer record.
    ///
    /// # Errors
    ///
    /// Refuses empty, oversized or control-bearing names and invalid UTC Unix
    /// timestamps.
    pub fn new(
        name: &str,
        identity: PublicIdentity,
        first_seen: i64,
        last_seen: i64,
    ) -> Result<Self, KnownPeerStoreError> {
        validate_name(name)?;
        validate_timestamps(first_seen, last_seen)?;
        Ok(Self {
            name: name.to_owned(),
            identity,
            first_seen,
            last_seen,
        })
    }

    /// Returns the user-chosen local name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the complete public identity accepted for this record.
    #[must_use]
    pub const fn identity(&self) -> &PublicIdentity {
        &self.identity
    }

    /// Returns the first accepted contact as UTC Unix seconds.
    #[must_use]
    pub const fn first_seen(&self) -> i64 {
        self.first_seen
    }

    /// Returns the last matching contact as UTC Unix seconds.
    #[must_use]
    pub const fn last_seen(&self) -> i64 {
        self.last_seen
    }
}

/// A complete, validated known-peer store.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnownPeers {
    records: Vec<KnownPeer>,
}

impl KnownPeers {
    /// Creates an empty store.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Number of records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether there are no known peers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Iterates in stored order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &KnownPeer> {
        self.records.iter()
    }

    fn insert(&mut self, peer: KnownPeer) -> Result<(), KnownPeerStoreError> {
        if self.records.len() >= MAX_KNOWN_PEERS {
            return Err(KnownPeerStoreError::TooManyPeers {
                found: self.records.len().saturating_add(1),
            });
        }
        if self.records.iter().any(|known| known.name == peer.name) {
            return Err(KnownPeerStoreError::DuplicateName);
        }
        if self
            .records
            .iter()
            .any(|known| known.identity == peer.identity)
        {
            return Err(KnownPeerStoreError::DuplicateIdentity);
        }
        self.records.push(peer);
        Ok(())
    }
}

impl TryFrom<Vec<KnownPeer>> for KnownPeers {
    type Error = KnownPeerStoreError;

    fn try_from(records: Vec<KnownPeer>) -> Result<Self, Self::Error> {
        if records.len() > MAX_KNOWN_PEERS {
            return Err(KnownPeerStoreError::TooManyPeers {
                found: records.len(),
            });
        }
        let mut store = Self::new();
        for record in records {
            store.insert(record)?;
        }
        Ok(store)
    }
}

/// A public identity from the completed handshake plus a locally selected name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerCandidate {
    expected_name: String,
    identity: PublicIdentity,
}

impl PeerCandidate {
    /// Constructs a candidate. `expected_name` is local state, never peer input.
    ///
    /// # Errors
    ///
    /// Applies the same name validation as [`KnownPeer::new`].
    pub fn new(expected_name: &str, identity: PublicIdentity) -> Result<Self, KnownPeerStoreError> {
        validate_name(expected_name)?;
        Ok(Self {
            expected_name: expected_name.to_owned(),
            identity,
        })
    }

    /// Returns the local record name the caller selected.
    #[must_use]
    pub fn expected_name(&self) -> &str {
        &self.expected_name
    }

    /// Returns the complete identity proved by the handshake.
    #[must_use]
    pub const fn identity(&self) -> &PublicIdentity {
        &self.identity
    }
}

/// Purely compares a handshake candidate with an immutable store.
#[must_use]
pub fn decide_trust(candidate: &PeerCandidate, store: &KnownPeers) -> TrustVerdict {
    let Some(known) = store
        .records
        .iter()
        .find(|peer| peer.name == candidate.expected_name)
    else {
        return TrustVerdict::New;
    };
    if known.identity == candidate.identity {
        TrustVerdict::KnownAndMatches
    } else {
        TrustVerdict::KnownAndChanged
    }
}

fn validate_name(name: &str) -> Result<(), KnownPeerStoreError> {
    if name.is_empty() {
        return Err(KnownPeerStoreError::EmptyName);
    }
    if name.len() > MAX_PEER_NAME_LEN {
        return Err(KnownPeerStoreError::NameTooLong { found: name.len() });
    }
    if name.chars().any(char::is_control) {
        return Err(KnownPeerStoreError::NameContainsControl);
    }
    Ok(())
}

fn validate_timestamps(first_seen: i64, last_seen: i64) -> Result<(), KnownPeerStoreError> {
    if first_seen < 0 || last_seen < first_seen {
        return Err(KnownPeerStoreError::InvalidTimestamps {
            first_seen,
            last_seen,
        });
    }
    Ok(())
}

fn header_prefix(version: u8, wrap: u8) -> [u8; HEADER_PREFIX_LEN] {
    let mut out = [0u8; HEADER_PREFIX_LEN];
    let (magic, rest) = out.split_at_mut(MAGIC.len());
    magic.copy_from_slice(&MAGIC);
    if let [stored_version, stored_wrap, reserved_0, reserved_1] = rest {
        *stored_version = version;
        *stored_wrap = wrap;
        *reserved_0 = 0;
        *reserved_1 = 0;
    }
    out
}

fn entropy_for_known_peers(version: u8, wrap: u8) -> Vec<u8> {
    let mut entropy = Vec::with_capacity(ENTROPY_DOMAIN.len() + HEADER_PREFIX_LEN);
    entropy.extend_from_slice(ENTROPY_DOMAIN);
    entropy.extend_from_slice(&header_prefix(version, wrap));
    entropy
}

fn encode_clear(store: &KnownPeers) -> Result<Vec<u8>, KnownPeerStoreError> {
    let record_count =
        u32::try_from(store.records.len()).map_err(|_| KnownPeerStoreError::TooManyPeers {
            found: store.records.len(),
        })?;
    let mut clear = Vec::with_capacity(4 + store.records.len() * (4 + MIN_RECORD_LEN));
    clear.extend_from_slice(&record_count.to_be_bytes());
    for peer in &store.records {
        let name_len =
            u16::try_from(peer.name.len()).map_err(|_| KnownPeerStoreError::NameTooLong {
                found: peer.name.len(),
            })?;
        let record_len = RECORD_FIXED_LEN + peer.name.len();
        let encoded_record_len = u32::try_from(record_len)
            .map_err(|_| KnownPeerStoreError::InvalidRecordLength { found: u32::MAX })?;
        clear.extend_from_slice(&encoded_record_len.to_be_bytes());
        clear.extend_from_slice(&peer.identity.encode());
        clear.extend_from_slice(&name_len.to_be_bytes());
        clear.extend_from_slice(&peer.first_seen.to_be_bytes());
        clear.extend_from_slice(&peer.last_seen.to_be_bytes());
        clear.extend_from_slice(peer.name.as_bytes());
    }
    Ok(clear)
}

/// Serializes and protects a complete known-peer store.
///
/// # Errors
///
/// Refuses unknown wrappers and oversized wrapper output, or propagates a
/// typed platform refusal.
pub fn seal_known_peers(
    store: &KnownPeers,
    wrapper: &impl SecretWrapper,
) -> Result<Vec<u8>, KnownPeerStoreError> {
    let wrap = wrapper.wrap_id();
    if !crate::blob::KNOWN_WRAPS.contains(&wrap) {
        return Err(KnownPeerStoreError::UnsupportedKnownPeerWrap { found: wrap });
    }
    let clear = Zeroizing::new(encode_clear(store)?);
    let entropy = entropy_for_known_peers(VERSION, wrap);
    let wrapped = wrapper
        .wrap(&clear, &entropy)
        .map_err(KnownPeerStoreError::Wrapper)?;
    if wrapped.len() > MAX_WRAPPED_KNOWN_PEERS_LEN {
        return Err(KnownPeerStoreError::WrappedTooLarge {
            found: wrapped.len(),
        });
    }
    let wrapped_len =
        u32::try_from(wrapped.len()).map_err(|_| KnownPeerStoreError::WrappedTooLarge {
            found: wrapped.len(),
        })?;

    let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(wrapped.len()));
    out.extend_from_slice(&header_prefix(VERSION, wrap));
    out.extend_from_slice(&wrapped_len.to_be_bytes());
    out.extend_from_slice(&wrapped);
    Ok(out)
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn take(&mut self, needed: usize) -> Result<&'a [u8], KnownPeerStoreError> {
        if self.remaining.len() < needed {
            return Err(KnownPeerStoreError::Truncated {
                found: self.remaining.len(),
                needed,
            });
        }
        let (taken, remaining) = self.remaining.split_at(needed);
        self.remaining = remaining;
        Ok(taken)
    }

    fn u8(&mut self) -> Result<u8, KnownPeerStoreError> {
        let bytes = self.take(1)?;
        bytes
            .first()
            .copied()
            .ok_or(KnownPeerStoreError::Truncated {
                found: 0,
                needed: 1,
            })
    }

    fn u16(&mut self) -> Result<u16, KnownPeerStoreError> {
        let bytes: [u8; 2] =
            self.take(2)?
                .try_into()
                .map_err(|_| KnownPeerStoreError::Truncated {
                    found: 0,
                    needed: 2,
                })?;
        Ok(u16::from_be_bytes(bytes))
    }

    fn u32(&mut self) -> Result<u32, KnownPeerStoreError> {
        let bytes: [u8; 4] =
            self.take(4)?
                .try_into()
                .map_err(|_| KnownPeerStoreError::Truncated {
                    found: 0,
                    needed: 4,
                })?;
        Ok(u32::from_be_bytes(bytes))
    }

    fn i64(&mut self) -> Result<i64, KnownPeerStoreError> {
        let bytes: [u8; 8] =
            self.take(8)?
                .try_into()
                .map_err(|_| KnownPeerStoreError::Truncated {
                    found: 0,
                    needed: 8,
                })?;
        Ok(i64::from_be_bytes(bytes))
    }

    const fn len(&self) -> usize {
        self.remaining.len()
    }
}

struct OuterHeader<'a> {
    version: u8,
    wrap: u8,
    wrapped: &'a [u8],
}

fn parse_outer(bytes: &[u8]) -> Result<OuterHeader<'_>, KnownPeerStoreError> {
    let mut reader = Reader::new(bytes);
    let magic = reader.take(MAGIC.len())?;
    if magic != MAGIC {
        return Err(KnownPeerStoreError::NotAKnownPeerStore);
    }
    let version = reader.u8()?;
    if version != VERSION {
        return Err(KnownPeerStoreError::UnsupportedKnownPeerVersion { found: version });
    }
    let wrap = reader.u8()?;
    if !crate::blob::KNOWN_WRAPS.contains(&wrap) {
        return Err(KnownPeerStoreError::UnsupportedKnownPeerWrap { found: wrap });
    }
    if reader.take(2)? != [0u8; 2] {
        return Err(KnownPeerStoreError::ReservedNotZero);
    }
    let wrapped_len = reader.u32()?;
    if reader.len() > MAX_WRAPPED_KNOWN_PEERS_LEN {
        return Err(KnownPeerStoreError::WrappedTooLarge {
            found: reader.len(),
        });
    }
    if usize::try_from(wrapped_len).is_ok_and(|declared| declared == reader.len()) {
        // exact
    } else {
        return Err(KnownPeerStoreError::LengthMismatch {
            declared: wrapped_len,
            present: reader.len(),
        });
    }
    Ok(OuterHeader {
        version,
        wrap,
        wrapped: reader.remaining,
    })
}

fn decode_record(bytes: &[u8], declared: u32) -> Result<KnownPeer, KnownPeerStoreError> {
    let mut reader = Reader::new(bytes);
    let identity = PublicIdentity::decode(reader.take(PUBLIC_IDENTITY_LEN)?)
        .map_err(KnownPeerStoreError::MalformedPublicIdentity)?;
    let name_len = usize::from(reader.u16()?);
    let expected = RECORD_FIXED_LEN.saturating_add(name_len);
    if usize::try_from(declared).ok() != Some(expected) {
        return Err(KnownPeerStoreError::RecordLengthMismatch { declared, expected });
    }
    let first_seen = reader.i64()?;
    let last_seen = reader.i64()?;
    let name = core::str::from_utf8(reader.take(name_len)?)
        .map_err(|_| KnownPeerStoreError::MalformedName)?;
    if reader.len() != 0 {
        return Err(KnownPeerStoreError::TrailingBytes {
            found: reader.len(),
        });
    }
    KnownPeer::new(name, identity, first_seen, last_seen)
}

fn decode_clear(clear: &[u8]) -> Result<KnownPeers, KnownPeerStoreError> {
    if clear.len() > MAX_CLEAR_STORE_LEN {
        return Err(KnownPeerStoreError::UnwrappedTooLarge { found: clear.len() });
    }
    let mut reader = Reader::new(clear);
    let record_count = usize::try_from(reader.u32()?)
        .map_err(|_| KnownPeerStoreError::TooManyPeers { found: usize::MAX })?;
    if record_count > MAX_KNOWN_PEERS {
        return Err(KnownPeerStoreError::TooManyPeers {
            found: record_count,
        });
    }
    let mut store = KnownPeers {
        records: Vec::with_capacity(record_count),
    };
    for _ in 0..record_count {
        let record_len = reader.u32()?;
        if usize::try_from(record_len)
            .is_ok_and(|length| (MIN_RECORD_LEN..=MAX_RECORD_LEN).contains(&length))
        {
            // bounded before taking or allocating
        } else {
            return Err(KnownPeerStoreError::InvalidRecordLength { found: record_len });
        }
        let record_len_usize = usize::try_from(record_len)
            .map_err(|_| KnownPeerStoreError::InvalidRecordLength { found: record_len })?;
        let record = decode_record(reader.take(record_len_usize)?, record_len)?;
        store.insert(record)?;
    }
    if reader.len() != 0 {
        return Err(KnownPeerStoreError::TrailingBytes {
            found: reader.len(),
        });
    }
    Ok(store)
}

/// Authenticates and parses a complete known-peer store, all or nothing.
///
/// # Errors
///
/// Refuses malformed headers before calling the wrapper, checks the wrapper id,
/// then refuses every malformed record without returning a valid prefix.
pub fn open_known_peers(
    bytes: &[u8],
    wrapper: &impl SecretWrapper,
) -> Result<KnownPeers, KnownPeerStoreError> {
    let header = parse_outer(bytes)?;
    if header.wrap != wrapper.wrap_id() {
        return Err(KnownPeerStoreError::WrapMismatch {
            blob: header.wrap,
            wrapper: wrapper.wrap_id(),
        });
    }
    let entropy = entropy_for_known_peers(header.version, header.wrap);
    let clear = wrapper
        .unwrap(header.wrapped, &entropy)
        .map_err(KnownPeerStoreError::Wrapper)?;
    decode_clear(clear.as_slice())
}
