//! Blocks, checksums, and asking again — the protocol a line with a return path
//! should use.
//!
//! Specification: ADR-0045 §4.
//!
//! # Why not the fountain code that already exists
//!
//! Because **serial has a return path and a screen does not.** That one
//! difference decides it: a fountain pays 5–15 % overhead on every transfer to
//! survive losses it cannot ask about, and on a one-metre null-modem cable there
//! usually are none. At 9–11 KB/s that overhead is minutes on a 10 MB file, paid
//! for nothing.
//!
//! And a product reason on top of the arithmetic: **a retry is observable and
//! overhead is not.** «4 blocks were re-sent» tells somebody their cable is bad.
//! «12 % more frames were needed» tells nobody anything.
//!
//! # The line format
//!
//! One line of text per block, because the receiver on the far end may be
//! fifteen lines of PowerShell reading with `ReadLine`.
//!
//! ```text
//! QS1 <index> <total> <crc32-hex> <base64>
//! ```
//!
//! and the receiver answers with one of
//!
//! ```text
//! OK <index>
//! NAK <index>
//! ```

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::base64;
use crate::crc::crc32;
use crate::error::SerialError;

/// The tag that starts every data line.
pub const LINE_PREFIX: &str = "QS1";

/// Payload bytes per block, before Base64.
///
/// **A multiple of three, and that is not a rounding preference.** The pasted
/// receiver appends each block's Base64 to one file and hands the whole thing to
/// `certutil -decode` in a single call. A block whose length is not a multiple
/// of three encodes with `=` padding, and concatenating padded chunks puts `=`
/// in the middle of the stream — which is not valid Base64.
///
/// This was 512 and **`certutil` refused the result**, measured, before any of
/// it shipped: `DecodeFile returned Invalid Data. 0x8007000d`. The transfer
/// would have reported success while the far machine held nothing. 510 encodes
/// to exactly 680 characters with no padding, so blocks concatenate cleanly.
///
/// The rest of the sizing is unchanged: 680 characters plus a short header sits
/// comfortably under the 1 024-byte buffer `SerialPort.ReadLine` uses by default
/// on the PowerShell side, and one retry costs under half a second at 9–11 KB/s.
pub const BLOCK_BYTES: usize = 510;

/// The invariant above, checked where it cannot be optimised away.
const _: () = assert!(
    BLOCK_BYTES % 3 == 0,
    "a block that is not a multiple of three encodes with padding, and      concatenated padded blocks are not valid Base64 -- `certutil -decode`      refuses the file and the far machine ends up with nothing"
);

/// How many times one block is offered before the transfer is given up on.
///
/// **Bounded on purpose.** An unbounded retry against an unplugged cable is a
/// hang, and a hang is the failure people cannot diagnose. Five is enough to
/// ride out a burst of noise and few enough that a dead line is reported in
/// seconds rather than never.
pub const MAX_ATTEMPTS: u32 = 5;

/// One block, ready to put on the wire.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub index: u32,
    pub total: u32,
    pub payload: Vec<u8>,
}

/// Splits a payload into the blocks the protocol sends.
#[must_use]
pub fn split(payload: &[u8]) -> Vec<Block> {
    let chunks: Vec<&[u8]> = payload.chunks(BLOCK_BYTES).collect();
    let total = u32::try_from(chunks.len()).unwrap_or(u32::MAX);
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| Block {
            index: u32::try_from(index).unwrap_or(u32::MAX),
            total,
            payload: chunk.to_vec(),
        })
        .collect()
}

/// Renders a block as the single line that goes on the wire.
#[must_use]
pub fn line_of(block: &Block) -> String {
    format!(
        "{LINE_PREFIX} {} {} {:08x} {}",
        block.index,
        block.total,
        crc32(&block.payload),
        base64::encode(&block.payload)
    )
}

/// Reads a line back into a block, checking the CRC.
///
/// # Errors
///
/// [`SerialError::NotALine`] for anything that is not this protocol — including
/// the noise a serial port emits when it is opened, which is the common case and
/// not a fault. [`SerialError::Corrupt`] when the CRC disagrees, which is the
/// whole point of sending one.
pub fn block_of(line: &str) -> Result<Block, SerialError> {
    let mut fields = line.trim().split(' ');
    if fields.next() != Some(LINE_PREFIX) {
        return Err(SerialError::NotALine);
    }
    let index: u32 = fields
        .next()
        .and_then(|field| field.parse().ok())
        .ok_or(SerialError::NotALine)?;
    let total: u32 = fields
        .next()
        .and_then(|field| field.parse().ok())
        .ok_or(SerialError::NotALine)?;
    let claimed = fields
        .next()
        .and_then(|field| u32::from_str_radix(field, 16).ok())
        .ok_or(SerialError::NotALine)?;
    let encoded = fields.next().ok_or(SerialError::NotALine)?;
    if fields.next().is_some() {
        // A sixth field means the line is not what this protocol writes. Refused
        // rather than ignored: a receiver that tolerated extra fields would
        // accept a line two senders interleaved on the same wire.
        return Err(SerialError::NotALine);
    }

    let payload = base64::decode(encoded).ok_or(SerialError::Corrupt { index })?;
    if crc32(&payload) != claimed {
        return Err(SerialError::Corrupt { index });
    }
    Ok(Block {
        index,
        total,
        payload,
    })
}

/// What the receiver says back about one block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reply {
    Accepted(u32),
    Rejected(u32),
}

impl Reply {
    /// The line the receiver writes.
    #[must_use]
    pub fn line(self) -> String {
        match self {
            Self::Accepted(index) => format!("OK {index}"),
            Self::Rejected(index) => format!("NAK {index}"),
        }
    }

    /// Reads a reply, treating anything else as silence.
    ///
    /// `None` rather than an error, because a serial line carries other
    /// people's noise and a sender that failed on the first unexpected byte
    /// would never get started.
    #[must_use]
    pub fn of(line: &str) -> Option<Self> {
        let mut fields = line.trim().split(' ');
        let verb = fields.next()?;
        let index: u32 = fields.next()?.parse().ok()?;
        match verb {
            "OK" => Some(Self::Accepted(index)),
            "NAK" => Some(Self::Rejected(index)),
            _ => None,
        }
    }
}

/// What a completed transfer cost, so somebody can be told.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Tally {
    pub blocks: u32,
    pub retries: u32,
}

/// Collects blocks until the file is whole.
#[derive(Debug)]
pub struct Receiver {
    blocks: Vec<Option<Vec<u8>>>,
    total: Option<u32>,
}

impl Default for Receiver {
    fn default() -> Self {
        Self::new()
    }
}

impl Receiver {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            blocks: Vec::new(),
            total: None,
        }
    }

    /// Takes one line and answers with the reply that should go back.
    ///
    /// A line that is not this protocol gets **no reply at all**, which is why
    /// this returns an `Option`: answering somebody else's noise with a `NAK`
    /// would put this protocol's traffic onto a wire it does not own.
    pub fn accept(&mut self, line: &str) -> Option<Reply> {
        match block_of(line) {
            Ok(block) => {
                let total = block.total as usize;
                if self.blocks.len() < total {
                    self.blocks.resize(total, None);
                }
                self.total = Some(block.total);
                if let Some(slot) = self.blocks.get_mut(block.index as usize) {
                    *slot = Some(block.payload);
                }
                Some(Reply::Accepted(block.index))
            }
            Err(SerialError::Corrupt { index }) => Some(Reply::Rejected(index)),
            Err(_) => None,
        }
    }

    /// How many blocks are still missing.
    #[must_use]
    pub fn missing(&self) -> usize {
        match self.total {
            None => usize::MAX,
            Some(_) => self.blocks.iter().filter(|slot| slot.is_none()).count(),
        }
    }

    /// Whether every block has arrived.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.total.is_some() && !self.blocks.is_empty() && self.blocks.iter().all(Option::is_some)
    }

    /// The file, once it is whole — and never a piece of it.
    #[must_use]
    pub fn finish(&self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }
        let mut out = Vec::new();
        for block in self.blocks.iter().flatten() {
            out.extend_from_slice(block);
        }
        Some(out)
    }
}

/// Sends every block, re-offering the ones that come back rejected.
///
/// `wire` writes a line and returns whatever the far end said, or `None` if it
/// said nothing before the read window closed. Taking a closure rather than a
/// port is what lets the whole protocol be exercised over a pipe, which is the
/// only honest test available without a cable (ADR-0045 §8).
///
/// # Errors
///
/// [`SerialError::GaveUp`] when one block has been offered [`MAX_ATTEMPTS`]
/// times, which is the bounded end of the loop. [`SerialError::Wire`] when the
/// line itself failed.
pub fn send_all<W>(payload: &[u8], mut wire: W) -> Result<Tally, SerialError>
where
    W: FnMut(&str) -> Result<Option<Reply>, SerialError>,
{
    let blocks = split(payload);
    let mut tally = Tally {
        blocks: u32::try_from(blocks.len()).unwrap_or(u32::MAX),
        retries: 0,
    };

    for block in &blocks {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > MAX_ATTEMPTS {
                return Err(SerialError::GaveUp {
                    index: block.index,
                    attempts: MAX_ATTEMPTS,
                });
            }
            if attempts > 1 {
                tally.retries += 1;
            }

            match wire(&line_of(block))? {
                Some(Reply::Accepted(index)) if index == block.index => break,
                // A reply about a different block, or silence, is another
                // attempt rather than a failure: on a shared or noisy wire both
                // happen, and giving up on the first one would fail transfers
                // that were about to work.
                Some(_) | None => {}
            }
        }
    }
    Ok(tally)
}

/// Reads whatever the wire offers until the file is whole or the budget runs out.
///
/// # Errors
///
/// [`SerialError::Wire`] if the line failed, and [`SerialError::GaveUp`] if the
/// budget of lines was spent without the file completing — **bounded**, because
/// a receiver that waited forever on an unplugged cable is a hang.
pub fn receive_all<R>(mut read_line: R, line_budget: u32) -> Result<Vec<u8>, SerialError>
where
    R: FnMut(Option<Reply>) -> Result<Option<String>, SerialError>,
{
    let mut receiver = Receiver::new();
    let mut reply: Option<Reply> = None;
    for _ in 0..line_budget {
        let Some(line) = read_line(reply)? else {
            reply = None;
            continue;
        };
        reply = receiver.accept(&line);
        if receiver.is_complete() {
            // The last acknowledgement still has to go out, or the sender
            // re-offers a block the receiver already has and the transfer ends
            // in a retry storm rather than a full stop.
            let _ = read_line(reply)?;
            return receiver.finish().ok_or(SerialError::Wire);
        }
    }
    Err(SerialError::GaveUp {
        index: u32::try_from(receiver.missing().min(u32::MAX as usize)).unwrap_or(u32::MAX),
        attempts: line_budget,
    })
}
