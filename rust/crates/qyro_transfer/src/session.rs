//! The transfer session: a sender and a receiver that only exchange bytes.
//!
//! Specification: `docs/adr/ADR-0026-transfer-session.md`.
//!
//! Neither end holds a reference to the other. Every method takes bytes in and
//! hands bytes out, which is what makes it possible to drop, reorder or replay a
//! frame in a test without a transport existing.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use qyro_crypto::aead::{FrameOpener, FrameSealer};
use qyro_manifest::TransferManifest;
use qyro_protocol::{DecodedFrame, Frame, FrameDecoder, MessageType};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::error::{ItemVerdict, TransferError};
use crate::wire::{self, Accept, Ack, ChunkRef, Complete, Control, Integrity, ItemStart, Offer};

/// Content bytes per chunk. ADR-0026 §2.
pub const CHUNK_SIZE: usize = 65_536;

/// Chunks the sender may have unacknowledged. ADR-0026 §3.
pub const WINDOW_CHUNKS: u32 = 16;

/// Where a sender reads item content.
///
/// A trait rather than a slice so the engine never has to be handed a whole
/// file. What the caller does behind it is the caller's business; what matters
/// here is that the engine only ever asks for one chunk at a time.
pub trait ContentSource {
    /// Fills `out` from `item_id` starting at `offset`, returning bytes written.
    fn read_at(&self, item_id: u32, offset: u64, out: &mut [u8]) -> usize;
}

/// Where a receiver puts verified content.
///
/// Also a trait, and for the same reason from the other side: 5A writes to a
/// buffer, 5B will write to a `.qyro-part` file, and the engine should not know
/// the difference.
pub trait ContentSink {
    /// Accepts `bytes` for `item_id` at `offset`.
    fn write_at(&mut self, item_id: u32, offset: u64, bytes: &[u8]);
}

/// What phase an end of the transfer is in.
///
/// Every inbound message is dispatched against this, and one with no transition
/// from here is [`TransferError::UnexpectedMessage`] — refused by type rather
/// than tolerated (ADR-0026 §4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Sender: offer sent, waiting for accept. Receiver: waiting for an offer.
    Negotiating,
    /// Manifest exchanged; content is moving.
    Transferring,
    /// Paused by either end. Content stops; control still flows.
    Paused,
    /// Sender: `Complete` sent, waiting for the verdicts.
    AwaitingIntegrity,
    /// Terminal, and the transfer succeeded as far as this end knows.
    Done,
    /// Terminal by request.
    Cancelled,
    /// Terminal by refusal. Nothing more is accepted.
    Poisoned,
}

impl Phase {
    /// Whether this phase accepts anything at all.
    const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled | Self::Poisoned)
    }
}

/// Per-item bookkeeping on the sending side.
#[derive(Clone, Copy, Debug)]
struct SendItem {
    item_id: u32,
    size: u64,
    chunks_total: u32,
    /// Next index to put on the wire.
    next_to_send: u32,
    /// Highest contiguous index the receiver confirmed, if any.
    acked_through: Option<u32>,
    started: bool,
}

impl SendItem {
    /// Chunks sent and not yet acknowledged.
    const fn in_flight(&self) -> u32 {
        let acked = match self.acked_through {
            Some(index) => index.saturating_add(1),
            None => 0,
        };
        self.next_to_send.saturating_sub(acked)
    }

    const fn is_drained(&self) -> bool {
        self.next_to_send >= self.chunks_total
    }

    const fn is_acked(&self) -> bool {
        match self.acked_through {
            Some(index) => index.saturating_add(1) >= self.chunks_total,
            None => self.chunks_total == 0,
        }
    }
}

/// How many chunks an item of `size` needs at `chunk_size`.
fn chunk_count(size: u64, chunk_size: usize) -> u32 {
    if size == 0 {
        return 0;
    }
    let chunk = chunk_size as u64;
    let count = size.div_ceil(chunk);
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The sending end.
pub struct Sender {
    sealer: FrameSealer,
    opener: FrameOpener,
    decoder: FrameDecoder,
    manifest: TransferManifest,
    phase: Phase,
    window: u32,
    items: Vec<SendItem>,
    total_sent: u64,
    integrity: Option<Vec<(u32, ItemVerdict)>>,
    /// Peak content bytes this engine held at once, counted under test.
    ///
    /// A counter and not a clock, for the reason the decoder's `bytes_moved`
    /// exists: a wall clock on a shared runner measures the runner.
    #[cfg(test)]
    pub(crate) peak_content_held: usize,
}

impl Sender {
    /// Builds a sender over an established session's sealer and opener.
    #[must_use]
    pub fn new(sealer: FrameSealer, opener: FrameOpener, manifest: TransferManifest) -> Self {
        let items = manifest
            .items()
            .iter()
            .map(|item| SendItem {
                item_id: item.item_id(),
                size: item.size(),
                chunks_total: chunk_count(item.size(), CHUNK_SIZE),
                next_to_send: 0,
                acked_through: None,
                started: false,
            })
            .collect();
        Self {
            sealer,
            opener,
            decoder: FrameDecoder::new(),
            manifest,
            phase: Phase::Negotiating,
            window: WINDOW_CHUNKS,
            items,
            total_sent: 0,
            integrity: None,
            #[cfg(test)]
            peak_content_held: 0,
        }
    }

    /// The phase this end believes it is in.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// The verdicts the receiver reported, once it has.
    #[must_use]
    pub fn integrity(&self) -> Option<&[(u32, ItemVerdict)]> {
        self.integrity.as_deref()
    }

    /// Seals `payload` as `message_type` and returns the bytes to put on the wire.
    fn emit(
        &mut self,
        message_type: MessageType,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, TransferError> {
        let frame = Frame::new(message_type, payload).map_err(|_| TransferError::Framing)?;
        let sealed = self
            .sealer
            .seal(&frame)
            .map_err(|_| TransferError::Framing)?;
        Ok(sealed.encode())
    }

    /// The opening offer, and the manifest behind it.
    ///
    /// # Errors
    ///
    /// [`TransferError::Framing`] when a frame cannot be sealed.
    pub fn open(&mut self) -> Result<Vec<Vec<u8>>, TransferError> {
        if self.phase != Phase::Negotiating {
            return Err(self.poison(TransferError::SessionPoisoned));
        }
        let total: u64 = self.items.iter().map(|item| item.size).sum();
        let offer = Offer {
            item_count: u32::try_from(self.items.len()).unwrap_or(u32::MAX),
            total_bytes: total,
            chunk_size: u32::try_from(CHUNK_SIZE).unwrap_or(u32::MAX),
            window_chunks: self.window,
        };
        let offer_bytes = self.emit(MessageType::TransferOffer, offer.encode())?;
        let manifest_bytes =
            qyro_manifest::codec::encode(&self.manifest).map_err(|_| TransferError::Framing)?;
        let manifest_frame = self.emit(MessageType::Manifest, manifest_bytes)?;
        Ok(vec![offer_bytes, manifest_frame])
    }

    /// Marks the session poisoned and returns the error that did it.
    fn poison(&mut self, error: TransferError) -> TransferError {
        if !self.phase.is_terminal() {
            self.phase = Phase::Poisoned;
        }
        error
    }

    /// Produces as many chunks as the window allows.
    ///
    /// # Errors
    ///
    /// Propagates framing failures; refuses when the session is terminal.
    pub fn pump(&mut self, source: &dyn ContentSource) -> Result<Vec<Vec<u8>>, TransferError> {
        match self.phase {
            Phase::Transferring => {}
            Phase::Paused => return Ok(Vec::new()),
            Phase::Cancelled => return Err(TransferError::Cancelled),
            _ => return Ok(Vec::new()),
        }

        let mut out = Vec::new();
        let mut buffer = vec![0u8; CHUNK_SIZE];
        #[cfg(test)]
        {
            // The engine holds one chunk buffer at a time. Recorded here rather
            // than asserted, so the test does the asserting.
            self.peak_content_held = self.peak_content_held.max(buffer.len());
        }

        loop {
            let Some(index) = self.items.iter().position(|item| !item.is_drained()) else {
                break;
            };
            let Some(item) = self.items.get(index) else {
                break;
            };
            let item = *item;

            if item.in_flight() >= self.window {
                break;
            }

            if !item.started {
                let start = ItemStart {
                    item_id: item.item_id,
                    item_bytes: item.size,
                };
                let bytes = self.emit(MessageType::ItemStart, start.encode())?;
                out.push(bytes);
                if let Some(slot) = self.items.get_mut(index) {
                    slot.started = true;
                }
            }

            let chunk_index = item.next_to_send;
            let offset = u64::from(chunk_index).saturating_mul(CHUNK_SIZE as u64);
            let remaining = item.size.saturating_sub(offset);
            let want = usize::try_from(remaining.min(CHUNK_SIZE as u64)).unwrap_or(CHUNK_SIZE);
            let filled = match buffer.get_mut(..want) {
                Some(slice) => source.read_at(item.item_id, offset, slice),
                None => 0,
            };
            let content = buffer.get(..filled).unwrap_or(&[]);
            let chunk = ChunkRef {
                item_id: item.item_id,
                chunk_index,
                content,
            };
            let bytes = self.emit(MessageType::DataChunk, chunk.encode())?;
            out.push(bytes);

            if let Some(slot) = self.items.get_mut(index) {
                slot.next_to_send = slot.next_to_send.saturating_add(1);
            }
            self.total_sent = self.total_sent.saturating_add(filled as u64);
        }

        if self.items.iter().all(SendItem::is_acked) && self.phase == Phase::Transferring {
            let complete = Complete {
                total_bytes: self.items.iter().map(|item| item.size).sum(),
            };
            let bytes = self.emit(MessageType::Complete, complete.encode())?;
            out.push(bytes);
            self.phase = Phase::AwaitingIntegrity;
        }

        Ok(out)
    }

    /// Total content bytes this sender has put on the wire.
    #[must_use]
    pub const fn bytes_sent(&self) -> u64 {
        self.total_sent
    }

    /// Chunks currently sent and unacknowledged, across all items.
    #[must_use]
    pub fn chunks_in_flight(&self) -> u32 {
        self.items.iter().map(SendItem::in_flight).sum()
    }

    /// Re-sends from `chunk_index`, and rewinds the window to match.
    ///
    /// **Go-back-N**, and it has to be: cumulative ACK plus a receiver that does
    /// not buffer out of order means everything after a gap was discarded, so
    /// resending only the missing chunk would leave the rest missing too. The
    /// engine rewinds `next_to_send` here rather than making the caller notice
    /// the stall, because a caller that has to notice is a caller that can
    /// forget.
    ///
    /// The frame is a **new** one with a new sequence, sealed again. Re-sending
    /// the same sealed bytes would be refused by the peer's replay window,
    /// correctly (ADR-0026 §5).
    ///
    /// # Errors
    ///
    /// [`TransferError::UnknownItem`] for an item this manifest has no entry
    /// for; framing failures otherwise.
    pub fn retransmit(
        &mut self,
        item_id: u32,
        chunk_index: u32,
        source: &dyn ContentSource,
    ) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        let Some(item) = self.items.iter().find(|item| item.item_id == item_id) else {
            return Err(self.poison(TransferError::UnknownItem { item_id }));
        };
        let item = *item;
        let offset = u64::from(chunk_index).saturating_mul(CHUNK_SIZE as u64);
        let remaining = item.size.saturating_sub(offset);
        let want = usize::try_from(remaining.min(CHUNK_SIZE as u64)).unwrap_or(CHUNK_SIZE);
        let mut buffer = vec![0u8; want];
        let filled = source.read_at(item_id, offset, &mut buffer);
        let chunk = ChunkRef {
            item_id,
            chunk_index,
            content: buffer.get(..filled).unwrap_or(&[]),
        };
        let frame = self.emit(MessageType::DataChunk, chunk.encode())?;
        if let Some(slot) = self.items.iter_mut().find(|item| item.item_id == item_id) {
            slot.next_to_send = chunk_index.saturating_add(1);
        }
        Ok(frame)
    }

    /// Asks the peer to pause.
    ///
    /// # Errors
    ///
    /// Framing failures, or a terminal session.
    pub fn request_pause(&mut self) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        self.phase = Phase::Paused;
        self.emit(MessageType::Pause, Control::USER.encode())
    }

    /// Asks the peer to resume.
    ///
    /// # Errors
    ///
    /// Framing failures, or a terminal session.
    pub fn request_resume(&mut self) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        self.phase = Phase::Transferring;
        self.emit(MessageType::Resume, Control::USER.encode())
    }

    /// Cancels the transfer from this end.
    ///
    /// # Errors
    ///
    /// Framing failures, or a session that already ended.
    pub fn request_cancel(&mut self) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        let bytes = self.emit(MessageType::Cancel, Control::USER.encode())?;
        self.phase = Phase::Cancelled;
        Ok(bytes)
    }

    /// Feeds inbound bytes and applies whatever they say.
    ///
    /// # Errors
    ///
    /// One variant per refusal in ADR-0026 §4.
    pub fn deliver(&mut self, bytes: &[u8]) -> Result<(), TransferError> {
        if self.phase == Phase::Poisoned {
            return Err(TransferError::SessionPoisoned);
        }
        // Poisoning here and not just propagating: a frame that did not
        // authenticate has no known sender, so this end can no longer say what
        // state the peer is in. ADR-0026 §4.
        let opened = open_all(&mut self.decoder, &mut self.opener, bytes)
            .map_err(|error| self.poison(error))?;
        for (message_type, payload) in opened {
            self.apply(message_type, &payload)?;
        }
        Ok(())
    }

    fn apply(&mut self, message_type: MessageType, payload: &[u8]) -> Result<(), TransferError> {
        match (self.phase, message_type) {
            (Phase::Negotiating, MessageType::TransferAccept) => {
                let accept = Accept::decode(payload).map_err(|error| self.poison(error))?;
                if accept.window_chunks > self.window {
                    return Err(self.poison(TransferError::WindowGrantTooLarge {
                        offered: self.window,
                        granted: accept.window_chunks,
                    }));
                }
                self.window = accept.window_chunks;
                self.phase = Phase::Transferring;
                Ok(())
            }
            (Phase::Transferring | Phase::Paused, MessageType::ChunkAck) => {
                let ack = Ack::decode(payload).map_err(|error| self.poison(error))?;
                let Some(index) = self
                    .items
                    .iter()
                    .position(|item| item.item_id == ack.item_id)
                else {
                    return Err(self.poison(TransferError::UnknownItem {
                        item_id: ack.item_id,
                    }));
                };
                let sent = self.items.get(index).map_or(0, |item| item.next_to_send);
                if ack.through_index.saturating_add(1) > sent {
                    return Err(self.poison(TransferError::AckAheadOfSender {
                        item_id: ack.item_id,
                        through: ack.through_index,
                        sent,
                    }));
                }
                if let Some(slot) = self.items.get_mut(index) {
                    let better = match slot.acked_through {
                        Some(current) => ack.through_index.max(current),
                        None => ack.through_index,
                    };
                    slot.acked_through = Some(better);
                }
                Ok(())
            }
            (Phase::Transferring | Phase::Paused, MessageType::Pause) => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Paused;
                Ok(())
            }
            (Phase::Paused | Phase::Transferring, MessageType::Resume) => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Transferring;
                Ok(())
            }
            (_, MessageType::Cancel) if !self.phase.is_terminal() => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Cancelled;
                Ok(())
            }
            (Phase::AwaitingIntegrity, MessageType::IntegrityResult) => {
                let result = Integrity::decode(payload).map_err(|error| self.poison(error))?;
                self.integrity = Some(result.verdicts);
                self.phase = Phase::Done;
                Ok(())
            }
            (_, got) => Err(self.poison(TransferError::UnexpectedMessage { got })),
        }
    }
}

/// Per-item bookkeeping on the receiving side.
struct RecvItem {
    item_id: u32,
    size: u64,
    expected_chunks: u32,
    /// The next index that would extend the contiguous run.
    next_expected: u32,
    hasher: Sha256,
    received: u64,
    expected_digest: Vec<u8>,
    verdict: Option<ItemVerdict>,
}

/// The receiving end.
pub struct Receiver {
    sealer: FrameSealer,
    opener: FrameOpener,
    decoder: FrameDecoder,
    phase: Phase,
    window: u32,
    items: Vec<RecvItem>,
    manifest_seen: bool,
    verdicts_sent: bool,
    /// The manifest this receiver accepted, kept rather than only consumed.
    ///
    /// Added in sprint 6A. The receiver derived `items` from the manifest and
    /// then dropped it, which is enough while both ends live in one process and
    /// the caller already holds a copy. Over a socket the receiving end learns
    /// the manifest **only** from the wire, and a filesystem sink is built from
    /// a `&TransferManifest` -- so with no way to read it back, no real
    /// receiver could materialise a file. Retaining it costs one clone of a
    /// structure the peer already sent.
    manifest: Option<TransferManifest>,
}

impl Receiver {
    /// Builds a receiver over an established session's sealer and opener.
    #[must_use]
    pub fn new(sealer: FrameSealer, opener: FrameOpener) -> Self {
        Self {
            sealer,
            opener,
            decoder: FrameDecoder::new(),
            phase: Phase::Negotiating,
            window: WINDOW_CHUNKS,
            items: Vec::new(),
            manifest_seen: false,
            verdicts_sent: false,
            manifest: None,
        }
    }

    /// The phase this end believes it is in.
    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    /// The manifest this receiver accepted, once one has arrived.
    ///
    /// `None` until the `Manifest` message has been delivered. A caller that
    /// has to create files needs this: over a transport, the manifest is
    /// something the receiving end is told, not something it already had.
    #[must_use]
    pub const fn manifest(&self) -> Option<&TransferManifest> {
        self.manifest.as_ref()
    }

    /// Verdicts computed so far, in manifest order.
    #[must_use]
    pub fn verdicts(&self) -> Vec<(u32, ItemVerdict)> {
        self.items
            .iter()
            .map(|item| {
                (
                    item.item_id,
                    item.verdict.unwrap_or(ItemVerdict::Incomplete),
                )
            })
            .collect()
    }

    fn emit(
        &mut self,
        message_type: MessageType,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, TransferError> {
        let frame = Frame::new(message_type, payload).map_err(|_| TransferError::Framing)?;
        let sealed = self
            .sealer
            .seal(&frame)
            .map_err(|_| TransferError::Framing)?;
        Ok(sealed.encode())
    }

    fn poison(&mut self, error: TransferError) -> TransferError {
        if !self.phase.is_terminal() {
            self.phase = Phase::Poisoned;
        }
        error
    }

    /// Cancels from this end.
    ///
    /// # Errors
    ///
    /// Framing failures, or a session that already ended.
    pub fn request_cancel(&mut self) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        let bytes = self.emit(MessageType::Cancel, Control::USER.encode())?;
        self.phase = Phase::Cancelled;
        Ok(bytes)
    }

    /// Asks the peer to pause.
    ///
    /// # Errors
    ///
    /// Framing failures, or a terminal session.
    pub fn request_pause(&mut self) -> Result<Vec<u8>, TransferError> {
        if self.phase.is_terminal() {
            return Err(TransferError::SessionPoisoned);
        }
        self.phase = Phase::Paused;
        self.emit(MessageType::Pause, Control::USER.encode())
    }

    /// Feeds inbound bytes, writing verified content to `sink`.
    ///
    /// # Errors
    ///
    /// One variant per refusal in ADR-0026 §4.
    pub fn deliver(
        &mut self,
        bytes: &[u8],
        sink: &mut dyn ContentSink,
    ) -> Result<Vec<Vec<u8>>, TransferError> {
        if self.phase == Phase::Poisoned {
            return Err(TransferError::SessionPoisoned);
        }
        let mut out = Vec::new();
        // Same reason as the sender's: see ADR-0026 §4.
        let opened = open_all(&mut self.decoder, &mut self.opener, bytes)
            .map_err(|error| self.poison(error))?;
        for (message_type, payload) in opened {
            if let Some(reply) = self.apply(message_type, &payload, sink)? {
                out.push(reply);
            }
        }
        Ok(out)
    }

    fn apply(
        &mut self,
        message_type: MessageType,
        payload: &[u8],
        sink: &mut dyn ContentSink,
    ) -> Result<Option<Vec<u8>>, TransferError> {
        match (self.phase, message_type) {
            (Phase::Negotiating, MessageType::TransferOffer) => {
                let offer = Offer::decode(payload).map_err(|error| self.poison(error))?;
                self.window = self.window.min(offer.window_chunks);
                let accept = Accept {
                    window_chunks: self.window,
                };
                let bytes = self.emit(MessageType::TransferAccept, accept.encode())?;
                Ok(Some(bytes))
            }
            (Phase::Negotiating, MessageType::Manifest) => {
                let manifest =
                    qyro_manifest::codec::decode(payload).map_err(|_| TransferError::Framing)?;
                self.items = manifest
                    .items()
                    .iter()
                    .map(|item| RecvItem {
                        item_id: item.item_id(),
                        size: item.size(),
                        expected_chunks: chunk_count(item.size(), CHUNK_SIZE),
                        next_expected: 0,
                        hasher: Sha256::new(),
                        received: 0,
                        expected_digest: item.hash().digest().to_vec(),
                        verdict: None,
                    })
                    .collect();
                self.manifest_seen = true;
                self.manifest = Some(manifest);
                self.phase = Phase::Transferring;
                Ok(None)
            }
            (Phase::Transferring, MessageType::ItemStart) => {
                let start = ItemStart::decode(payload).map_err(|error| self.poison(error))?;
                let Some(item) = self.items.iter().find(|item| item.item_id == start.item_id)
                else {
                    return Err(self.poison(TransferError::UnknownItem {
                        item_id: start.item_id,
                    }));
                };
                if item.size != start.item_bytes {
                    let manifest = item.size;
                    return Err(self.poison(TransferError::ItemSizeMismatch {
                        item_id: start.item_id,
                        declared: start.item_bytes,
                        manifest,
                    }));
                }
                Ok(None)
            }
            (Phase::Transferring | Phase::Paused, MessageType::DataChunk) => {
                let chunk = wire::decode_chunk(payload).map_err(|error| self.poison(error))?;
                self.accept_chunk(chunk, sink).map(Some)
            }
            (Phase::Transferring | Phase::Paused, MessageType::Pause) => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Paused;
                Ok(None)
            }
            (Phase::Paused | Phase::Transferring, MessageType::Resume) => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Transferring;
                Ok(None)
            }
            (_, MessageType::Cancel) if !self.phase.is_terminal() => {
                Control::decode(payload, message_type).map_err(|error| self.poison(error))?;
                self.phase = Phase::Cancelled;
                Ok(None)
            }
            (Phase::Transferring, MessageType::Complete) => {
                Complete::decode(payload).map_err(|error| self.poison(error))?;
                let delivered = self
                    .items
                    .iter()
                    .filter(|item| item.received >= item.size)
                    .count();
                if delivered != self.items.len() {
                    let expected = self.items.len();
                    return Err(self.poison(TransferError::CompleteBeforeAllItems {
                        delivered,
                        expected,
                    }));
                }
                self.finish_items();
                let result = Integrity {
                    verdicts: self.verdicts(),
                };
                let bytes = self.emit(MessageType::IntegrityResult, result.encode())?;
                self.verdicts_sent = true;
                self.phase = Phase::Done;
                Ok(Some(bytes))
            }
            (_, got) => Err(self.poison(TransferError::UnexpectedMessage { got })),
        }
    }

    /// Applies one chunk and returns the acknowledgement to send back.
    fn accept_chunk(
        &mut self,
        chunk: ChunkRef<'_>,
        sink: &mut dyn ContentSink,
    ) -> Result<Vec<u8>, TransferError> {
        if chunk.content.len() > CHUNK_SIZE {
            let found = chunk.content.len();
            return Err(self.poison(TransferError::ChunkTooLarge {
                found,
                limit: CHUNK_SIZE,
            }));
        }
        let Some(index) = self
            .items
            .iter()
            .position(|item| item.item_id == chunk.item_id)
        else {
            return Err(self.poison(TransferError::UnknownItem {
                item_id: chunk.item_id,
            }));
        };

        let (ack_through, out_of_order) = {
            let Some(item) = self.items.get_mut(index) else {
                return Err(TransferError::UnknownItem {
                    item_id: chunk.item_id,
                });
            };
            if item.verdict.is_some() {
                let item_id = item.item_id;
                return Err(self.poison(TransferError::ItemAlreadyComplete { item_id }));
            }
            if chunk.chunk_index == item.next_expected {
                // Contiguous: hash it, hand it on, and extend the run.
                let offset = u64::from(chunk.chunk_index).saturating_mul(CHUNK_SIZE as u64);
                item.hasher.update(chunk.content);
                item.received = item.received.saturating_add(chunk.content.len() as u64);
                item.next_expected = item.next_expected.saturating_add(1);
                sink.write_at(chunk.item_id, offset, chunk.content);
                (item.next_expected.saturating_sub(1), false)
            } else {
                // Out of order. Dropped rather than buffered — buffering is one
                // more memory bound to govern and cumulative ACK does not need
                // it (ADR-0026 §4). The ACK below tells the sender where the
                // gap starts.
                (item.next_expected.saturating_sub(1), true)
            }
        };

        let _ = out_of_order;
        let ack = Ack {
            item_id: chunk.item_id,
            through_index: ack_through,
        };
        self.emit(MessageType::ChunkAck, ack.encode())
    }

    /// Turns every item's accumulated hash into a verdict.
    fn finish_items(&mut self) {
        for item in &mut self.items {
            if item.verdict.is_some() {
                continue;
            }
            if item.received != item.size {
                item.verdict = Some(ItemVerdict::SizeMismatch);
                continue;
            }
            if item.next_expected < item.expected_chunks {
                item.verdict = Some(ItemVerdict::Incomplete);
                continue;
            }
            let digest = item.hasher.clone().finalize();
            item.verdict = Some(if digest.as_slice() == item.expected_digest.as_slice() {
                ItemVerdict::Ok
            } else {
                ItemVerdict::DigestMismatch
            });
        }
    }
}

/// Decodes and opens every frame in `bytes`.
///
/// Returns `(message_type, payload)` pairs. A frame that does not authenticate
/// is [`TransferError::NotAuthenticated`] and carries no detail: a frame with no
/// verified sender cannot testify to anything.
/// Authenticated frames: each one's kind, and its payload still wearing its wipe.
type OpenedFrames = Vec<(MessageType, Zeroizing<Vec<u8>>)>;

/// The payloads come back **still wearing their wipe**.
///
/// The previous shape ended in `.to_vec()`, and the subtlety is worth stating
/// because the obvious reading is wrong: `.to_vec()` did not undo the
/// `Zeroizing`. The temporary was still wiped when the statement ended. What it
/// did was **copy the plaintext into a fresh allocation that nothing wipes**, so
/// the verified bytes ended up in two places and only one of them was cleared.
/// The net exposure was identical to the `into_payload` that
/// `into_zeroizing_payload` exists to replace -- and `qyro_crypto`'s egress
/// guard forbids `into_payload` by name while being blind to a `.to_vec()` on
/// its replacement (QYR-0304).
///
/// `Zeroizing<Vec<u8>>` derefs to `[u8]`, so every caller that already took
/// `&payload` keeps compiling and starts benefiting.
fn open_all(
    decoder: &mut FrameDecoder,
    opener: &mut FrameOpener,
    bytes: &[u8],
) -> Result<OpenedFrames, TransferError> {
    decoder.push(bytes).map_err(|_| TransferError::Framing)?;
    let mut out = Vec::new();
    while let Some(decoded) = decoder.next_frame().map_err(|_| TransferError::Framing)? {
        match decoded {
            DecodedFrame::Encrypted(envelope) => {
                let authenticated = opener
                    .open(&envelope)
                    .map_err(|_| TransferError::NotAuthenticated)?;
                let message_type = authenticated.message_type();
                let payload = authenticated.into_zeroizing_payload();
                out.push((message_type, payload));
            }
            DecodedFrame::Message(_) | DecodedFrame::Unsupported(_) => {
                return Err(TransferError::NotAuthenticated);
            }
        }
    }
    Ok(out)
}
