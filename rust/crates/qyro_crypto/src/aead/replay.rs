//! The fixed replay window.
//!
//! Specified by ADR-0022. The window answers one question — *has this sequence
//! already been accepted in this direction?* — and it answers it in two
//! separate steps on purpose.
//!
//! [`ReplayWindow::check`] decides whether a sequence *could* be accepted and
//! changes nothing. [`ReplayWindow::record`] commits it. The caller runs the
//! AEAD between the two, so a frame whose tag does not verify never reaches
//! `record`.
//!
//! That split is the whole security property. If the window were updated when
//! the sequence was read, anyone at all — no key required — could send
//! `sequence = u64::MAX - 1` with sixteen random bytes as a tag and leave the
//! session unable to accept anything again.

use super::error::AeadError;

/// Sequences the window covers behind the highest one accepted.
pub const REPLAY_WINDOW: usize = 1024;

/// `u64` words needed for one bit per covered sequence.
const WORDS: usize = REPLAY_WINDOW / 64;

/// Which sequences have already been accepted in one direction.
///
/// `pub` only so that `crate::aead` can re-export it under `--cfg fuzzing`; in
/// every ordinary build the re-export does not exist and this type is
/// unreachable from outside the crate. A `pub(crate)` item cannot be
/// re-exported at all, which is why the visibility sits here rather than there.
#[derive(Debug)]
pub struct ReplayWindow {
    /// The largest sequence accepted so far, if any.
    ///
    /// `None` before the first frame: sequence 0 is a legitimate first frame,
    /// so a sentinel of 0 would be indistinguishable from having accepted it.
    highest_seen: Option<u64>,
    /// Bit `n` marks `highest_seen - n` as accepted. Bit 0 is `highest_seen`.
    bitmap: [u64; WORDS],
}

impl ReplayWindow {
    /// A fresh window.
    ///
    /// `pub` under `--cfg fuzzing` only, where `crate::aead` re-exports the type
    /// so a target can drive `check` and `record` directly.
    #[cfg_attr(
        fuzzing,
        expect(clippy::missing_const_for_fn, reason = "shape is shared")
    )]
    pub const fn new() -> Self {
        Self {
            highest_seen: None,
            bitmap: [0; WORDS],
        }
    }

    /// Decides whether `sequence` may be accepted, **without recording it**.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::ReplayDetected`] when the sequence has already been
    /// accepted, or [`AeadError::SequenceTooOld`] when it fell behind the
    /// window and can no longer be told apart from a replay.
    pub fn check(&self, sequence: u64) -> Result<(), AeadError> {
        let Some(highest) = self.highest_seen else {
            return Ok(());
        };

        if sequence > highest {
            return Ok(());
        }

        let behind = highest - sequence;
        if behind >= REPLAY_WINDOW as u64 {
            return Err(AeadError::SequenceTooOld {
                sequence,
                window: REPLAY_WINDOW as u64,
            });
        }

        // `behind` is under 1024, so the cast is exact.
        let behind = behind as usize;
        if self.slot(behind)? == 0 {
            Ok(())
        } else {
            Err(AeadError::ReplayDetected { sequence })
        }
    }

    /// Reads the bit `offset` positions behind the highest sequence.
    ///
    /// Indexing directly would panic on an out-of-range offset, and the offset
    /// is derived from a sequence a peer chose. Every caller has already proved
    /// the offset is inside the window; this returns an error instead of
    /// trusting that proof, because the cost of being wrong is a remote crash.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::ReplayStateCorrupt`] when the offset is outside the
    /// window.
    fn slot(&self, offset: usize) -> Result<u64, AeadError> {
        let word = self
            .bitmap
            .get(offset / 64)
            .ok_or(AeadError::ReplayStateCorrupt)?;
        Ok(word & (1u64 << (offset % 64)))
    }

    /// Records `sequence` as accepted.
    ///
    /// Call only after the frame has authenticated. Recording a sequence the
    /// window would have rejected is a caller bug, and is reported rather than
    /// silently ignored.
    ///
    /// # Errors
    ///
    /// Returns whatever [`ReplayWindow::check`] would have returned.
    pub fn record(&mut self, sequence: u64) -> Result<(), AeadError> {
        self.check(sequence)?;

        match self.highest_seen {
            None => {
                self.highest_seen = Some(sequence);
                self.bitmap = [0; WORDS];
                self.set(0)?;
            }
            Some(highest) if sequence > highest => {
                let advance = sequence - highest;
                if advance >= REPLAY_WINDOW as u64 {
                    // The jump cleared the whole window. Zeroing matters: a
                    // shift that left stale bits in place would mark sequences
                    // as accepted that never arrived, and then reject them.
                    self.bitmap = [0; WORDS];
                } else {
                    self.shift(advance as usize)?;
                }
                self.highest_seen = Some(sequence);
                self.set(0)?;
            }
            Some(highest) => {
                // Inside the window and not yet seen: fill it in without moving
                // the top. Networks reorder; that is not an attack.
                self.set((highest - sequence) as usize)?;
            }
        }
        Ok(())
    }

    /// The highest sequence accepted so far.
    #[cfg(test)]
    pub(crate) const fn highest_seen(&self) -> Option<u64> {
        self.highest_seen
    }

    /// Marks the slot `offset` positions behind the highest sequence.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::ReplayStateCorrupt`] when the offset is outside the
    /// window. A `debug_assert` used to stand here, which is no check at all in
    /// a release build — and a release build is the only one a user runs.
    fn set(&mut self, offset: usize) -> Result<(), AeadError> {
        let word = self
            .bitmap
            .get_mut(offset / 64)
            .ok_or(AeadError::ReplayStateCorrupt)?;
        *word |= 1u64 << (offset % 64);
        Ok(())
    }

    /// Moves every recorded bit `by` positions further into the past.
    ///
    /// # Errors
    ///
    /// Returns [`AeadError::ReplayStateCorrupt`] when `by` is not inside the
    /// window, which would make the shift meaningless rather than merely wrong.
    fn shift(&mut self, by: usize) -> Result<(), AeadError> {
        if by >= REPLAY_WINDOW {
            return Err(AeadError::ReplayStateCorrupt);
        }
        let words = by / 64;
        let bits = by % 64;

        let mut shifted = [0u64; WORDS];
        for index in (0..WORDS).rev() {
            let Some(source) = index.checked_sub(words) else {
                continue;
            };
            let carried = self
                .bitmap
                .get(source)
                .ok_or(AeadError::ReplayStateCorrupt)?;
            let mut value = carried << bits;
            if bits > 0 && source > 0 {
                // The bits pushed out of the previous word arrive here.
                let previous = self
                    .bitmap
                    .get(source - 1)
                    .ok_or(AeadError::ReplayStateCorrupt)?;
                value |= previous >> (64 - bits);
            }
            let slot = shifted
                .get_mut(index)
                .ok_or(AeadError::ReplayStateCorrupt)?;
            *slot = value;
        }
        self.bitmap = shifted;
        Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn a_fresh_window_accepts_any_first_sequence() {
        for first in [0u64, 1, 1024, u64::MAX] {
            let window = ReplayWindow::new();
            assert!(
                window.check(first).is_ok(),
                "{first} is a valid first frame"
            );
        }
    }

    #[test]
    fn sequence_zero_is_a_real_frame_not_an_empty_window() {
        // `highest_seen` is an Option precisely so these two states differ. A
        // sentinel of 0 would make the first frame indistinguishable from
        // having already accepted frame 0.
        let mut window = ReplayWindow::new();
        assert_eq!(window.highest_seen(), None);
        window.record(0).expect("first frame");
        assert_eq!(window.highest_seen(), Some(0));
        assert_eq!(
            window.check(0),
            Err(AeadError::ReplayDetected { sequence: 0 })
        );
    }

    #[test]
    fn checking_never_mutates() {
        let mut window = ReplayWindow::new();
        window.record(10).expect("first");
        for _ in 0..5 {
            assert!(window.check(11).is_ok());
        }
        assert_eq!(window.highest_seen(), Some(10), "check left the top alone");
        window
            .record(11)
            .expect("still acceptable after five checks");
    }

    #[test]
    fn out_of_order_inside_the_window_is_accepted_once_each() {
        let mut window = ReplayWindow::new();
        for sequence in [7u64, 3, 5, 0, 6, 1, 2, 4] {
            window.record(sequence).unwrap_or_else(|error| {
                panic!("{sequence} arrived out of order, not twice: {error}")
            });
        }
        for sequence in 0..=7u64 {
            assert_eq!(
                window.check(sequence),
                Err(AeadError::ReplayDetected { sequence })
            );
        }
        assert_eq!(window.highest_seen(), Some(7));
    }

    #[test]
    fn the_edge_of_the_window_is_where_the_off_by_one_would_be() {
        let mut window = ReplayWindow::new();
        let top = REPLAY_WINDOW as u64 - 1;
        window.record(top).expect("first");

        // Exactly `REPLAY_WINDOW - 1` behind is the last acceptable one.
        assert!(window.check(0).is_ok(), "the oldest covered sequence");
        window.record(0).expect("accepted");

        // One further back is outside.
        let mut window = ReplayWindow::new();
        window.record(REPLAY_WINDOW as u64).expect("first");
        assert_eq!(
            window.check(0),
            Err(AeadError::SequenceTooOld {
                sequence: 0,
                window: REPLAY_WINDOW as u64
            })
        );
        assert!(window.check(1).is_ok(), "one inside the window");
    }

    #[test]
    fn a_jump_past_the_window_clears_every_stale_bit() {
        // The bug this exists to prevent: a shift that leaves old bits in place
        // marks sequences as accepted that never arrived, and the peer's real
        // frames then look like replays.
        let mut window = ReplayWindow::new();
        for sequence in 0..64u64 {
            window.record(sequence).expect("in order");
        }
        window
            .record(100_000)
            .expect("a large gap is packet loss, not an attack");

        for sequence in 100_000 - REPLAY_WINDOW as u64 + 1..100_000 {
            assert!(
                window.check(sequence).is_ok(),
                "{sequence} never arrived and must not be marked accepted"
            );
        }
        assert_eq!(
            window.check(100_000),
            Err(AeadError::ReplayDetected { sequence: 100_000 })
        );
    }

    #[test]
    fn shifting_carries_bits_across_word_boundaries() {
        // Multi-word shifts are where a hand-written bitmap goes wrong, so this
        // walks a single recorded bit across every word of the window.
        for advance in [1usize, 63, 64, 65, 127, 128, 512, 1023] {
            let mut window = ReplayWindow::new();
            window.record(0).expect("first");
            window
                .record(advance as u64)
                .expect("advancing inside the window");

            assert_eq!(
                window.check(0),
                Err(AeadError::ReplayDetected { sequence: 0 }),
                "after advancing {advance}, sequence 0 is still recorded"
            );
            assert_eq!(
                window.check(advance as u64),
                Err(AeadError::ReplayDetected {
                    sequence: advance as u64
                })
            );
            for between in 1..advance as u64 {
                assert!(
                    window.check(between).is_ok(),
                    "after advancing {advance}, {between} never arrived"
                );
            }
        }
    }

    #[test]
    fn advancing_from_a_nonzero_sequence_uses_the_delta() {
        let mut window = ReplayWindow::new();
        window.record(100).expect("first nonzero sequence");
        window.record(102).expect("advance by two");

        assert_eq!(window.highest_seen(), Some(102));
        assert_eq!(
            window.check(100),
            Err(AeadError::ReplayDetected { sequence: 100 }),
            "the old top moves back by the delta, not by a sum involving its absolute value"
        );
        assert!(window.check(101).is_ok(), "101 never arrived");
    }

    #[test]
    fn a_recorded_bit_crosses_a_bitmap_word_boundary() {
        let mut window = ReplayWindow::new();
        window.record(100).expect("first");
        window.record(37).expect("63 behind the top");
        window
            .record(102)
            .expect("push the older bit from offset 63 to 65");

        assert_eq!(
            window.check(37),
            Err(AeadError::ReplayDetected { sequence: 37 }),
            "the bit carried from one u64 word to the next must remain recorded"
        );
        assert!(
            window.check(38).is_ok(),
            "the neighbouring sequence never arrived"
        );
    }

    #[test]
    fn recording_the_same_sequence_twice_reports_it() {
        let mut window = ReplayWindow::new();
        window.record(5).expect("first");
        assert_eq!(
            window.record(5),
            Err(AeadError::ReplayDetected { sequence: 5 })
        );
    }

    #[test]
    fn the_window_survives_the_top_of_the_sequence_space() {
        let mut window = ReplayWindow::new();
        window.record(u64::MAX).expect("the last sequence");
        assert_eq!(
            window.check(u64::MAX),
            Err(AeadError::ReplayDetected { sequence: u64::MAX })
        );
        assert!(window.check(u64::MAX - 1).is_ok());
        assert_eq!(
            window.check(0),
            Err(AeadError::SequenceTooOld {
                sequence: 0,
                window: REPLAY_WINDOW as u64
            })
        );
    }
}
