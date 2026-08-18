//! The on-disk shape of a stored identity.
//!
//! Specification: `docs/adr/ADR-0024-secure-identity-storage.md` and
//! `docs/security/identity-storage.md`. Where this file and the ADR disagree,
//! the ADR is right and this file is a defect.
//!
//! Nothing here does cryptography. It lays out bytes and refuses the ones that
//! do not fit, so that the wrapper underneath is handed a length it agreed to.

// Every byte reaching this module came off a disk somebody else can write to.
// A panic here is a crash on a file an attacker controls, which is why the
// whole family is denied rather than avoided by inspection.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::error::StoreError;

/// Eight bytes that say what this file is before anything else is believed.
pub(crate) const MAGIC: [u8; 8] = *b"QYRO-IDS";

/// The only format version this build writes or accepts.
pub(crate) const VERSION: u8 = 1;

/// DPAPI, user scope. ADR-0024.
pub(crate) const WRAP_DPAPI_USER: u8 = 1;

/// Android Keystore, AES-256-GCM under a non-exportable key. ADR-0025 §5.
pub(crate) const WRAP_ANDROID_KEYSTORE: u8 = 2;

/// A wrapper the host installed across the FFI. ADR-0037 §2, ADR-0040 §5.
///
/// **This byte was missing and the omission was silent.** `BridgedWrapper`
/// has returned 3 from `wrap_id()` since ADR-0037, and this list stopped at 2,
/// so `seal_identity` through the bridge wrote a header that `open_identity`
/// then refused with `UnsupportedWrap { found: 3 }`: **a blob sealed by the
/// bridge could never be reopened**.
///
/// Nothing caught it because the bridge's own contract exercises the wrapper
/// against `entropy_for` and never through `seal_identity`/`open_identity` —
/// two correct pieces and a seam no test crossed, which is the shape of every
/// defect this project has found the hard way.
pub(crate) const WRAP_BRIDGED: u8 = 3;

/// Every wrap byte this build knows how to read.
///
/// A list rather than a range: "less than four" would accept a value this
/// build has no wrapper for, and step 5 exists to refuse exactly that by name.
/// No wrapper at all: the filesystem sandbox is the only protection.
///
/// ADR-0040 enmienda 1. Android's stage A. It is recorded in the blob **because
/// a file must say what protected it**: a stage B build that meets a byte 4 can
/// refuse, migrate or warn, and all three are decisions only a written fact
/// allows. A format that cannot tell protected from unprotected forces a guess,
/// and guessing about key material is the thing this project does not do.
pub(crate) const WRAP_NONE_SANDBOX: u8 = 4;

/// Every wrap byte this build knows how to read.
///
/// A list rather than a range: "less than five" would accept a value this
/// build has no wrapper for, and step 5 exists to refuse exactly that by name.
pub(crate) const KNOWN_WRAPS: [u8; 4] = [
    WRAP_DPAPI_USER,
    WRAP_ANDROID_KEYSTORE,
    WRAP_BRIDGED,
    WRAP_NONE_SANDBOX,
];

/// Bytes before `wrapped` begins.
pub(crate) const HEADER_LEN: usize = 16;

/// Header bytes that go into the wrapper's additional entropy.
///
/// Twelve, not sixteen: `wrapped_len` sits at `12..16` and is not known until
/// the wrapper has already run, so including it made the rule circular and
/// unimplementable. QYR-0048; the amendment in ADR-0024 explains why the defect
/// survived review, which is that only a read order had ever been specified.
///
/// Nothing is lost. What binding the header buys is that a wrapper output
/// cannot be relabelled under a different but valid header — that is about the
/// *interpretation* of the bytes, and `version`, `wrap` and `reserved` carry all
/// of it. A length is not an interpretation.
pub(crate) const ENTROPY_HEADER_LEN: usize = 12;

/// The parsed header of a stored identity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct BlobHeader {
    pub(crate) version: u8,
    pub(crate) wrap: u8,
    pub(crate) wrapped_len: u32,
}

impl BlobHeader {
    /// The twelve bytes that are known before the wrapper runs.
    ///
    /// Step 2 of the write order: a header that exists half-formed while the
    /// procedure is in flight. The previous specification could not express
    /// this, which is exactly how QYR-0048 got frozen.
    pub(crate) fn entropy_prefix(version: u8, wrap: u8) -> [u8; ENTROPY_HEADER_LEN] {
        let mut out = [0u8; ENTROPY_HEADER_LEN];
        let (magic, rest) = out.split_at_mut(MAGIC.len());
        magic.copy_from_slice(&MAGIC);
        // `rest` is exactly four bytes: version, wrap, and two reserved zeroes.
        if let [v, w, r0, r1] = rest {
            *v = version;
            *w = wrap;
            *r0 = 0;
            *r1 = 0;
        }
        out
    }

    /// Serialises the full sixteen-byte header once `wrapped_len` is known.
    pub(crate) fn encode(&self) -> [u8; HEADER_LEN] {
        let mut out = [0u8; HEADER_LEN];
        let prefix = Self::entropy_prefix(self.version, self.wrap);
        let (head, tail) = out.split_at_mut(ENTROPY_HEADER_LEN);
        head.copy_from_slice(&prefix);
        tail.copy_from_slice(&self.wrapped_len.to_be_bytes());
        out
    }
}

/// Splits a stored file into its header and its wrapped body.
///
/// Implements steps 2 through 7 of the read order, in that order. Step 1 —
/// whether a blob exists at all — belongs to the store, not to the format: a
/// missing file is not a malformed one, and conflating them is how a device
/// silently mints a second identity.
pub(crate) fn parse(bytes: &[u8]) -> Result<(BlobHeader, &[u8]), StoreError> {
    // 2. Enough bytes to hold a header.
    if bytes.len() < HEADER_LEN {
        return Err(StoreError::Truncated { found: bytes.len() });
    }
    let (header, body) = bytes.split_at(HEADER_LEN);

    // 3. Magic, before anything in this file is believed.
    let Some(magic) = header.get(..MAGIC.len()) else {
        return Err(StoreError::Truncated { found: bytes.len() });
    };
    if magic != MAGIC {
        return Err(StoreError::NotAnIdentityBlob);
    }

    // 4. Version, refused by name rather than guessed at.
    let Some(&version) = header.get(8) else {
        return Err(StoreError::Truncated { found: bytes.len() });
    };
    if version != VERSION {
        return Err(StoreError::UnsupportedVersion { found: version });
    }

    // 5. Wrap algorithm.
    let Some(&wrap) = header.get(9) else {
        return Err(StoreError::Truncated { found: bytes.len() });
    };
    if !KNOWN_WRAPS.contains(&wrap) {
        return Err(StoreError::UnsupportedWrap { found: wrap });
    }

    // 6. Reserved must be zero. A field that is ignored is a field two versions
    //    read differently, which is the lesson ADR-0018 already paid for.
    let Some(reserved) = header.get(10..12) else {
        return Err(StoreError::Truncated { found: bytes.len() });
    };
    if reserved != [0u8, 0u8] {
        return Err(StoreError::ReservedNotZero);
    }

    // 7. Declared length against the bytes actually present. This is also what
    //    catches a flipped bit inside `wrapped_len` itself, which the entropy
    //    no longer covers (QYR-0048).
    let Some(len_bytes) = header.get(12..16) else {
        return Err(StoreError::Truncated { found: bytes.len() });
    };
    let mut declared = [0u8; 4];
    declared.copy_from_slice(len_bytes);
    let wrapped_len = u32::from_be_bytes(declared);
    if usize::try_from(wrapped_len).is_ok_and(|declared| declared == body.len()) {
        // lengths agree
    } else {
        return Err(StoreError::LengthMismatch {
            declared: wrapped_len,
            present: body.len(),
        });
    }

    Ok((
        BlobHeader {
            version,
            wrap,
            wrapped_len,
        },
        body,
    ))
}

/// Assembles a stored file from a wrapper's output.
///
/// Step 6 of the write order. Step 5 lives here too: a wrapper output that does
/// not fit a `u32` is refused rather than truncated. It is unreachable with any
/// real DPAPI output, which is the reason to check it — a conversion that
/// "cannot fail" and is not checked is the one that eventually does.
pub(crate) fn encode(version: u8, wrap: u8, wrapped: &[u8]) -> Result<Vec<u8>, StoreError> {
    let wrapped_len = u32::try_from(wrapped.len()).map_err(|_| StoreError::WrappedTooLarge {
        found: wrapped.len(),
    })?;
    let header = BlobHeader {
        version,
        wrap,
        wrapped_len,
    };
    let mut out = Vec::with_capacity(HEADER_LEN.saturating_add(wrapped.len()));
    out.extend_from_slice(&header.encode());
    out.extend_from_slice(wrapped);
    Ok(out)
}
