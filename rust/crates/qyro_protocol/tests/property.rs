//! Property tests for the QYRO/1 codec.
//!
//! These use a seeded generator defined here rather than `proptest`. That crate
//! was evaluated (see `TESTING.md`): its licence is fine, but it pulls 39
//! transitive packages into a workspace that otherwise has none, which widens
//! what `cargo audit` must watch for a dev-only tool. The trade accepted here is
//! losing automatic shrinking; in exchange every failure is reproducible from
//! the printed seed alone, with no regression file to keep in sync.

use qyro_protocol::{
    DecodedFrame, Flags, Frame, FrameDecoder, FrameError, HEADER_LEN, MAX_PAYLOAD_LEN, MessageType,
};

/// xorshift64*, deterministic and dependency-free.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut state = self.0;
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        self.0 = state;
        state.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        if bound == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % bound as u64).unwrap_or(0)
    }

    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| u8::try_from(self.next_u64() & 0xFF).unwrap_or(0))
            .collect()
    }
}

fn arbitrary_frame(rng: &mut Rng) -> Frame {
    let message_type = MessageType::ALL[rng.below(MessageType::ALL.len())];
    let payload_len = rng.below(512);
    let payload = rng.bytes(payload_len);
    // Only transport flags: ENCRYPTED and COMPRESSED are not publicly settable,
    // because a caller cannot make either assertion true. (ADR-0018)
    let flags = match rng.below(4) {
        0 => Flags::NONE,
        1 => Flags::END_OF_ITEM,
        2 => Flags::END_OF_TRANSFER,
        _ => Flags::END_OF_ITEM.union(Flags::END_OF_TRANSFER),
    };
    Frame::new(message_type, payload)
        .expect("generated payload stays within limits")
        .with_identifiers(rng.next_u64(), rng.next_u64(), 0, 0)
        .with_sequence(rng.next_u64())
        .with_flags(flags)
        .expect("transport flags only")
}

/// Unwraps a known message; generated frames always use known types.
fn expect_message(decoded: DecodedFrame) -> Frame {
    match decoded {
        DecodedFrame::Message(frame) => frame,
        DecodedFrame::Encrypted(_) => panic!("generated frames are never encrypted"),
        DecodedFrame::Unsupported(event) => {
            panic!(
                "generated frames use known types, got {}",
                event.message_type_value()
            )
        }
    }
}

#[test]
fn decoding_what_was_encoded_preserves_the_frame() {
    let mut rng = Rng::new(0x5159_524F_0001);
    for case in 0..2_000 {
        let frame = arbitrary_frame(&mut rng);
        let mut decoder = FrameDecoder::new();
        decoder.push(&frame.encode()).expect("within buffer");
        let decoded = expect_message(
            decoder
                .next_frame()
                .unwrap_or_else(|error| panic!("case {case} failed to decode: {error}"))
                .unwrap_or_else(|| panic!("case {case} produced no frame")),
        );
        assert_eq!(decoded, frame, "case {case} did not round-trip");
    }
}

#[test]
fn incremental_delivery_always_matches_whole_delivery() {
    let mut rng = Rng::new(0x5159_524F_0002);
    for case in 0..500 {
        let frames: Vec<Frame> = (0..1 + rng.below(4))
            .map(|_| arbitrary_frame(&mut rng))
            .collect();
        let mut stream = Vec::new();
        for frame in &frames {
            frame.encode_into(&mut stream);
        }

        let mut whole = FrameDecoder::new();
        whole.push(&stream).expect("within buffer");
        let mut expected = Vec::new();
        while let Some(frame) = whole.next_frame().expect("valid stream") {
            expected.push(expect_message(frame));
        }

        // Split the same stream at arbitrary boundaries.
        let mut incremental = FrameDecoder::new();
        let mut actual = Vec::new();
        let mut offset = 0;
        while offset < stream.len() {
            let take = 1 + rng.below(stream.len() - offset);
            incremental
                .push(&stream[offset..offset + take])
                .expect("within buffer");
            offset += take;
            while let Some(frame) = incremental.next_frame().expect("valid stream") {
                actual.push(expect_message(frame));
            }
        }

        assert_eq!(actual, expected, "case {case} diverged between deliveries");
        assert_eq!(actual, frames, "case {case} lost a frame");
    }
}

#[test]
fn arbitrary_bytes_never_panic_and_never_exceed_limits() {
    let mut rng = Rng::new(0x5159_524F_0003);
    for _ in 0..5_000 {
        let input_len = rng.below(4 * HEADER_LEN);
        let mut input = rng.bytes(input_len);

        // Half the cases start from a well-formed header so the generator
        // reaches deeper validation instead of dying on the magic every time.
        if rng.below(2) == 0 && input.len() >= HEADER_LEN {
            let template = Frame::new(MessageType::DataChunk, Vec::new())
                .expect("empty payload")
                .encode();
            input[..HEADER_LEN].copy_from_slice(&template);
            // Then corrupt one byte of it.
            let index = rng.below(HEADER_LEN);
            input[index] ^= u8::try_from(1 + rng.below(255)).unwrap_or(1);
        }

        let mut decoder = FrameDecoder::new();
        if decoder.push(&input).is_err() {
            continue;
        }
        // Needing more bytes or rejecting the input are both fine; the property
        // is only about what the decoder is willing to accept.
        if let Ok(Some(DecodedFrame::Message(frame))) = decoder.next_frame() {
            assert!(frame.payload().len() <= MAX_PAYLOAD_LEN);
            assert_eq!(frame.payload().len(), frame.header().payload_len() as usize);
            assert_eq!(frame.header().header_len() as usize, HEADER_LEN);
            assert_eq!(frame.header().trailer_len(), 0);
        }
    }
}

#[test]
fn a_hostile_length_is_never_honoured() {
    let mut rng = Rng::new(0x5159_524F_0004);
    for _ in 0..1_000 {
        let mut bytes = Frame::new(MessageType::DataChunk, Vec::new())
            .expect("empty payload")
            .encode();
        // Any declared payload length above the limit must be refused.
        let hostile = u32::try_from(MAX_PAYLOAD_LEN).unwrap_or(u32::MAX)
            + u32::try_from(1 + rng.below(1_000_000)).unwrap_or(1);
        bytes[12..16].copy_from_slice(&hostile.to_be_bytes());

        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes).expect("header fits");
        assert!(
            matches!(
                decoder.next_frame(),
                Err(FrameError::PayloadTooLarge { .. })
            ),
            "declared {hostile} must be refused"
        );
        assert!(decoder.buffer_capacity() <= qyro_protocol::MAX_BUFFER_LEN);
    }
}
