//! Message types and frame flags.
//!
//! The numeric values below are part of the wire contract and are frozen by
//! tests. Changing one is a major version change.

use crate::error::FrameError;

/// Kind of QYRO/1 frame.
///
/// Discriminants are stable wire values. Zero is reserved and never valid, so a
/// zeroed buffer cannot decode as a legitimate message.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum MessageType {
    /// Opening greeting.
    Hello = 1,
    /// Advertised capabilities.
    Capabilities = 2,
    /// Pairing exchange.
    Pairing = 3,
    /// Offer to send a transfer.
    TransferOffer = 4,
    /// Receiver accepted the offer.
    TransferAccept = 5,
    /// Receiver refused the offer.
    TransferReject = 6,
    /// Transfer manifest.
    Manifest = 7,
    /// Start of one manifest item.
    ItemStart = 8,
    /// Content chunk.
    DataChunk = 9,
    /// Selective acknowledgement.
    ChunkAck = 10,
    /// Pause request.
    Pause = 11,
    /// Resume request.
    Resume = 12,
    /// Cancel request.
    Cancel = 13,
    /// Transfer finished.
    Complete = 14,
    /// Final integrity verdict.
    IntegrityResult = 15,
    /// Error report.
    Error = 16,
    /// Liveness probe.
    Heartbeat = 17,
}

impl MessageType {
    /// Every message type of QYRO/1, in wire-value order.
    pub const ALL: [Self; 17] = [
        Self::Hello,
        Self::Capabilities,
        Self::Pairing,
        Self::TransferOffer,
        Self::TransferAccept,
        Self::TransferReject,
        Self::Manifest,
        Self::ItemStart,
        Self::DataChunk,
        Self::ChunkAck,
        Self::Pause,
        Self::Resume,
        Self::Cancel,
        Self::Complete,
        Self::IntegrityResult,
        Self::Error,
        Self::Heartbeat,
    ];

    /// Returns the stable wire value.
    #[must_use]
    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    /// Resolves a wire value, or reports it as unknown.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::UnknownMessageType`] when the value is not part of
    /// this protocol version, including the reserved value zero.
    pub const fn from_wire(value: u8) -> Result<Self, FrameError> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Capabilities),
            3 => Ok(Self::Pairing),
            4 => Ok(Self::TransferOffer),
            5 => Ok(Self::TransferAccept),
            6 => Ok(Self::TransferReject),
            7 => Ok(Self::Manifest),
            8 => Ok(Self::ItemStart),
            9 => Ok(Self::DataChunk),
            10 => Ok(Self::ChunkAck),
            11 => Ok(Self::Pause),
            12 => Ok(Self::Resume),
            13 => Ok(Self::Cancel),
            14 => Ok(Self::Complete),
            15 => Ok(Self::IntegrityResult),
            16 => Ok(Self::Error),
            17 => Ok(Self::Heartbeat),
            other => Err(FrameError::UnknownMessageType { value: other }),
        }
    }
}

/// Frame flags.
///
/// Reserved bits must be zero. An unknown flag can change how the payload is
/// meant to be read, so it is rejected rather than ignored: two peers
/// disagreeing about the meaning of a payload is a security failure, not a
/// compatibility inconvenience.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Flags(u8);

impl Flags {
    /// No flags.
    pub const NONE: Self = Self(0);
    /// Last frame of the current item.
    pub const END_OF_ITEM: Self = Self(1 << 0);
    /// Last frame of the transfer.
    pub const END_OF_TRANSFER: Self = Self(1 << 1);
    /// Payload is protected by the content layer.
    pub const ENCRYPTED: Self = Self(1 << 2);
    /// Payload is compressed.
    pub const COMPRESSED: Self = Self(1 << 3);

    /// Bits defined by QYRO/1.0.
    pub const KNOWN_MASK: u8 = 0b0000_1111;
    /// Bits that must stay zero.
    pub const RESERVED_MASK: u8 = !Self::KNOWN_MASK;

    /// Returns the raw bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Parses flag bits.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::InvalidFlags`] when any reserved bit is set.
    pub const fn from_bits(bits: u8) -> Result<Self, FrameError> {
        if bits & Self::RESERVED_MASK != 0 {
            return Err(FrameError::InvalidFlags {
                bits,
                reserved_mask: Self::RESERVED_MASK,
            });
        }
        Ok(Self(bits))
    }

    /// Returns true when every bit of `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns the union of both flag sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
