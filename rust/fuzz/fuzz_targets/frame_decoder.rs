#![no_main]
//! Feeds arbitrary bytes through the incremental frame decoder.
//!
//! The decoder must never panic and must never accept a frame that violates the
//! bounds, however the input is split.

use libfuzzer_sys::fuzz_target;
use qyro_protocol::{FrameDecoder, HEADER_LEN, MAX_PAYLOAD_LEN};

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
                Ok(Some(frame)) => {
                    assert!(frame.payload().len() <= MAX_PAYLOAD_LEN);
                    assert_eq!(frame.payload().len(), frame.header().payload_len as usize);
                    assert!(frame.header().header_len as usize >= HEADER_LEN);
                    assert_eq!(frame.header().trailer_len, 0);
                }
                Ok(None) => break,
                Err(_) => return,
            }
        }
    }
});
