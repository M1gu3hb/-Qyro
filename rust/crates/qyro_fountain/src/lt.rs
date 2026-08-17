//! Luby Transform: frames that keep arriving until the file is whole.
//!
//! Specification: ADR-0044 §4, measured in `R8` §3.
//!
//! # The failure this exists to prevent
//!
//! BBQr splits a payload into N fixed pieces and, in its own words, *«there is
//! no way to skip one»*. Miss frame 3 of 40 on a screen that does not rewind and
//! the person starts over — and they will miss one, because a camera and a
//! screen are not synchronised and any frame caught mid-transition is rubbish.
//!
//! A fountain code has no piece numbers to miss. The sender emits **an endless
//! stream** of combinations; the receiver collects until it has enough, and
//! *which* ones it caught does not matter. A frame lost at 90 % costs one frame.
//!
//! # What it costs
//!
//! 5–15 % more frames than the theoretical minimum, against RaptorQ's 0.02 %.
//! On a channel that moves 8 KB/s that is seconds. RaptorQ's licence is forever
//! (ADR-0044 §4).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::rng::Rng;

/// The robust-soliton constant, ADR-0044 §4.
///
/// Luby's analysis leaves `c` free; 0.03 is the value the practical
/// implementations converge on for the block counts this channel produces
/// (tens to low hundreds). It shifts the spike toward smaller degrees, which is
/// what starts a decode: **a decode cannot begin without at least one degree-1
/// frame**, and a distribution tuned for asymptotic optimality produces those
/// too rarely at k = 40.
const SOLITON_C: f64 = 0.03;

/// The failure probability the distribution is tuned against.
const SOLITON_DELTA: f64 = 0.05;

/// How a source payload was cut up, and how to put it back.
///
/// Carried in **every** frame rather than sent once at the start: the receiver
/// may point the camera at a stream that is already running, and a header it
/// missed is a transfer it cannot join. Nine bytes per frame against a 1 465 B
/// capacity is a cost worth paying to make the stream joinable at any moment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shape {
    /// Bytes in the original payload.
    pub payload_len: u32,
    /// Bytes per source block.
    pub block_size: u16,
}

impl Shape {
    /// How many source blocks this shape implies.
    #[must_use]
    pub const fn blocks(&self) -> usize {
        if self.block_size == 0 {
            return 0;
        }
        let size = self.block_size as usize;
        let len = self.payload_len as usize;
        len.div_ceil(size)
    }
}

/// One frame: a seed, the shape, and the XOR of the blocks the seed chooses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub seed: u64,
    pub shape: Shape,
    pub payload: Vec<u8>,
}

/// Which source blocks a seed selects, for a payload of `blocks` blocks.
///
/// **Both ends run this.** The frame carries the seed, never the list — a list
/// of indices would cost more than the payload it describes. That makes this
/// function part of the wire format, along with [`Rng`].
#[must_use]
pub fn neighbours(seed: u64, blocks: usize) -> Vec<usize> {
    if blocks == 0 {
        return Vec::new();
    }
    let mut rng = Rng::seeded(seed);
    let degree = robust_soliton_degree(&mut rng, blocks);

    let mut chosen: Vec<usize> = Vec::with_capacity(degree);
    // Sampling without replacement by rejection. A block XORed twice cancels
    // itself out, so a duplicate does not merely waste space — it silently
    // changes which blocks the frame actually encodes, and the decoder would
    // combine it wrongly while believing it had understood.
    let mut attempts = 0;
    while chosen.len() < degree && attempts < degree * 64 {
        attempts += 1;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "below() returns a value under `blocks`, which came from a usize"
        )]
        let candidate = rng.below(blocks as u64) as usize;
        if !chosen.contains(&candidate) {
            chosen.push(candidate);
        }
    }
    chosen.sort_unstable();
    chosen
}

/// Draws a degree from the robust soliton distribution.
fn robust_soliton_degree(rng: &mut Rng, blocks: usize) -> usize {
    let weights = robust_soliton_weights(blocks);
    let total: f64 = weights.iter().sum();
    let mut target = rng.unit() * total;
    for (index, weight) in weights.iter().enumerate() {
        target -= *weight;
        if target <= 0.0 {
            return index + 1;
        }
    }
    // Floating-point arithmetic can leave `target` a hair above zero after the
    // last subtraction. Falling back to the highest degree is correct and, more
    // to the point, is not a panic: a rounding error must not kill a transfer.
    blocks.max(1)
}

/// The unnormalised robust-soliton weights for degrees `1..=blocks`.
fn robust_soliton_weights(blocks: usize) -> Vec<f64> {
    let count = blocks.max(1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "block counts here are tens to low hundreds"
    )]
    let k = count as f64;

    // The ideal soliton: 1/k at degree 1, then 1/(d(d-1)).
    let mut weights: Vec<f64> = (1..=count)
        .map(|degree| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "degree is bounded by the block count"
            )]
            let d = degree as f64;
            if degree == 1 {
                1.0 / k
            } else {
                1.0 / (d * (d - 1.0))
            }
        })
        .collect();

    // The robust part: a spike at k/spike, which is what makes a decode start
    // and finish rather than stall with a handful of blocks left.
    let spike_position = SOLITON_C * (k / SOLITON_DELTA).ln() * k.sqrt();
    let spike = if spike_position < 1.0 {
        count
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "spike_position is positive and far below usize::MAX here"
        )]
        let position = (k / spike_position) as usize;
        position.clamp(1, count)
    };

    for (index, weight) in weights.iter_mut().enumerate() {
        let degree = index + 1;
        #[expect(
            clippy::cast_precision_loss,
            reason = "degree is bounded by the block count"
        )]
        let d = degree as f64;
        if degree < spike {
            *weight += 1.0 / (d * k / k);
            *weight += (k / spike_position).max(1.0) / (d * k);
        } else if degree == spike {
            *weight += (k / spike_position).max(1.0) * (k / SOLITON_DELTA).ln() / k;
        }
    }
    weights
}

/// Cuts a payload into source blocks, zero-padding the last one.
///
/// The padding is why [`Shape::payload_len`] travels: without it the receiver
/// cannot tell a file that ends in zeroes from one that was padded, and it
/// would deliver a file that is *nearly* right — the worst kind, because the
/// hash fails and nothing says why.
#[must_use]
pub fn split(payload: &[u8], block_size: u16) -> Vec<Vec<u8>> {
    if block_size == 0 {
        return Vec::new();
    }
    let size = block_size as usize;
    payload
        .chunks(size)
        .map(|chunk| {
            let mut block = vec![0_u8; size];
            if let Some(slot) = block.get_mut(..chunk.len()) {
                slot.copy_from_slice(chunk);
            }
            block
        })
        .collect()
}

/// Produces the frame for `seed` from already-split source blocks.
#[must_use]
pub fn encode(blocks: &[Vec<u8>], shape: Shape, seed: u64) -> Frame {
    let mut payload = vec![0_u8; shape.block_size as usize];
    for index in neighbours(seed, blocks.len()) {
        if let Some(block) = blocks.get(index) {
            for (slot, byte) in payload.iter_mut().zip(block.iter()) {
                *slot ^= *byte;
            }
        }
    }
    Frame {
        seed,
        shape,
        payload,
    }
}

/// Collects frames until the payload can be rebuilt.
///
/// The peeling decoder: a frame that reduces to one unknown block **solves**
/// that block, which may in turn reduce others. That cascade is the whole
/// algorithm, and it is why a decode can sit at 95 % for many frames and then
/// finish in one.
#[derive(Debug)]
pub struct Decoder {
    shape: Shape,
    solved: Vec<Option<Vec<u8>>>,
    pending: Vec<(Vec<usize>, Vec<u8>)>,
    seen: Vec<u64>,
}

impl Decoder {
    /// Starts a decode for a known shape.
    #[must_use]
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            solved: vec![None; shape.blocks()],
            pending: Vec::new(),
            seen: Vec::new(),
        }
    }

    /// How many source blocks are known.
    #[must_use]
    pub fn solved_count(&self) -> usize {
        self.solved.iter().filter(|slot| slot.is_some()).count()
    }

    /// Whether the payload can be produced.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self.solved.is_empty() && self.solved.iter().all(Option::is_some)
    }

    /// Takes one frame in.
    ///
    /// Returns whether the frame told the decoder anything new. **A repeat is
    /// not an error** — on a loop of frames the receiver sees the same seeds
    /// again every cycle, and treating that as a fault would fail every transfer
    /// that needed more than one pass.
    pub fn accept(&mut self, frame: &Frame) -> bool {
        if frame.shape != self.shape || self.seen.contains(&frame.seed) {
            return false;
        }
        self.seen.push(frame.seed);

        let indices = neighbours(frame.seed, self.solved.len());
        if indices.is_empty() {
            return false;
        }
        self.pending.push((indices, frame.payload.clone()));
        self.peel();
        true
    }

    /// Reduces everything reducible, repeatedly, until nothing moves.
    fn peel(&mut self) {
        loop {
            // Substitute every known block out of every pending combination.
            for (indices, payload) in &mut self.pending {
                indices.retain(|index| {
                    let Some(Some(block)) = self.solved.get(*index) else {
                        return true;
                    };
                    for (slot, byte) in payload.iter_mut().zip(block.iter()) {
                        *slot ^= *byte;
                    }
                    false
                });
            }

            // Anything down to a single unknown solves it.
            let mut progressed = false;
            let mut freshly: Vec<(usize, Vec<u8>)> = Vec::new();
            for (indices, payload) in &self.pending {
                if let [only] = indices.as_slice()
                    && self.solved.get(*only).is_some_and(Option::is_none)
                    && !freshly.iter().any(|(index, _)| index == only)
                {
                    freshly.push((*only, payload.clone()));
                }
            }
            for (index, block) in freshly {
                if let Some(slot) = self.solved.get_mut(index) {
                    *slot = Some(block);
                    progressed = true;
                }
            }

            self.pending.retain(|(indices, _)| !indices.is_empty());
            if !progressed {
                return;
            }
        }
    }

    /// The payload, once every block is known.
    ///
    /// `None` while anything is missing — never a partial file. A decoder that
    /// handed back what it had would produce a file that is nearly right, which
    /// is the worst outcome available: the hash fails and nothing says why.
    #[must_use]
    pub fn finish(&self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }
        let mut payload = Vec::with_capacity(self.shape.payload_len as usize);
        for block in self.solved.iter().flatten() {
            payload.extend_from_slice(block);
        }
        payload.truncate(self.shape.payload_len as usize);
        Some(payload)
    }
}
