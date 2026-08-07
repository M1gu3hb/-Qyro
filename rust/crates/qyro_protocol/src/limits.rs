//! Bounds that cap what a peer can make this process allocate or wait for.
//!
//! Every limit here is a compile-time constant. A remote peer declares lengths
//! in the frame header, but the values below decide how much memory those
//! declarations can ever reserve. See `docs/adr/ADR-0016-qyro1-wire-framing.md`.

/// Bytes in a QYRO/1.0 header. Fixed layout, see [`crate::FrameHeader`].
pub const HEADER_LEN: usize = 48;

/// Largest header a future minor version may declare.
///
/// **Corrected in sprint 4C.2 (QYR-0031).** This said the extra bytes are
/// "skipped, never interpreted". They are not skipped: `FrameHeader::decode`
/// refuses any declared length other than [`HEADER_LEN`] with
/// `FrameError::UnsupportedHeaderExtension`. ADR-0018 gives the reason —
/// skipping bytes that are neither stored nor re-serialized breaks byte-exact
/// re-encoding and leaves the AEAD unable to authenticate them.
///
/// What this constant bounds is the *declaration*, before the refusal: a peer
/// announcing a four-gigabyte header is an error, not an allocation.
pub const MAX_HEADER_LEN: usize = 1024;

/// Largest payload a single frame may carry.
///
/// This is the ceiling for a data chunk. It bounds the memory one frame can
/// occupy while it is being reassembled.
pub const MAX_PAYLOAD_LEN: usize = 1024 * 1024;

/// Largest authentication trailer, sized for the AEAD tags in `SECURITY.md`.
///
/// **Corrected in sprint 4C.2 (QYR-0031).** This said "QYRO/1.0 requires a
/// trailer length of zero". That has not been true since the AEAD landed. The
/// rule depends on the `ENCRYPTED` flag: a frame carrying it must declare a
/// trailer of `1..=MAX_TRAILER_LEN`, because a frame claiming to be sealed
/// without a tag is asserting protection it does not have. A frame without the
/// flag must declare exactly [`SUPPORTED_TRAILER_LEN`].
pub const MAX_TRAILER_LEN: usize = 64;

/// Trailer length a **plain** frame carries: none.
///
/// **Corrected in sprint 4C.2 (QYR-0031).** This was documented as "the trailer
/// length accepted by QYRO/1.0", which now reads as a statement about every
/// frame. It is the plain-frame rule only; see [`MAX_TRAILER_LEN`] for the
/// sealed one. Accepting a trailer on a frame nothing authenticates would mean
/// accepting unauthenticated bytes, which is why this stays zero.
pub const SUPPORTED_TRAILER_LEN: usize = 0;

/// Largest complete frame: header, payload and trailer.
pub const MAX_FRAME_LEN: usize = MAX_HEADER_LEN + MAX_PAYLOAD_LEN + MAX_TRAILER_LEN;

/// Default ceiling for the incremental decoder's internal buffer.
///
/// The buffer must hold one whole frame plus whatever a read happened to
/// deliver, so it is sized to a frame rather than to a socket read.
pub const MAX_BUFFER_LEN: usize = MAX_FRAME_LEN;
