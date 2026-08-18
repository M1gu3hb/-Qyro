//! What can go wrong on a wire, named.
//!
//! In its own module because the workspace's variant guard counts a variant as
//! real only when something **other than its own declaration** constructs it,
//! and it is right to: a variant written and never produced reads in a `match`
//! arm as if the case were handled.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Why a line, or a transfer, did not work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerialError {
    /// Not this protocol at all.
    ///
    /// **Not a fault.** A serial port emits noise when it is opened, another
    /// program may be talking on the same wire, and a receiver that treated the
    /// first unexpected byte as an error would never start.
    NotALine,
    /// The CRC disagreed, so the block is asked for again.
    Corrupt { index: u32 },
    /// A block was offered `MAX_ATTEMPTS` times and never landed.
    ///
    /// The bounded end of the retry loop. An unbounded retry against an
    /// unplugged cable is a hang, and a hang is the failure nobody can diagnose.
    GaveUp { index: u32, attempts: u32 },
    /// The wire itself failed.
    Wire,
}

impl core::fmt::Display for SerialError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotALine => write!(formatter, "that was not a Qyro serial line"),
            Self::Corrupt { index } => {
                write!(formatter, "block {index} arrived corrupt")
            }
            Self::GaveUp { index, attempts } => write!(
                formatter,
                "block {index} was sent {attempts} times and never arrived intact -- \
                 check the cable, the speed, and that both ends agree on flow control"
            ),
            Self::Wire => write!(formatter, "the serial port stopped answering"),
        }
    }
}
