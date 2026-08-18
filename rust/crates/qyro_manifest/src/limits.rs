//! Bounds applied to a manifest supplied by a peer.
//!
//! Counts and lengths are checked against these constants before anything is
//! reserved. See `docs/adr/ADR-0017-manifest-serialization.md`.

/// Largest number of items a manifest may declare.
pub const MAX_ITEMS: usize = 100_000;

/// Largest declared transfer size, in bytes.
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024 * 1024;

/// Largest complete relative path, in bytes.
pub const MAX_PATH_LEN: usize = 1024;

/// Largest single path segment, in bytes.
pub const MAX_SEGMENT_LEN: usize = 255;

/// Largest number of segments in a relative path.
pub const MAX_PATH_SEGMENTS: usize = 64;

/// Largest display name, in bytes.
pub const MAX_NAME_LEN: usize = 255;

/// Largest MIME type string, in bytes.
pub const MAX_MIME_LEN: usize = 128;

/// Largest hash digest, in bytes. Sized for SHA-512.
pub const MAX_HASH_LEN: usize = 64;

/// Largest serialized manifest, in bytes.
pub const MAX_ENCODED_LEN: usize = 8 * 1024 * 1024;

/// Manifest format version written by this crate.
///
/// Version 2 removed `display_name` from the wire; see ADR-0019.
pub const MANIFEST_VERSION: u16 = 2;

/// Magic prefix of a serialized manifest: the ASCII bytes `QYRM`.
pub const MANIFEST_MAGIC: [u8; 4] = *b"QYRM";
