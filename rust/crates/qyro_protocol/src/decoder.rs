//! Incremental frame decoder.
//!
//! A socket read is not a frame. It may deliver half a header, several frames,
//! or a frame split at any byte. This decoder buffers whatever arrives and
//! yields frames only once they are complete, under a hard memory ceiling.
//!
//! It distinguishes two kinds of failure, per
//! `docs/adr/ADR-0018-protocol-semantic-errors.md`:
//!
//! - **Structural** failures mean framing itself is no longer trustworthy. The
//!   decoder is poisoned and only an explicit [`FrameDecoder::reset`] recovers.
//! - **Delimited semantic events**, today just an unknown message type, keep the
//!   stream synchronised: the frame is consumed whole and reported as
//!   [`DecodedFrame::Unsupported`].

use crate::envelope::EncryptedEnvelope;
use crate::error::FrameError;
use crate::frame::Frame;
use crate::header::{ParsedHeader, UnknownHeader};
use crate::limits::{HEADER_LEN, MAX_BUFFER_LEN};
use crate::message::MessageType;

/// A frame the peer sent whose type this version does not implement.
///
/// Carries enough to answer `Error` without re-parsing bytes, and deliberately
/// does **not** expose the payload: bytes whose meaning is unknown must not
/// become something the application can process.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedFrame {
    message_type_value: u8,
    payload_len: u32,
    total_len: usize,
    session_id: u64,
    transfer_id: u64,
    sequence: u64,
}

impl UnsupportedFrame {
    /// Builds the event from a parsed unknown header and its consumed size.
    pub(crate) const fn from(unknown: UnknownHeader, total_len: usize) -> Self {
        Self {
            message_type_value: unknown.raw_message_type,
            payload_len: unknown.payload_len,
            total_len,
            session_id: unknown.session_id,
            transfer_id: unknown.transfer_id,
            sequence: unknown.sequence,
        }
    }

    /// The wire value that did not map to a known message.
    #[must_use]
    pub const fn message_type_value(&self) -> u8 {
        self.message_type_value
    }

    /// Payload length the frame declared.
    #[must_use]
    pub const fn payload_len(&self) -> u32 {
        self.payload_len
    }

    /// Total bytes consumed from the stream.
    #[must_use]
    pub const fn total_len(&self) -> usize {
        self.total_len
    }

    /// Session the frame belonged to.
    #[must_use]
    pub const fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Transfer the frame belonged to.
    #[must_use]
    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    /// Sequence number the frame carried.
    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }
}

/// What the decoder produced for one complete frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodedFrame {
    /// A frame this version understands.
    Message(Frame),
    /// A frame whose payload is ciphertext, **not yet verified**.
    ///
    /// No plaintext is available here and none may be inferred: this crate
    /// computes no tags, so the trailer proves nothing until `qyro_crypto`
    /// checks it.
    Encrypted(EncryptedEnvelope),
    /// A well-formed frame whose type this version does not implement.
    Unsupported(UnsupportedFrame),
}

impl DecodedFrame {
    /// Returns the message type, or `None` when the type is not implemented.
    #[must_use]
    pub const fn message_type(&self) -> Option<MessageType> {
        match self {
            Self::Message(frame) => Some(frame.message_type()),
            Self::Encrypted(envelope) => Some(envelope.message_type()),
            Self::Unsupported(_) => None,
        }
    }

    /// Returns the plaintext, and only when there genuinely is some.
    ///
    /// `None` for an encrypted envelope and for an unsupported frame. An empty
    /// slice used to stand in for all three, which made "the peer sent an empty
    /// message", "this is ciphertext" and "this type is unknown" indistinguishable.
    #[must_use]
    pub fn plaintext(&self) -> Option<&[u8]> {
        match self {
            Self::Message(frame) => Some(frame.payload()),
            Self::Encrypted(_) | Self::Unsupported(_) => None,
        }
    }

    /// Re-serializes the frame.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::UnknownMessageType`] for an unsupported frame,
    /// whose bytes were deliberately not retained. Reporting it beats panicking:
    /// the variant came from legitimate peer input, so a caller iterating over
    /// decoded frames must not be able to crash the process by re-encoding one.
    pub fn try_encode(&self) -> Result<Vec<u8>, FrameError> {
        match self {
            Self::Message(frame) => Ok(frame.encode()),
            Self::Encrypted(envelope) => Ok(envelope.encode()),
            Self::Unsupported(event) => Err(FrameError::UnknownMessageType {
                value: event.message_type_value(),
            }),
        }
    }

    /// Returns the plain frame, if this is one.
    #[must_use]
    pub const fn as_plain(&self) -> Option<&Frame> {
        match self {
            Self::Message(frame) => Some(frame),
            Self::Encrypted(_) | Self::Unsupported(_) => None,
        }
    }

    /// Returns the encrypted envelope, if this is one.
    ///
    /// The envelope is unverified; see [`EncryptedEnvelope`].
    #[must_use]
    pub const fn as_encrypted(&self) -> Option<&EncryptedEnvelope> {
        match self {
            Self::Encrypted(envelope) => Some(envelope),
            Self::Message(_) | Self::Unsupported(_) => None,
        }
    }
}

/// Buffers bytes and yields complete frames.
#[derive(Debug)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    /// Bytes at the front of `buffer` that have already left as frames.
    ///
    /// Yielding a frame used to reclaim its bytes immediately with
    /// `drain(..total)`, which memmoves the whole rest of the buffer — so a
    /// peer sending small frames made the receive loop do quadratic work with
    /// perfectly valid traffic (QYR-0024). Now a frame only moves this cursor,
    /// and the space is reclaimed in [`Self::compact`] on a schedule that keeps
    /// the cost amortized.
    read: usize,
    max_buffer_len: usize,
    poisoned: Option<FrameError>,
    /// Bytes memmoved by reclaiming the consumed prefix of the buffer.
    ///
    /// `cfg(test)`, never a feature. It exists because the cost of this decoder
    /// is a security property — a peer choosing frame sizes chooses how much
    /// work the receive loop does — and a property nothing measures is a
    /// comment. A wall clock cannot be that measurement: it is unstable on a
    /// shared runner and it does not say what broke.
    #[cfg(test)]
    bytes_moved: u64,
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    /// Creates a decoder bounded by [`MAX_BUFFER_LEN`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buffer: Vec::new(),
            read: 0,
            max_buffer_len: MAX_BUFFER_LEN,
            poisoned: None,
            #[cfg(test)]
            bytes_moved: 0,
        }
    }

    /// Creates a decoder with a custom buffer ceiling.
    ///
    /// The ceiling is clamped to [`MAX_BUFFER_LEN`] so a caller cannot widen the
    /// bound past what the protocol guarantees.
    #[must_use]
    pub const fn with_max_buffer_len(max_buffer_len: usize) -> Self {
        let bounded = if max_buffer_len > MAX_BUFFER_LEN {
            MAX_BUFFER_LEN
        } else {
            max_buffer_len
        };
        Self {
            buffer: Vec::new(),
            read: 0,
            max_buffer_len: bounded,
            poisoned: None,
            #[cfg(test)]
            bytes_moved: 0,
        }
    }

    /// Bytes currently buffered.
    #[must_use]
    pub fn buffered_len(&self) -> usize {
        self.buffer.len() - self.read
    }

    /// Buffer capacity, exposed so tests can assert no hostile reservation.
    #[must_use]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// Whether a structural failure has poisoned the stream.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned.is_some()
    }

    /// Clears the buffer and the poisoned state.
    ///
    /// The only way out of a structural failure, and deliberately explicit:
    /// pushing more bytes must never look like recovery.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.read = 0;
        self.poisoned = None;
    }

    /// Appends received bytes.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::BufferLimitExceeded`] when the bytes would push the
    /// buffer past its ceiling. The buffer is left untouched, so the caller can
    /// drain frames and retry rather than losing the connection state.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), FrameError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        let attempted =
            self.buffer
                .len()
                .checked_add(bytes.len())
                .ok_or(FrameError::BufferLimitExceeded {
                    attempted: usize::MAX,
                    limit: self.max_buffer_len,
                })?;
        if attempted > self.max_buffer_len {
            return Err(FrameError::BufferLimitExceeded {
                attempted,
                limit: self.max_buffer_len,
            });
        }

        // Reclaim the consumed prefix when the incoming bytes would not
        // otherwise fit under the ceiling, or when at least half the buffer is
        // already spent. The second clause is what makes the cost amortized: a
        // compaction moves at most half the buffer and cannot happen again
        // until that much has been consumed, so a byte is copied a bounded
        // number of times between arriving and leaving.
        if self.read > 0
            && (self.buffer.len() + bytes.len() > self.max_buffer_len
                || self.read >= self.buffer.len() / 2)
        {
            self.compact();
        }

        self.reserve_for(bytes.len());
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    /// Returns the next complete frame, or `None` when more bytes are needed.
    ///
    /// The declared lengths are validated by [`FrameHeader::decode`] before this
    /// method computes how many bytes to wait for, so a hostile length becomes
    /// an error instead of a reservation or an unbounded wait.
    ///
    /// # Errors
    ///
    /// Returns the structural [`FrameError`] that poisoned the stream. An
    /// unknown message type is **not** an error here; it arrives as
    /// [`DecodedFrame::Unsupported`].
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, FrameError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        if self.buffered_len() < HEADER_LEN {
            return Ok(None);
        }

        // Everything below indexes from `self.read`, never from zero: the bytes
        // before it belong to frames already yielded and are reclaimed later.
        let start = self.read;
        let Some(header_bytes) = self.buffer.get(start..start + HEADER_LEN) else {
            return Ok(None);
        };
        let parsed = match ParsedHeader::parse_from(header_bytes) {
            Ok(parsed) => parsed,
            Err(error) => return Err(self.poison(error)),
        };
        let total_declared = parsed.total_len();

        // total_len() is u64 and was already bounded by MAX_FRAME_LEN, so this
        // conversion cannot truncate on any supported target.
        let Ok(total) = usize::try_from(total_declared) else {
            return Err(self.poison(FrameError::FrameTooLarge {
                declared: total_declared,
                limit: self.max_buffer_len as u64,
            }));
        };

        if total > self.max_buffer_len {
            return Err(self.poison(FrameError::BufferLimitExceeded {
                attempted: total,
                limit: self.max_buffer_len,
            }));
        }

        if self.buffered_len() < total {
            return Ok(None);
        }

        // Fully delimited from here on, so an unknown type is consumed cleanly
        // and the stream stays synchronised. (ADR-0018)
        let header = match parsed {
            ParsedHeader::Unknown(unknown) => {
                self.read += total;
                return Ok(Some(DecodedFrame::Unsupported(UnsupportedFrame::from(
                    unknown, total,
                ))));
            }
            ParsedHeader::Known(header) => header,
        };

        // Ciphertext keeps its trailer and never becomes a plain payload.
        if header.flags().contains(crate::message::Flags::ENCRYPTED) {
            let body = self
                .buffer
                .get(start + HEADER_LEN..start + total)
                .unwrap_or_default()
                .to_vec();
            self.read += total;
            return match EncryptedEnvelope::from_parts(header, &body) {
                Ok(envelope) => Ok(Some(DecodedFrame::Encrypted(envelope))),
                Err(error) => Err(self.poison(error)),
            };
        }

        let payload_end = start + HEADER_LEN + header.payload_len() as usize;
        let payload = self
            .buffer
            .get(start + HEADER_LEN..payload_end)
            .unwrap_or_default()
            .to_vec();
        self.read += total;

        match Frame::from_parts(header, payload) {
            Ok(frame) => Ok(Some(DecodedFrame::Message(frame))),
            Err(error) => Err(self.poison(error)),
        }
    }

    /// Drops the consumed prefix, sliding the rest of the buffer down.
    ///
    /// This is the only place that moves bytes, and its callers are chosen so
    /// that it cannot run more often than the bytes consumed pay for.
    fn compact(&mut self) {
        if self.read == 0 {
            return;
        }
        #[cfg(test)]
        {
            self.bytes_moved += (self.buffer.len() - self.read) as u64;
        }
        self.buffer.drain(..self.read);
        self.read = 0;
    }

    /// Makes room for `additional` bytes without letting the capacity pass the
    /// ceiling.
    ///
    /// `Vec` doubles on its own and would reach 2 097 152 against a
    /// `MAX_BUFFER_LEN` of 1 049 664 (QYR-0027). Doubling is still what keeps a
    /// push amortized O(1), so the growth is kept and clamped: the target is
    /// whichever is larger of what this push needs and twice the current
    /// capacity, capped at the ceiling. Since the caller has already refused
    /// anything that would exceed the ceiling, the capped value is never below
    /// what is needed.
    fn reserve_for(&mut self, additional: usize) {
        let needed = self.buffer.len() + additional;
        if needed <= self.buffer.capacity() {
            return;
        }
        let doubled = self.buffer.capacity().saturating_mul(2);
        let target = needed.max(doubled.min(self.max_buffer_len));
        self.buffer.reserve_exact(target - self.buffer.len());
    }

    /// Bytes memmoved so far to reclaim consumed buffer space.
    #[cfg(test)]
    pub(crate) const fn bytes_moved(&self) -> u64 {
        self.bytes_moved
    }

    fn poison(&mut self, error: FrameError) -> FrameError {
        self.poisoned = Some(error);
        error
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    reason = "a test that cannot assert or index reports failures worse"
)]
mod cost_tests {
    use super::{FrameDecoder, HEADER_LEN, MAX_BUFFER_LEN};
    use crate::error::FrameError;
    use crate::frame::Frame;
    use crate::limits::MAX_PAYLOAD_LEN;
    use crate::message::MessageType;

    /// The smallest well-formed frame: a header and nothing else.
    fn minimal_frame() -> Vec<u8> {
        Frame::new(MessageType::Heartbeat, Vec::new())
            .expect("a heartbeat with no payload is a valid frame")
            .encode()
    }

    #[test]
    fn draining_a_full_buffer_copies_a_bounded_number_of_bytes() {
        // QYR-0024. `next_frame` reclaimed the frame it had just yielded with
        // `drain(..total)`, which memmoves the entire rest of the buffer. Filling
        // the buffer with minimal frames and draining it therefore costs
        // Theta(n^2 / frame_len): a peer sending well-formed heartbeats — valid
        // traffic, no errors, nothing a validity-based limiter would catch —
        // makes the receive loop do quadratic work.
        //
        // Counted, not timed. A wall clock on a shared runner measures the
        // runner.
        let frame = minimal_frame();
        let mut decoder = FrameDecoder::new();

        let mut pushed = 0usize;
        while pushed + frame.len() <= MAX_BUFFER_LEN {
            decoder.push(&frame).expect("within the ceiling");
            pushed += frame.len();
        }
        let frames = pushed / frame.len();
        assert!(frames > 20_000, "the buffer should hold many frames");

        while decoder
            .next_frame()
            .expect("every frame is well formed")
            .is_some()
        {}
        assert_eq!(decoder.buffered_len(), 0, "everything was consumed");

        // The bound: reclaiming space may copy each pushed byte a small constant
        // number of times, so the work is proportional to the bytes that arrived
        // and not to their square.
        let limit = 2 * pushed as u64;
        assert!(
            decoder.bytes_moved() <= limit,
            "draining {frames} frames moved {} bytes, more than the {limit} a \
             bounded reclaim allows. A byte must be copied a bounded number of \
             times between entering the buffer and leaving it (ADR-0016).",
            decoder.bytes_moved()
        );
    }

    #[test]
    fn the_cost_of_draining_does_not_grow_with_the_buffer() {
        // The same bound at five sizes, which is what makes it a bound rather
        // than a threshold somebody tuned until one case passed. With a
        // `drain` per frame the moved count grows with the square of the
        // ceiling, so this fails at the smallest size and gets worse from
        // there.
        let frame = minimal_frame();

        for ceiling in [
            64 * 1024,
            128 * 1024,
            256 * 1024,
            512 * 1024,
            MAX_BUFFER_LEN,
        ] {
            let mut decoder = FrameDecoder::with_max_buffer_len(ceiling);
            let mut pushed = 0u64;
            while pushed as usize + frame.len() <= ceiling {
                decoder.push(&frame).expect("within the ceiling");
                pushed += frame.len() as u64;
            }
            while decoder.next_frame().expect("well formed").is_some() {}

            assert_eq!(decoder.buffered_len(), 0, "everything was consumed");
            assert!(
                decoder.bytes_moved() <= 2 * pushed,
                "at a ceiling of {ceiling} bytes, draining moved {} against \
                 {pushed} pushed; the cost per byte must not grow with the size \
                 of the buffer",
                decoder.bytes_moved()
            );
        }
    }

    #[test]
    fn a_socket_loop_with_a_backlog_stays_bounded() {
        // The shape a transport actually produces, and the one that matters:
        // fill-then-drain never compacts at all, so it cannot show that
        // compaction is amortized. Here a backlog is held while frames keep
        // arriving, so every compaction has something to move.
        //
        // With a `drain` per frame this is the worst case rather than the
        // interesting one: each of the 50 000 frames would memmove the whole
        // backlog.
        let frame = minimal_frame();
        let mut decoder = FrameDecoder::new();
        let mut pushed = 0u64;

        for _ in 0..4_096 {
            decoder.push(&frame).expect("building the backlog");
            pushed += frame.len() as u64;
        }
        for _ in 0..50_000 {
            decoder.push(&frame).expect("one more arrives");
            pushed += frame.len() as u64;
            decoder
                .next_frame()
                .expect("well formed")
                .expect("a whole frame is buffered");
        }

        assert!(
            decoder.bytes_moved() <= 2 * pushed,
            "a backlogged socket loop moved {} bytes against {pushed} pushed",
            decoder.bytes_moved()
        );
    }

    #[test]
    fn a_buffer_filled_one_byte_at_a_time_still_yields_its_frames() {
        // Byte-at-a-time delivery is what a slow or hostile peer produces, and
        // it is the path that walked the capacity past the ceiling.
        let frame = minimal_frame();
        let mut decoder = FrameDecoder::new();
        let mut pushed = 0usize;

        while pushed + frame.len() <= MAX_BUFFER_LEN {
            for byte in &frame {
                decoder
                    .push(core::slice::from_ref(byte))
                    .expect("one byte at a time");
            }
            pushed += frame.len();
            assert!(decoder.buffer_capacity() <= MAX_BUFFER_LEN);
        }

        let mut yielded = 0usize;
        while decoder.next_frame().expect("well formed").is_some() {
            yielded += 1;
        }
        assert_eq!(yielded, pushed / frame.len(), "every frame came back out");
        assert!(decoder.bytes_moved() <= 2 * pushed as u64);
    }

    #[test]
    fn a_maximum_frame_dripped_one_byte_at_a_time_arrives_whole() {
        // One frame the size of the whole ceiling, delivered in the worst
        // possible shape. The cursor must not confuse "nothing consumed yet"
        // with "nothing buffered", and the capacity must still not overshoot.
        let frame = Frame::new(MessageType::DataChunk, vec![0x5A; MAX_PAYLOAD_LEN])
            .expect("a maximum payload is a valid frame")
            .encode();
        let mut decoder = FrameDecoder::new();

        for byte in &frame {
            decoder
                .push(core::slice::from_ref(byte))
                .expect("the frame fits under the ceiling");
            assert!(decoder.buffer_capacity() <= MAX_BUFFER_LEN);
        }

        let decoded = decoder
            .next_frame()
            .expect("well formed")
            .expect("the whole frame is buffered");
        assert_eq!(
            decoded.plaintext().map(<[u8]>::len),
            Some(MAX_PAYLOAD_LEN),
            "the payload arrived whole"
        );
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn a_frame_larger_than_a_custom_ceiling_is_refused_not_awaited() {
        // A small ceiling and a header declaring far more than it. The decoder
        // must refuse and poison rather than wait for bytes it would never
        // accept — a wait here is a connection that hangs for ever.
        let mut oversize = Frame::new(MessageType::DataChunk, vec![0u8; 1_000])
            .expect("a valid frame")
            .encode();
        oversize.truncate(HEADER_LEN);

        let mut decoder = FrameDecoder::with_max_buffer_len(128);
        decoder.push(&oversize).expect("the header alone fits");

        assert!(
            matches!(
                decoder.next_frame(),
                Err(FrameError::BufferLimitExceeded {
                    attempted: 1_048,
                    ..
                })
            ),
            "a frame that cannot fit must be an error, not a wait"
        );
        assert!(decoder.is_poisoned(), "and it must poison the stream");
    }

    #[test]
    fn poisoning_and_reset_survive_the_cursor() {
        // The compaction change is internal, and this is the invariant most
        // likely to break silently if it were not: a structural failure still
        // poisons, `reset` is still the only way out, and the cursor does not
        // leak consumed bytes across it.
        let frame = minimal_frame();
        let mut decoder = FrameDecoder::new();

        decoder.push(&frame).expect("a valid frame");
        assert!(decoder.next_frame().expect("well formed").is_some());

        decoder
            .push(&[0u8; HEADER_LEN])
            .expect("garbage still buffers");
        assert!(decoder.next_frame().is_err(), "bad magic is structural");
        assert!(decoder.is_poisoned());
        assert!(
            decoder.push(&frame).is_err(),
            "a poisoned decoder takes nothing"
        );

        decoder.reset();
        assert!(!decoder.is_poisoned());
        assert_eq!(decoder.buffered_len(), 0, "reset clears the cursor too");

        decoder.push(&frame).expect("usable again");
        assert!(decoder.next_frame().expect("well formed").is_some());
        assert_eq!(decoder.buffered_len(), 0);
    }

    #[test]
    fn the_buffer_never_reserves_more_than_its_limit() {
        // QYR-0027. `push` bounds `len`, and `Vec::extend_from_slice` grows
        // `capacity` geometrically, so dripping one byte at a time walks the
        // capacity past the ceiling the decoder exists to enforce: measured at
        // 2 097 152 against a MAX_BUFFER_LEN of 1 049 664.
        //
        // Two existing assertions in `wire_contract.rs` and `property.rs` claim
        // this already holds. They pass because neither ever fills the buffer.
        let mut decoder = FrameDecoder::new();
        let byte = [0u8; 1];

        while decoder.buffered_len() < MAX_BUFFER_LEN {
            decoder
                .push(&byte)
                .expect("one byte at a time, under the ceiling");
            assert!(
                decoder.buffer_capacity() <= MAX_BUFFER_LEN,
                "capacity reached {} at {} buffered bytes, past the \
                 MAX_BUFFER_LEN of {MAX_BUFFER_LEN}",
                decoder.buffer_capacity(),
                decoder.buffered_len()
            );
        }
        assert_eq!(decoder.buffered_len(), MAX_BUFFER_LEN);
    }
}
