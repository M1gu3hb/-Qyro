#![no_main]
//! Feeds arbitrary bytes through the incremental frame decoder.
//!
//! The decoder must never panic and must never accept a frame that violates the
//! bounds, however the input is split.

use libfuzzer_sys::fuzz_target;
use qyro_protocol::{DecodedFrame, FrameDecoder, HEADER_LEN, MAX_PAYLOAD_LEN};

fuzz_target!(|data: &[u8]| {
    // Use the first byte to choose a chunk size, so splitting is fuzzed too.
    let (chunk_size, body) = match data.split_first() {
        Some((first, rest)) => (usize::from(*first).max(1), rest),
        None => return,
    };

    let mut decoder = FrameDecoder::new();
    for chunk in body.chunks(chunk_size) {
        if decoder.push(chunk).is_err() {
            return;
        }
        loop {
            match decoder.next_frame() {
                Ok(Some(DecodedFrame::Message(frame))) => {
                    assert!(frame.payload().len() <= MAX_PAYLOAD_LEN);
                    assert_eq!(frame.payload().len(), frame.header().payload_len() as usize);
                    assert_eq!(frame.header().header_len() as usize, HEADER_LEN);
                    assert_eq!(frame.header().trailer_len(), 0);
                }
                // An unknown type is delimited, so the stream survives it. A
                // sealed frame keeps its ciphertext; `encrypted_envelope`
                // covers that shape.
                Ok(Some(DecodedFrame::Unsupported(_) | DecodedFrame::Encrypted(_))) => {}
                Ok(None) => break,
                Err(_) => return,
            }
        }
    }
});
