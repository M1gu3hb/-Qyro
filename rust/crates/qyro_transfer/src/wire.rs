//! Message bodies, byte for byte as ADR-0026 §1 freezes them.
//!
//! Big-endian, like the rest of QYRO/1. Every decoder here refuses a body that
//! is shorter than its fixed fields rather than reading past it, and none of
//! them index without checking.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use qyro_protocol::MessageType;

use crate::error::{ItemVerdict, TransferError};

/// Reads a big-endian `u32` at `offset`, or reports the body too short.
fn u32_at(body: &[u8], offset: usize, message: MessageType) -> Result<u32, TransferError> {
    let end = offset.checked_add(4).ok_or(TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    let slice = body.get(offset..end).ok_or(TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    let array: [u8; 4] = slice.try_into().map_err(|_| TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    Ok(u32::from_be_bytes(array))
}

/// Reads a big-endian `u64` at `offset`, or reports the body too short.
fn u64_at(body: &[u8], offset: usize, message: MessageType) -> Result<u64, TransferError> {
    let end = offset.checked_add(8).ok_or(TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    let slice = body.get(offset..end).ok_or(TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    let array: [u8; 8] = slice.try_into().map_err(|_| TransferError::BodyTooShort {
        message,
        found: body.len(),
    })?;
    Ok(u64::from_be_bytes(array))
}

/// `TransferOffer`: what the sender declares it is about to do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Offer {
    /// Number of manifest entries.
    pub item_count: u32,
    /// Sum of item sizes.
    pub total_bytes: u64,
    /// The chunk size the sender will use. Declared, not negotiated.
    pub chunk_size: u32,
    /// The window the sender promises to respect.
    pub window_chunks: u32,
}

impl Offer {
    /// Serialises the sixteen-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(20);
        out.extend_from_slice(&self.item_count.to_be_bytes());
        out.extend_from_slice(&self.total_bytes.to_be_bytes());
        out.extend_from_slice(&self.chunk_size.to_be_bytes());
        out.extend_from_slice(&self.window_chunks.to_be_bytes());
        out
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the fixed fields do not fit.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        let message = MessageType::TransferOffer;
        Ok(Self {
            item_count: u32_at(body, 0, message)?,
            total_bytes: u64_at(body, 4, message)?,
            chunk_size: u32_at(body, 12, message)?,
            window_chunks: u32_at(body, 16, message)?,
        })
    }
}

/// `TransferAccept`: the window the receiver grants.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Accept {
    /// May be smaller than the offer, never larger.
    pub window_chunks: u32,
}

impl Accept {
    /// Serialises the four-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.window_chunks.to_be_bytes().to_vec()
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the fixed field does not fit.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        Ok(Self {
            window_chunks: u32_at(body, 0, MessageType::TransferAccept)?,
        })
    }
}

/// `ItemStart`: which manifest entry is beginning, and how long it claims to be.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemStart {
    /// The manifest entry's id.
    pub item_id: u32,
    /// Size repeated from the manifest so the receiver can disagree early.
    pub item_bytes: u64,
}

impl ItemStart {
    /// Serialises the twelve-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.item_id.to_be_bytes());
        out.extend_from_slice(&self.item_bytes.to_be_bytes());
        out
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the fixed fields do not fit.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        let message = MessageType::ItemStart;
        Ok(Self {
            item_id: u32_at(body, 0, message)?,
            item_bytes: u64_at(body, 4, message)?,
        })
    }
}

/// Bytes of `DataChunk` before the content begins.
pub const CHUNK_HEADER_LEN: usize = 8;

/// `DataChunk`: content, with the item repeated in every one.
///
/// Borrowing rather than owning: the content is the hot path and copying it into
/// a struct to copy it out again is the kind of cost that only shows up on a
/// large transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkRef<'a> {
    /// Which item this content belongs to.
    pub item_id: u32,
    /// Zero-based index within the item.
    pub chunk_index: u32,
    /// The bytes.
    pub content: &'a [u8],
}

impl ChunkRef<'_> {
    /// Serialises header and content.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(CHUNK_HEADER_LEN + self.content.len());
        out.extend_from_slice(&self.item_id.to_be_bytes());
        out.extend_from_slice(&self.chunk_index.to_be_bytes());
        out.extend_from_slice(self.content);
        out
    }
}

/// Reads a chunk body, borrowing the content from `body`.
///
/// # Errors
///
/// [`TransferError::BodyTooShort`] when the eight-byte header does not fit.
pub fn decode_chunk(body: &[u8]) -> Result<ChunkRef<'_>, TransferError> {
    let message = MessageType::DataChunk;
    let item_id = u32_at(body, 0, message)?;
    let chunk_index = u32_at(body, 4, message)?;
    let content = body
        .get(CHUNK_HEADER_LEN..)
        .ok_or(TransferError::BodyTooShort {
            message,
            found: body.len(),
        })?;
    Ok(ChunkRef {
        item_id,
        chunk_index,
        content,
    })
}

/// `ChunkAck`: everything up to and including `through_index` has arrived.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ack {
    /// Which item.
    pub item_id: u32,
    /// Cumulative: through this index, inclusive.
    pub through_index: u32,
}

impl Ack {
    /// Serialises the eight-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8);
        out.extend_from_slice(&self.item_id.to_be_bytes());
        out.extend_from_slice(&self.through_index.to_be_bytes());
        out
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the fixed fields do not fit.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        let message = MessageType::ChunkAck;
        Ok(Self {
            item_id: u32_at(body, 0, message)?,
            through_index: u32_at(body, 4, message)?,
        })
    }
}

/// `Complete`: what the sender believes it sent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Complete {
    /// Total content bytes.
    pub total_bytes: u64,
}

impl Complete {
    /// Serialises the eight-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.total_bytes.to_be_bytes().to_vec()
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the fixed field does not fit.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        Ok(Self {
            total_bytes: u64_at(body, 0, MessageType::Complete)?,
        })
    }
}

/// `IntegrityResult`: one verdict per item, in manifest order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Integrity {
    /// `(item_id, verdict)` pairs.
    pub verdicts: Vec<(u32, ItemVerdict)>,
}

impl Integrity {
    /// Serialises count, ids and verdicts.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let count = u32::try_from(self.verdicts.len()).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(4 + self.verdicts.len() * 5);
        out.extend_from_slice(&count.to_be_bytes());
        for (item_id, _) in &self.verdicts {
            out.extend_from_slice(&item_id.to_be_bytes());
        }
        for (_, verdict) in &self.verdicts {
            out.push(*verdict as u8);
        }
        out
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the declared count does not fit the
    /// bytes present, or a verdict byte is one this build does not define.
    pub fn decode(body: &[u8]) -> Result<Self, TransferError> {
        let message = MessageType::IntegrityResult;
        let short = || TransferError::BodyTooShort {
            message,
            found: body.len(),
        };
        let count = usize::try_from(u32_at(body, 0, message)?).map_err(|_| short())?;

        let ids_end = count
            .checked_mul(4)
            .and_then(|n| n.checked_add(4))
            .ok_or_else(short)?;
        let verdicts_end = ids_end.checked_add(count).ok_or_else(short)?;
        if body.len() < verdicts_end {
            return Err(short());
        }

        let mut verdicts = Vec::with_capacity(count);
        for index in 0..count {
            let id_offset = index
                .checked_mul(4)
                .and_then(|n| n.checked_add(4))
                .ok_or_else(short)?;
            let item_id = u32_at(body, id_offset, message)?;
            let verdict_offset = ids_end.checked_add(index).ok_or_else(short)?;
            let byte = *body.get(verdict_offset).ok_or_else(short)?;
            let verdict = ItemVerdict::from_byte(byte).ok_or_else(short)?;
            verdicts.push((item_id, verdict));
        }
        Ok(Self { verdicts })
    }
}

/// Why a receiver refused a transfer.
///
/// QYR-0089. Travels as the one reason byte of [`Control`], so `TransferReject`
/// has the body every control message already has and the format does not grow.
///
/// **Every byte decodes.** An unknown value becomes [`Self::Unspecified`] rather
/// than an error: a peer sending a reason this build has not heard of is a peer
/// running a later version, and refusing to understand *why* it refused would
/// turn a clear "no" into a framing failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    /// The person said no.
    Declined,
    /// There is no room for what was offered.
    NoRoom,
    /// The manifest itself was refused — a name, a size, a count.
    UnacceptableManifest,
    /// A reason this build does not know, including a peer that sent none.
    Unspecified,
}

impl RejectReason {
    /// The byte that travels. Written out, so reordering the enum is not a
    /// format change.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        match self {
            Self::Declined => 0,
            Self::NoRoom => 1,
            Self::UnacceptableManifest => 2,
            Self::Unspecified => 255,
        }
    }

    /// Reads one. Total on purpose — see the type comment.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Declined,
            1 => Self::NoRoom,
            2 => Self::UnacceptableManifest,
            _ => Self::Unspecified,
        }
    }
}

/// Body of `Pause`, `Resume` and `Cancel`: one reason byte.
///
/// A control message with no body cannot be extended without changing its
/// length, and a length that changes is a format change.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Control {
    /// 0 is "the user asked".
    pub reason: u8,
}

impl Control {
    /// The body every control message carries when the user asked.
    pub const USER: Self = Self { reason: 0 };

    /// Serialises the one-byte body.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        vec![self.reason]
    }

    /// Reads the body.
    ///
    /// # Errors
    ///
    /// [`TransferError::BodyTooShort`] when the body is empty.
    pub fn decode(body: &[u8], message: MessageType) -> Result<Self, TransferError> {
        let reason = *body.first().ok_or(TransferError::BodyTooShort {
            message,
            found: body.len(),
        })?;
        Ok(Self { reason })
    }
}
