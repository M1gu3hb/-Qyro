//! CRC-32, the one every other tool on the old machine already speaks.
//!
//! Specification: ADR-0045 §5.
//!
//! # Why this and not a hash
//!
//! A CRC is not a hash and is not trying to be. It catches **the errors a wire
//! makes** — single bits, short bursts, a byte the UART dropped — and it catches
//! them cheaply enough that a fifteen-line PowerShell receiver can compute one
//! per block. It does not catch somebody substituting a block on purpose, and
//! nothing in this file pretends otherwise: that is what the whole-file SHA-256
//! is for, with the limits QYR-0359 wrote down.
//!
//! # The polynomial, and why it is spelled out
//!
//! `0xEDB88320` is CRC-32/ISO-HDLC reflected — the one zlib, PNG, gzip and
//! `System.IO.Hashing` use. It is written here rather than pulled in because the
//! receiver on the other end is **a script somebody pastes**, and a script can
//! only use a CRC its own platform already has. Choosing an exotic polynomial
//! would mean the dumb receiver could not check anything at all.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// CRC-32/ISO-HDLC, reflected, as zlib and PowerShell compute it.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
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

    use super::crc32;

    #[test]
    fn the_check_value_every_crc32_table_publishes() {
        // `"123456789"` is the standard check vector, and its answer for
        // CRC-32/ISO-HDLC is 0xCBF43926. **This is the interoperability
        // assertion**: the receiver is a script using the platform's own CRC,
        // and if this number were different the two ends would disagree on
        // every block while both believing they were right.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn the_empty_input_is_zero_and_not_an_accident() {
        // An empty block has a defined CRC and it is 0. Worth pinning: a
        // receiver that returned 0 for "I could not compute one" would match
        // every empty block and nothing else, which fails in the confusing
        // direction.
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn one_flipped_bit_changes_it() {
        // The whole job. A CRC that missed single-bit errors would be
        // decoration, and this channel exists precisely because the wire makes
        // them.
        let clean = b"the quick brown fox";
        let mut dirty = *clean;
        dirty[4] ^= 0x01;
        assert_ne!(crc32(clean), crc32(&dirty));
    }

    #[test]
    fn and_swapping_two_bytes_changes_it_too() {
        // The control for the test above. A checksum that merely added the
        // bytes up would pass that one and miss this, and transposition is
        // exactly what a UART with a timing problem produces.
        assert_ne!(crc32(b"ab"), crc32(b"ba"));
    }
}
