//! The random numbers both ends must agree on, exactly.
//!
//! Specification: ADR-0044 §4.
//!
//! # Why this is written out and not imported
//!
//! A fountain frame does **not** carry the list of source blocks it combines —
//! that list would cost more than the payload. It carries a **seed**, and both
//! ends regenerate the same list from it. So the generator is part of the wire
//! format: change it and every frame in flight becomes undecodable, on a channel
//! where the two ends are different builds of different ages.
//!
//! `rand` would be a dependency whose *version* is now protocol. `StdRng` is
//! explicitly not reproducible across releases, and `SmallRng` is documented as
//! free to change. Neither can be a wire format.
//!
//! So: xorshift64\*, written here, sixteen lines, frozen. Marsaglia 2003, public
//! domain, passes BigCrush in this form. It is **not** cryptographic and nothing
//! here wants it to be — an attacker who predicts which blocks a frame combines
//! learns which blocks a frame combines.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// xorshift64\*, frozen because it is part of the wire format.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Seeds the generator.
    ///
    /// Zero is remapped, because xorshift has one fixed point and it is zero:
    /// seeded with it the generator returns zero forever, and a frame with seed
    /// 0 would combine no blocks and look like a decoder bug. The constant is
    /// arbitrary and only has to be non-zero.
    #[must_use]
    pub const fn seeded(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    /// The next value in the sequence.
    pub const fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `0..bound`, without modulo bias.
    ///
    /// Rejection sampling rather than `% bound`. The bias is small for the
    /// bounds this code uses, and it is still wrong: it would make some source
    /// blocks systematically rarer, and a block that appears rarely is the one
    /// that keeps a decode from finishing.
    ///
    /// A `bound` of zero yields zero, because there is no value to choose and
    /// panicking here would take down a decoder over a malformed frame.
    pub const fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        let zone = u64::MAX - (u64::MAX % bound) - 1;
        loop {
            let candidate = self.next_u64();
            if candidate <= zone {
                return candidate % bound;
            }
        }
    }

    /// A value in `[0, 1)`, as `f64`.
    ///
    /// The top 53 bits, which is exactly the mantissa of an `f64`, so every
    /// value is representable and the spacing is uniform.
    pub const fn unit(&mut self) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "53 bits into an f64 mantissa is exact by construction"
        )]
        let value = (self.next_u64() >> 11) as f64;
        value / ((1_u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::Rng;

    #[test]
    fn the_sequence_is_frozen_because_it_is_a_wire_format() {
        // **These numbers are protocol.** A frame carries a seed and not the
        // block list, so both ends regenerate the list from it. If this test
        // fails, every frame in flight became undecodable and the change needs a
        // version bump, not a new expected value.
        let mut rng = Rng::seeded(1);
        let first: Vec<u64> = (0..4).map(|_| rng.next_u64()).collect();
        assert_eq!(
            first,
            vec![
                5_180_492_295_206_395_165,
                12_380_297_144_915_551_517,
                13_389_498_078_930_870_103,
                5_599_127_315_341_312_413,
            ],
            "the generator changed and it is part of the wire format"
        );
    }

    #[test]
    fn a_zero_seed_does_not_freeze_the_generator() {
        // xorshift's one fixed point is zero: seeded with it, it returns zero
        // forever. A frame with seed 0 would then combine no blocks and read as
        // a decoder bug rather than the generator bug it is.
        let mut rng = Rng::seeded(0);
        let values: Vec<u64> = (0..3).map(|_| rng.next_u64()).collect();
        assert!(values.iter().all(|value| *value != 0), "{values:?}");
    }

    #[test]
    fn below_stays_in_range_and_reaches_both_ends() {
        // In range is the easy half. The half that matters is that it *reaches*
        // both ends: a generator that never returned 0 or never returned
        // bound-1 would make one source block unreachable, and one unreachable
        // block is a decode that never finishes.
        let mut rng = Rng::seeded(7);
        let mut seen = [false; 5];
        for _ in 0..500 {
            let value = rng.below(5);
            assert!(value < 5);
            seen[value as usize] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "some value never came up");
    }

    #[test]
    fn below_zero_yields_zero_instead_of_dividing_by_it() {
        // A malformed frame must not take the decoder down.
        let mut rng = Rng::seeded(3);
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn unit_stays_inside_the_half_open_interval() {
        let mut rng = Rng::seeded(11);
        for _ in 0..1000 {
            let value = rng.unit();
            assert!((0.0..1.0).contains(&value), "{value}");
        }
    }
}
