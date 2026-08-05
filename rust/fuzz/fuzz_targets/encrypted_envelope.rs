#![no_main]
//! Feeds arbitrary bytes through the decoder and inspects encrypted envelopes.
//!
//! Narrower than `frame_decoder`, which fuzzes framing in general. This one
//! cares about the shape the AEAD consumes: whatever comes out as an
//! `EncryptedEnvelope` must re-encode to the bytes it was decoded from, because
//! the whole 48-byte header is the associated data and a header that does not
//! survive a round trip authenticates something other than what travelled.

use libfuzzer_sys::fuzz_target;
use qyro_protocol::{DecodedFrame, FrameDecoder, HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN};

fuzz_target!(|data: &[u8]| {
    let (chunk_size, body) = match data.split_first() {
        Some((first, rest)) => (usize::from(*first).max(1), rest),
        None => return,
    };

    let mut decoder = FrameDecoder::new();
    let mut consumed = 0usize;
    for chunk in body.chunks(chunk_size) {
        if decoder.push(chunk).is_err() {
            return;
        }
        consumed += chunk.len();
        loop {
            match decoder.next_frame() {
                Ok(Some(DecodedFrame::Encrypted(envelope))) => {
                    let header = envelope.header();
                    assert!(envelope.ciphertext().len() <= MAX_PAYLOAD_LEN);
                    assert_eq!(envelope.ciphertext().len(), header.payload_len() as usize);
                    assert_eq!(envelope.tag().len(), usize::from(header.trailer_len()));
                    assert!(envelope.tag().len() <= MAX_TRAILER_LEN);
                    assert!(!envelope.tag().is_empty(), "ENCRYPTED implies a trailer");
                    assert_eq!(header.header_len() as usize, HEADER_LEN);

                    // The associated data is the header on the wire, or it is
                    // not associated data at all.
                    let encoded = envelope.encode();
                    assert_eq!(&encoded[..HEADER_LEN], &envelope.associated_data()[..]);
                    assert!(encoded.len() <= consumed);
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => return,
            }
        }
    }
});
