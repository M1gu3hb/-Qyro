//! Contracts for the forward-compatibility and state promises QYRO/1 makes.
//!
//! Each test here corresponds to a contradiction between what ADR-0016 claimed
//! and what the code did. They are written as the behaviour the protocol must
//! have, not as the behaviour it had.

use qyro_protocol::{
    Flags, Frame, FrameDecoder, FrameError, FrameHeader, HEADER_LEN, MAX_HEADER_LEN, MessageType,
};

fn encoded(message_type: MessageType, payload: &[u8]) -> Vec<u8> {
    Frame::new(message_type, payload.to_vec())
        .expect("payload within limits")
        .encode()
}

// ------------------------------------------------ semantic vs structural

#[test]
fn an_unknown_message_type_does_not_desynchronise_the_stream() {
    // ADR-0016 promised this is recoverable: lengths are validated before the
    // type is resolved, so the receiver knows the frame's exact size.
    let mut unknown = encoded(MessageType::Hello, b"opaque payload");
    unknown[6] = 200; // a type this version does not know
    let following = encoded(MessageType::Heartbeat, b"still here");

    let mut stream = unknown.clone();
    stream.extend_from_slice(&following);

    let mut decoder = FrameDecoder::new();
    decoder.push(&stream).expect("within buffer");

    let first = decoder.next_frame().expect("must not be a framing failure");
    assert!(
        matches!(first, Some(qyro_protocol::DecodedFrame::Unsupported(ref event))
            if event.message_type_value() == 200),
        "an unknown type must surface as a delimited event, got {first:?}"
    );

    // The whole unsupported frame must have been consumed, so the next frame
    // still decodes. This is the property that was broken.
    let second = decoder
        .next_frame()
        .expect("stream stays usable")
        .expect("the following frame is still there");
    assert_eq!(second.message_type(), Some(MessageType::Heartbeat));
    assert_eq!(second.plaintext().expect("plain"), b"still here");
    assert!(!decoder.is_poisoned());
}

#[test]
fn an_unsupported_event_reports_the_frame_it_consumed() {
    let mut unknown = encoded(MessageType::DataChunk, b"1234567890");
    unknown[6] = 199;

    let mut decoder = FrameDecoder::new();
    decoder.push(&unknown).expect("within buffer");

    let Some(qyro_protocol::DecodedFrame::Unsupported(event)) =
        decoder.next_frame().expect("not a framing failure")
    else {
        panic!("expected an unsupported event");
    };

    // The caller needs enough to answer Error without re-parsing bytes.
    assert_eq!(event.message_type_value(), 199);
    assert_eq!(event.payload_len(), 10);
    assert_eq!(event.total_len(), HEADER_LEN + 10);
    assert_eq!(decoder.buffered_len(), 0, "the frame must be consumed");
}

#[test]
fn structural_failures_still_poison_the_stream() {
    // Once framing itself is untrustworthy there is no safe way to resynchronise.
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("bad magic", {
            let mut bytes = encoded(MessageType::Hello, b"");
            bytes[0] = b'Z';
            bytes
        }),
        ("bad major", {
            let mut bytes = encoded(MessageType::Hello, b"");
            bytes[4] = 9;
            bytes
        }),
        ("oversize payload", {
            let mut bytes = encoded(MessageType::DataChunk, b"");
            bytes[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
            bytes
        }),
        ("reserved flag", {
            let mut bytes = encoded(MessageType::Hello, b"");
            bytes[7] = 0b1000_0000;
            bytes
        }),
        ("non-zero reserved byte", {
            let mut bytes = encoded(MessageType::Hello, b"");
            bytes[11] = 1;
            bytes
        }),
        ("trailer without sealing", {
            let mut bytes = encoded(MessageType::Hello, b"");
            bytes[10] = 16;
            bytes
        }),
    ];

    for (label, bytes) in cases {
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes).expect("within buffer");
        assert!(
            decoder.next_frame().is_err(),
            "{label} must be a framing failure"
        );
        assert!(decoder.is_poisoned(), "{label} must poison the stream");
        assert!(
            decoder.next_frame().is_err(),
            "{label} must stay poisoned until reset"
        );
        decoder.reset();
        assert!(!decoder.is_poisoned(), "{label} must recover after reset");
    }
}

#[test]
fn recovery_from_a_structural_failure_requires_an_explicit_reset() {
    let mut bytes = encoded(MessageType::Hello, b"");
    bytes[0] = b'Z';

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    let error = decoder.next_frame().expect_err("magic is wrong");

    // Pushing more bytes must not silently clear the failure.
    assert_eq!(decoder.push(b"more").expect_err("still poisoned"), error);
    assert_eq!(decoder.next_frame().expect_err("still poisoned"), error);

    decoder.reset();
    decoder
        .push(&encoded(MessageType::Hello, b"ok"))
        .expect("usable again");
    let frame = decoder.next_frame().expect("ok").expect("frame");
    assert_eq!(frame.plaintext(), Some(&b"ok"[..]));
}

// ------------------------------------------------------- header extensions

#[test]
fn qyro1_rejects_a_header_extension_it_cannot_preserve() {
    // Accepting extension bytes while dropping them made decode->encode lossy
    // and would leave a future AEAD unable to authenticate them.
    let mut bytes = FrameHeader::new(MessageType::Capabilities, 3)
        .expect("within limits")
        .with_header_len(u16::try_from(HEADER_LEN + 8).expect("fits"))
        .expect_err("1.0 must refuse to build an extended header")
        .to_string()
        .into_bytes();
    bytes.clear();

    // Built by hand, the way a newer peer would send it.
    let mut raw = FrameHeader::new(MessageType::Capabilities, 3)
        .expect("within limits")
        .encode()
        .to_vec();
    raw[8..10].copy_from_slice(&u16::try_from(HEADER_LEN + 8).expect("fits").to_be_bytes());
    raw.extend_from_slice(&[0xDE; 8]);
    raw.extend_from_slice(b"abc");

    let mut decoder = FrameDecoder::new();
    decoder.push(&raw).expect("within buffer");
    assert!(
        matches!(
            decoder.next_frame(),
            Err(FrameError::UnsupportedHeaderExtension { declared, .. }) if declared as usize == HEADER_LEN + 8
        ),
        "an unpreservable extension must be refused, not silently skipped"
    );
}

#[test]
fn header_length_stays_exactly_the_fixed_size() {
    let header = FrameHeader::new(MessageType::Hello, 0).expect("within limits");
    assert_eq!(header.header_len() as usize, HEADER_LEN);
    // MAX_HEADER_LEN stays as a validation ceiling so an absurd declared length
    // is distinguishable from a merely extended one.
    assert_eq!(MAX_HEADER_LEN, 1024);
}

#[test]
fn decode_then_encode_is_byte_exact_for_every_accepted_frame() {
    // The property that unpreserved extensions violated.
    let cases: Vec<Vec<u8>> = vec![
        encoded(MessageType::Hello, b""),
        encoded(MessageType::DataChunk, &[0xAB; 512]),
        encoded(MessageType::Heartbeat, b"x"),
        Frame::new(MessageType::ItemStart, b"payload".to_vec())
            .expect("valid")
            .with_identifiers(u64::MAX, 7, u32::MAX, 9)
            .with_sequence(u64::MAX)
            .encode(),
    ];

    for bytes in cases {
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes).expect("within buffer");
        let frame = decoder.next_frame().expect("ok").expect("frame");
        assert_eq!(
            frame.try_encode().expect("retained frame"),
            bytes,
            "re-encoding must reproduce the exact bytes"
        );
    }
}

// ------------------------------------------------------------ impossible flags

#[test]
fn a_plain_frame_cannot_claim_to_be_encrypted() {
    // ENCRYPTED must only ever be set by the sealing path in qyro_crypto, which
    // also sets the tag. A frame that claims it without one is a lie.
    let mut bytes = encoded(MessageType::DataChunk, b"plaintext");
    bytes[7] = Flags::ENCRYPTED.bits();

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    assert!(
        matches!(
            decoder.next_frame(),
            Err(FrameError::EncryptedWithoutTrailer { .. })
        ),
        "ENCRYPTED without an authentication trailer must be refused"
    );
}

#[test]
fn compression_cannot_be_declared_before_compression_exists() {
    let mut bytes = encoded(MessageType::DataChunk, b"data");
    bytes[7] = Flags::COMPRESSED.bits();

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    assert!(
        matches!(
            decoder.next_frame(),
            Err(FrameError::UnsupportedFlag { .. })
        ),
        "COMPRESSED must be refused until compression is implemented"
    );
}

#[test]
fn no_public_api_can_build_a_frame_the_decoder_would_reject() {
    // Every frame the builders can produce must survive a round trip.
    for message_type in MessageType::ALL {
        for flags in [
            Flags::NONE,
            Flags::END_OF_ITEM,
            Flags::END_OF_TRANSFER,
            Flags::END_OF_ITEM.union(Flags::END_OF_TRANSFER),
        ] {
            let frame = Frame::new(message_type, b"payload".to_vec())
                .expect("valid")
                .with_flags(flags)
                .expect("only transport flags are publicly settable");

            let mut decoder = FrameDecoder::new();
            decoder.push(&frame.encode()).expect("within buffer");
            let decoded = decoder
                .next_frame()
                .unwrap_or_else(|error| {
                    panic!("{message_type:?} with {flags:?} was rejected: {error}")
                })
                .expect("frame");
            assert_eq!(decoded.try_encode().expect("retained"), frame.encode());
        }
    }
}

#[test]
fn transport_flags_cannot_smuggle_in_a_protected_flag() {
    let frame = Frame::new(MessageType::DataChunk, b"x".to_vec()).expect("valid");
    assert!(
        frame.clone().with_flags(Flags::ENCRYPTED).is_err(),
        "ENCRYPTED must not be publicly settable"
    );
    assert!(
        frame.with_flags(Flags::COMPRESSED).is_err(),
        "COMPRESSED must not be publicly settable"
    );
}
