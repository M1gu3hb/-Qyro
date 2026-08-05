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
/// A peer may declare a higher minor: minor versions only append header fields
/// or add message types, so unknown trailing header bytes are skipped.
pub const VERSION_MINOR: u8 = 0;

/// Human-readable protocol identifier shared with the FFI boundary.
pub const PROTOCOL_VERSION: &str = "QYRO/1";
