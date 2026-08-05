//! Bounds that cap what a peer can make this process allocate or wait for.
//!
//! Every limit here is a compile-time constant. A remote peer declares lengths
//! in the frame header, but the values below decide how much memory those
//! declarations can ever reserve. See `docs/adr/ADR-0016-qyro1-wire-framing.md`.

/// Bytes in a QYRO/1.0 header. Fixed layout, see [`crate::FrameHeader`].
pub const HEADER_LEN: usize = 48;

/// Largest header a future minor version may declare.
///
/// A peer speaking a newer minor version may append fields to the header. The
/// extra bytes are skipped, never interpreted, and never unbounded.
pub const MAX_HEADER_LEN: usize = 1024;

/// Largest payload a single frame may carry.
///
/// This is the ceiling for a data chunk. It bounds the memory one frame can
/// occupy while it is being reassembled.
pub const MAX_PAYLOAD_LEN: usize = 1024 * 1024;

/// Largest authentication trailer, sized for the AEAD tags in `SECURITY.md`.
///
/// QYRO/1.0 requires a trailer length of zero: accepting a trailer that nothing
/// verifies yet would mean accepting unauthenticated bytes.
pub const MAX_TRAILER_LEN: usize = 64;

/// Trailer length accepted by QYRO/1.0.
pub const SUPPORTED_TRAILER_LEN: usize = 0;

/// Largest complete frame: header, payload and trailer.
pub const MAX_FRAME_LEN: usize = MAX_HEADER_LEN + MAX_PAYLOAD_LEN + MAX_TRAILER_LEN;

/// Default ceiling for the incremental decoder's internal buffer.
///
/// The buffer must hold one whole frame plus whatever a read happened to
/// deliver, so it is sized to a frame rather than to a socket read.
pub const MAX_BUFFER_LEN: usize = MAX_FRAME_LEN;
