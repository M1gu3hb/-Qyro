//! Base64, because a three-wire cable cannot carry binary.
//!
//! Specification: ADR-0045 §5.
//!
//! # The failure this prevents, and it is silent
//!
//! A cable without RTS/CTS has to use XON/XOFF for flow control, and XON/XOFF
//! **is** bytes `0x11` and `0x13` — the wire eats them. Raw binary containing
//! either one arrives corrupted with nothing to report it, and the corruption
//! looks like a bad cable rather than a protocol that cannot work.
//!
//! It costs +33 %, and on 9–11 KB/s that is real. It is accepted: **a channel
//! that runs at 75 % of its speed is worth infinitely more than one that
//! corrupts.**
//!
//! # Why written here
//!
//! The alphabet has to be the one `certutil -decode` accepts, because that is
//! what reassembles the file on a machine where nothing can be installed. That
//! makes the encoding a wire format, and this project keeps its wire formats
//! where it can see them.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// RFC 4648 §4, the alphabet `certutil` reads.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encodes with padding, as `certutil -decode` expects.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let (a, b, c) = (
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        );
        let triple = (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c);
        for shift in [18, 12, 6, 0] {
            let index = ((triple >> shift) & 0x3F) as usize;
            out.push(char::from(ALPHABET.get(index).copied().unwrap_or(b'A')));
        }
        // Padding is not decoration: `certutil` refuses input whose length is
        // not a multiple of four.
        let padding = 3 - chunk.len();
        let kept = out.len() - padding;
        out.truncate(kept);
        for _ in 0..padding {
            out.push('=');
        }
    }
    out
}

/// Decodes, refusing anything that is not exactly this alphabet.
///
/// # Errors
///
/// `None` for a character outside the alphabet, or a length that is not a
/// multiple of four. **Refused rather than skipped:** a decoder that ignored
/// stray characters would silently accept a line the wire mangled and produce a
/// file that is nearly right.
#[must_use]
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() % 4 != 0 {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for quad in bytes.chunks(4) {
        let mut value = 0_u32;
        let mut padding = 0;
        for (position, byte) in quad.iter().enumerate() {
            let six = if *byte == b'=' {
                // Padding is only legal in the last two positions of the last
                // quad. Anywhere else it is a mangled line.
                if position < 2 {
                    return None;
                }
                padding += 1;
                0
            } else {
                if padding > 0 {
                    return None;
                }
                let index = ALPHABET.iter().position(|candidate| candidate == byte)?;
                u32::try_from(index).ok()?
            };
            value = (value << 6) | six;
        }
        let triple = value.to_be_bytes();
        // `to_be_bytes` on a u32 gives four bytes and the first is always zero
        // here, because only 24 bits were filled.
        for byte in triple.iter().skip(1).take(3 - padding) {
            out.push(*byte);
        }
    }
    Some(out)
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

    use super::{decode, encode};

    #[test]
    fn the_vectors_rfc_4648_publishes() {
        // Not invented. These are the RFC's own, and they are here because the
        // decoder on the other end is `certutil`, which has never heard of this
        // project and will not negotiate.
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn and_they_come_back() {
        for vector in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            let text = encode(vector);
            if vector.is_empty() {
                assert_eq!(decode(&text), None, "an empty string decodes to nothing");
                continue;
            }
            assert_eq!(decode(&text).as_deref(), Some(vector));
        }
    }

    #[test]
    fn every_byte_value_survives_including_the_two_that_xon_xoff_eats() {
        // **The reason this module exists.** 0x11 and 0x13 are XON and XOFF: on
        // a three-wire cable the flow control eats them out of the stream and
        // the corruption is silent. Encoded, they are ordinary letters.
        let all: Vec<u8> = (0..=255).collect();
        let text = encode(&all);
        assert!(!text.contains('\u{11}') && !text.contains('\u{13}'));
        assert_eq!(decode(&text).as_deref(), Some(all.as_slice()));
    }

    #[test]
    fn a_mangled_line_is_refused_and_not_partly_decoded() {
        // A decoder that skipped stray characters would silently accept a line
        // the wire chewed and hand back a file that is *nearly* right, which is
        // the worst outcome available.
        assert_eq!(decode("Zm9v!"), None, "a stray character was accepted");
        assert_eq!(decode("Zm9"), None, "a short quad was accepted");
        assert_eq!(decode("Z=9v"), None, "padding in the middle was accepted");
        assert_eq!(
            decode("Zg=v"),
            None,
            "a character after padding was accepted"
        );
        assert_eq!(decode("Zm9v Zm9v"), None, "a space was accepted");
    }
}
