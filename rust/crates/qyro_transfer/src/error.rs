//! One variant per way a transfer can refuse.
//!
//! Every one of these is a typed refusal, not a recovery. ADR-0026 §4 lists the
//! situations and what the engine does with each; this enum is that table in
//! code.

// Same denials as every other crate that parses peer bytes. A panic here is a
// remote denial of service.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::fmt;

use qyro_protocol::MessageType;

/// Why a transfer session refused.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TransferError {
    /// A message body was shorter than its fixed fields.
    BodyTooShort { message: MessageType, found: usize },
    /// A message arrived that this state has no transition for.
    ///
    /// The state machine's whole point: ADR-0026 §4 says an unexpected message
    /// is refused by type rather than tolerated.
    UnexpectedMessage { got: MessageType },
    /// The receiver granted a larger window than the sender offered.
    ///
    /// Refused rather than clamped. Clamping silently is how two ends end up
    /// believing different things about one number (ADR-0026 §1).
    WindowGrantTooLarge { offered: u32, granted: u32 },
    /// A chunk or `ItemStart` named an item the manifest does not have.
    UnknownItem { item_id: u32 },
    /// `ItemStart` declared a size the manifest disagrees with.
    ItemSizeMismatch {
        item_id: u32,
        declared: u64,
        manifest: u64,
    },
    /// A chunk arrived for an item that is already closed.
    ItemAlreadyComplete { item_id: u32 },
    /// A chunk carried more content than the agreed chunk size.
    ChunkTooLarge { found: usize, limit: usize },
    /// The peer acknowledged a chunk that was never sent.
    ///
    /// Not a lagging receiver. A receiver that confirms what does not exist is
    /// a different program.
    AckAheadOfSender {
        item_id: u32,
        through: u32,
        sent: u32,
    },
    /// `Complete` arrived before every item had been delivered.
    CompleteBeforeAllItems { delivered: usize, expected: usize },
    /// The session already ended, or refused something earlier.
    ///
    /// Poisoned like `FrameSealer`: after an error nothing more is accepted.
    /// An engine that recovers from a state it did not understand is an engine
    /// still in a state it does not understand.
    SessionPoisoned,
    /// The transfer was cancelled by either end.
    Cancelled,
    /// The frame did not authenticate, or the framing itself was refused.
    ///
    /// Carries no detail on purpose: a frame that does not authenticate has no
    /// known sender, so nothing it contains can be reported as fact.
    NotAuthenticated,
    /// A frame could not be built or encoded.
    Framing,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BodyTooShort { message, found } => {
                write!(f, "{message:?} body is {found} bytes, too short")
            }
            Self::UnexpectedMessage { got } => {
                write!(f, "{got:?} is not expected in this state")
            }
            Self::WindowGrantTooLarge { offered, granted } => write!(
                f,
                "receiver granted a window of {granted} against an offer of {offered}"
            ),
            Self::UnknownItem { item_id } => write!(f, "no manifest item {item_id}"),
            Self::ItemSizeMismatch {
                item_id,
                declared,
                manifest,
            } => write!(
                f,
                "item {item_id} declared {declared} bytes, manifest says {manifest}"
            ),
            Self::ItemAlreadyComplete { item_id } => {
                write!(f, "item {item_id} is already complete")
            }
            Self::ChunkTooLarge { found, limit } => {
                write!(f, "chunk carries {found} bytes, limit is {limit}")
            }
            Self::AckAheadOfSender {
                item_id,
                through,
                sent,
            } => write!(
                f,
                "item {item_id} acknowledged through {through} but only {sent} were sent"
            ),
            Self::CompleteBeforeAllItems {
                delivered,
                expected,
            } => write!(
                f,
                "Complete arrived with {delivered} of {expected} items delivered"
            ),
            Self::SessionPoisoned => f.write_str("the session refused something earlier"),
            Self::Cancelled => f.write_str("the transfer was cancelled"),
            Self::NotAuthenticated => f.write_str("the frame did not authenticate"),
            Self::Framing => f.write_str("the frame could not be built"),
        }
    }
}

impl core::error::Error for TransferError {}

/// What the receiver concluded about one item.
///
/// One verdict per item and not one for the transfer: "something failed" is no
/// use to anyone who has to decide what to retry (ADR-0026 §1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ItemVerdict {
    /// Digest and size both matched the manifest.
    Ok = 0,
    /// The content hashed to something else.
    DigestMismatch = 1,
    /// The content was a different length than the manifest declared.
    SizeMismatch = 2,
    /// The item never finished arriving.
    Incomplete = 3,
}

impl ItemVerdict {
    /// Reads a verdict byte, refusing values this build does not define.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Ok),
            1 => Some(Self::DigestMismatch),
            2 => Some(Self::SizeMismatch),
            3 => Some(Self::Incomplete),
            _ => None,
        }
    }
}
