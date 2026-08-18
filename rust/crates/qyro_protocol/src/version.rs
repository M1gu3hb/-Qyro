//! Wire version constants.

/// Frame magic: the ASCII bytes `QYRO`.
pub const MAGIC: [u8; 4] = *b"QYRO";

/// Major version implemented by this crate.
///
/// A different major may change the header layout, so frames carrying one are
/// rejected without further interpretation.
pub const VERSION_MAJOR: u8 = 1;

/// Minor version implemented by this crate.
///
/// A peer may declare a higher minor, and the frame is still decoded: minor
/// versions only append header fields or add message types.
///
/// **Corrected in sprint 4C.2 (QYR-0031).** This said unknown trailing header
/// bytes "are skipped". They are not. A header longer than [`crate::HEADER_LEN`]
/// is refused with `FrameError::UnsupportedHeaderExtension`, because bytes that
/// are neither stored nor re-serialized cannot be re-encoded byte-exactly and
/// cannot be authenticated. See ADR-0018.
pub const VERSION_MINOR: u8 = 0;

/// Human-readable protocol identifier shared with the FFI boundary.
pub const PROTOCOL_VERSION: &str = "QYRO/1";
