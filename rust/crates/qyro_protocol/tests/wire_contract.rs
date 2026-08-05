//! Wire contracts for QYRO/1 framing.
//!
//! These tests freeze the byte layout and the numeric message values. A change
//! that breaks one of them is a wire-compatibility break, not a refactor.

use qyro_protocol::{
    DecodedFrame, Flags, Frame, FrameDecoder, FrameError, FrameHeader, HEADER_LEN, MAGIC,
    MAX_BUFFER_LEN, MAX_FRAME_LEN, MAX_HEADER_LEN, MAX_PAYLOAD_LEN, MessageType,
    SUPPORTED_TRAILER_LEN, VERSION_MAJOR, VERSION_MINOR,
};

fn encoded(message_type: MessageType, payload: Vec<u8>) -> Vec<u8> {
    Frame::new(message_type, payload)
        .expect("payload within limits")
        .encode()
}

fn decode_one(bytes: &[u8]) -> Result<Option<DecodedFrame>, FrameError> {
    let mut decoder = FrameDecoder::new();
    decoder.push(bytes)?;
    decoder.next_frame()
}

/// Decodes and unwraps a known message, failing loudly on anything else.
fn decode_message(bytes: &[u8]) -> Frame {
    match decode_one(bytes)
        .expect("decodes")
        .expect("one whole frame")
    {
        DecodedFrame::Message(frame) => frame,
        DecodedFrame::Encrypted(_) => panic!("expected a plain message, got an encrypted envelope"),
        DecodedFrame::Unsupported(event) => {
            panic!(
                "expected a known message, got type {}",
                event.message_type_value()
            )
        }
    }
}

#[test]
fn message_type_wire_values_are_frozen() {
    // Changing any of these is a major version change.
    let expected: [(MessageType, u8); 17] = [
        (MessageType::Hello, 1),
        (MessageType::Capabilities, 2),
        (MessageType::Pairing, 3),
        (MessageType::TransferOffer, 4),
        (MessageType::TransferAccept, 5),
        (MessageType::TransferReject, 6),
        (MessageType::Manifest, 7),
        (MessageType::ItemStart, 8),
        (MessageType::DataChunk, 9),
        (MessageType::ChunkAck, 10),
        (MessageType::Pause, 11),
        (MessageType::Resume, 12),
        (MessageType::Cancel, 13),
        (MessageType::Complete, 14),
        (MessageType::IntegrityResult, 15),
        (MessageType::Error, 16),
        (MessageType::Heartbeat, 17),
    ];

    for (message_type, wire) in expected {
        assert_eq!(message_type.to_wire(), wire, "{message_type:?} wire value");
        assert_eq!(
            MessageType::from_wire(wire).expect("known type"),
            message_type
        );
    }
    assert_eq!(MessageType::ALL.len(), expected.len());
}

#[test]
fn message_type_zero_is_reserved_so_a_zeroed_buffer_never_decodes() {
    assert_eq!(
        MessageType::from_wire(0),
        Err(FrameError::UnknownMessageType { value: 0 })
    );
    let zeroed = [0u8; HEADER_LEN];
    assert!(matches!(
        FrameHeader::decode(&zeroed),
        Err(FrameError::InvalidMagic { .. })
    ));
}

#[test]
fn header_layout_is_frozen() {
    let header = FrameHeader::new(MessageType::DataChunk, 4)
        .expect("within limits")
        .with_transport_flags(Flags::END_OF_ITEM)
        .expect("transport flag")
        .with_identifiers(
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            0x2122_2324,
            0x3132_3334,
        )
        .with_sequence(0x4142_4344_4546_4748);

    let bytes = header.encode();
    assert_eq!(bytes.len(), 48);
    assert_eq!(&bytes[0..4], &MAGIC);
    assert_eq!(&bytes[0..4], b"QYRO");
    assert_eq!(bytes[4], VERSION_MAJOR);
    assert_eq!(bytes[5], VERSION_MINOR);
    assert_eq!(bytes[6], MessageType::DataChunk.to_wire());
    assert_eq!(bytes[7], 0b0000_0001);
    // Big-endian everywhere.
    assert_eq!(&bytes[8..10], &[0x00, 0x30]);
    assert_eq!(bytes[10], SUPPORTED_TRAILER_LEN as u8);
    assert_eq!(bytes[11], 0);
    assert_eq!(&bytes[12..16], &[0x00, 0x00, 0x00, 0x04]);
    assert_eq!(&bytes[16..24], &0x0102_0304_0506_0708u64.to_be_bytes());
    assert_eq!(&bytes[24..32], &0x1112_1314_1516_1718u64.to_be_bytes());
    assert_eq!(&bytes[32..36], &0x2122_2324u32.to_be_bytes());
    assert_eq!(&bytes[36..40], &0x3132_3334u32.to_be_bytes());
    assert_eq!(&bytes[40..48], &0x4142_4344_4546_4748u64.to_be_bytes());

    assert_eq!(FrameHeader::decode(&bytes).expect("round trip"), header);
}

#[test]
fn round_trip_preserves_every_message_type() {
    for message_type in MessageType::ALL {
        let payload = vec![message_type.to_wire(); 16];
        let bytes = encoded(message_type, payload.clone());
        let frame = decode_message(&bytes);
        assert_eq!(frame.message_type(), message_type);
        assert_eq!(frame.payload(), payload.as_slice());
    }
}

#[test]
fn round_trip_preserves_identifiers_flags_and_sequence() {
    let frame = Frame::new(MessageType::ItemStart, b"payload".to_vec())
        .expect("within limits")
        .with_identifiers(u64::MAX, 7, u32::MAX, 9)
        .with_sequence(u64::MAX)
        .with_flags(Flags::END_OF_TRANSFER)
        .expect("transport flag");

    let decoded = decode_message(&frame.encode());
    assert_eq!(decoded, frame);
    assert_eq!(decoded.header().session_id(), u64::MAX);
    assert_eq!(decoded.header().sequence(), u64::MAX);
    assert!(decoded.header().flags().contains(Flags::END_OF_TRANSFER));
}

#[test]
fn empty_payload_round_trips() {
    let bytes = encoded(MessageType::Heartbeat, Vec::new());
    assert_eq!(bytes.len(), HEADER_LEN);
    let frame = decode_message(&bytes);
    assert!(frame.payload().is_empty());
}

#[test]
fn maximum_payload_is_accepted_and_one_more_byte_is_rejected() {
    let at_limit = vec![0xA5; MAX_PAYLOAD_LEN];
    let frame = Frame::new(MessageType::DataChunk, at_limit).expect("exactly at the limit");
    let decoded = decode_message(&frame.encode());
    assert_eq!(decoded.payload().len(), MAX_PAYLOAD_LEN);

    let over_limit = vec![0xA5; MAX_PAYLOAD_LEN + 1];
    assert!(matches!(
        Frame::new(MessageType::DataChunk, over_limit),
        Err(FrameError::PayloadTooLarge { .. })
    ));
}

#[test]
fn header_truncated_at_every_byte_reports_truncation_not_a_frame() {
    let bytes = encoded(MessageType::Hello, b"abc".to_vec());
    for cut in 0..HEADER_LEN {
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes[..cut]).expect("within buffer");
        assert_eq!(
            decoder.next_frame().expect("no framing error yet"),
            None,
            "a {cut}-byte prefix must not yield a frame"
        );
        assert!(matches!(
            FrameHeader::decode(&bytes[..cut]),
            Err(FrameError::TruncatedHeader { .. })
        ));
    }
}

#[test]
fn payload_truncated_at_every_byte_waits_instead_of_yielding() {
    let bytes = encoded(MessageType::Manifest, vec![7; 32]);
    for cut in HEADER_LEN..bytes.len() {
        let mut decoder = FrameDecoder::new();
        decoder.push(&bytes[..cut]).expect("within buffer");
        assert_eq!(decoder.next_frame().expect("header is valid"), None);
    }
}

#[test]
fn corrupt_magic_is_rejected() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[0] = b'X';
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::InvalidMagic { found }) if found == *b"XYRO"
    ));
}

#[test]
fn incompatible_major_version_is_rejected() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[4] = VERSION_MAJOR + 1;
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::UnsupportedMajorVersion { found, supported })
            if found == VERSION_MAJOR + 1 && supported == VERSION_MAJOR
    ));
}

#[test]
fn future_minor_version_is_accepted() {
    let mut bytes = encoded(MessageType::Hello, b"forward".to_vec());
    bytes[5] = 9;
    let frame = decode_message(&bytes);
    assert_eq!(frame.header().version_minor(), 9);
    assert_eq!(frame.payload(), b"forward");
}

#[test]
fn a_future_minor_header_extension_is_refused_not_skipped() {
    // ADR-0018 reversed this: skipping bytes that are never stored made
    // decode->encode lossy and would leave a future AEAD unable to authenticate
    // them. 1.0 now says plainly that it does not support extensions.
    let extension: usize = 8;
    let mut bytes = FrameHeader::new(MessageType::Capabilities, 3)
        .expect("within limits")
        .encode()
        .to_vec();
    bytes[5] = 4; // newer minor
    bytes[8..10].copy_from_slice(
        &u16::try_from(HEADER_LEN + extension)
            .expect("fits")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&[0xDE; 8]);
    bytes.extend_from_slice(b"abc");

    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::UnsupportedHeaderExtension { declared, supported })
            if declared as usize == HEADER_LEN + extension && supported as usize == HEADER_LEN
    ));
}

#[test]
fn header_length_below_the_fixed_minimum_is_rejected() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[8..10].copy_from_slice(&8u16.to_be_bytes());
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::InvalidHeaderLength { declared: 8, .. })
    ));
}

#[test]
fn header_length_above_the_maximum_is_rejected() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    let oversize = u16::try_from(MAX_HEADER_LEN + 1).expect("fits in u16");
    bytes[8..10].copy_from_slice(&oversize.to_be_bytes());
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::InvalidHeaderLength { declared, .. }) if declared == oversize
    ));
}

#[test]
fn unknown_flag_bits_are_rejected_rather_than_ignored() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[7] = 0b1000_0000;
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::InvalidFlags {
            bits: 0b1000_0000,
            ..
        })
    ));
}

#[test]
fn non_zero_reserved_byte_is_rejected() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[11] = 1;
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::InvalidFlags { bits: 1, .. })
    ));
}

#[test]
fn unknown_message_type_is_a_delimited_event_not_a_framing_failure() {
    // ADR-0018: the frame is fully delimited before the type is resolved, so it
    // is consumed whole and reported instead of killing the stream.
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[6] = 200;
    let decoded = decode_one(&bytes).expect("not a framing failure");
    assert!(matches!(
        decoded,
        Some(DecodedFrame::Unsupported(event)) if event.message_type_value() == 200
    ));

    // The type itself is still unknown at the MessageType level.
    assert_eq!(
        MessageType::from_wire(200),
        Err(FrameError::UnknownMessageType { value: 200 })
    );
}

#[test]
fn authentication_trailer_is_refused_until_something_verifies_it() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[10] = 16;
    assert!(matches!(
        decode_one(&bytes),
        Err(FrameError::AuthenticationTrailerInvalid {
            declared: 16,
            expected: 0
        })
    ));
}

#[test]
fn hostile_payload_length_is_rejected_without_a_proportional_reservation() {
    // The core safety property: a peer declaring 4 GiB must not make this
    // process reserve anything close to it.
    let mut bytes = encoded(MessageType::DataChunk, Vec::new());
    bytes[12..16].copy_from_slice(&u32::MAX.to_be_bytes());

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("48 bytes fit");
    let before = decoder.buffer_capacity();

    assert!(matches!(
        decoder.next_frame(),
        Err(FrameError::PayloadTooLarge {
            declared: u32::MAX,
            ..
        })
    ));

    assert_eq!(decoder.buffer_capacity(), before);
    assert!(
        decoder.buffer_capacity() <= MAX_BUFFER_LEN,
        "capacity {} must stay bounded",
        decoder.buffer_capacity()
    );
}

#[test]
fn every_declared_length_stays_within_the_frame_limit() {
    assert_eq!(MAX_FRAME_LEN, MAX_HEADER_LEN + MAX_PAYLOAD_LEN + 64);
    let header =
        FrameHeader::new(MessageType::DataChunk, MAX_PAYLOAD_LEN as u32).expect("at the limit");
    assert!(header.total_len() <= MAX_FRAME_LEN as u64);
}

#[test]
fn several_frames_in_one_buffer_are_yielded_in_order() {
    let mut stream = Vec::new();
    stream.extend_from_slice(&encoded(MessageType::Hello, b"one".to_vec()));
    stream.extend_from_slice(&encoded(MessageType::DataChunk, b"two".to_vec()));
    stream.extend_from_slice(&encoded(MessageType::Complete, Vec::new()));

    let mut decoder = FrameDecoder::new();
    decoder.push(&stream).expect("within buffer");

    let first = decoder.next_frame().expect("ok").expect("frame");
    let second = decoder.next_frame().expect("ok").expect("frame");
    let third = decoder.next_frame().expect("ok").expect("frame");

    assert_eq!(first.message_type(), Some(MessageType::Hello));
    assert_eq!(first.plaintext().expect("plain"), b"one");
    assert_eq!(second.message_type(), Some(MessageType::DataChunk));
    assert_eq!(second.plaintext().expect("plain"), b"two");
    assert_eq!(third.message_type(), Some(MessageType::Complete));
    assert_eq!(decoder.next_frame().expect("ok"), None);
    assert_eq!(decoder.buffered_len(), 0);
}

#[test]
fn byte_by_byte_delivery_matches_a_single_delivery() {
    let mut stream = Vec::new();
    stream.extend_from_slice(&encoded(MessageType::Hello, b"alpha".to_vec()));
    stream.extend_from_slice(&encoded(MessageType::ChunkAck, vec![9; 40]));

    let mut whole = FrameDecoder::new();
    whole.push(&stream).expect("within buffer");
    let mut expected = Vec::new();
    while let Some(frame) = whole.next_frame().expect("ok") {
        expected.push(frame);
    }

    let mut incremental = FrameDecoder::new();
    let mut actual = Vec::new();
    for byte in &stream {
        incremental.push(&[*byte]).expect("within buffer");
        while let Some(frame) = incremental.next_frame().expect("ok") {
            actual.push(frame);
        }
    }

    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 2);
}

#[test]
fn trailing_bytes_of_an_incomplete_frame_are_retained() {
    let complete = encoded(MessageType::Hello, b"done".to_vec());
    let partial = encoded(MessageType::DataChunk, vec![1; 64]);

    let mut decoder = FrameDecoder::new();
    decoder.push(&complete).expect("within buffer");
    decoder.push(&partial[..HEADER_LEN + 10]).expect("partial");

    assert!(decoder.next_frame().expect("ok").is_some());
    assert_eq!(decoder.next_frame().expect("ok"), None);
    assert_eq!(decoder.buffered_len(), HEADER_LEN + 10);

    decoder.push(&partial[HEADER_LEN + 10..]).expect("rest");
    let frame = decoder.next_frame().expect("ok").expect("now complete");
    assert_eq!(frame.plaintext().expect("plain").len(), 64);
    assert_eq!(decoder.buffered_len(), 0);
}

#[test]
fn buffer_limit_is_enforced_and_leaves_the_buffer_untouched() {
    let mut decoder = FrameDecoder::with_max_buffer_len(128);
    decoder.push(&[0; 100]).expect("fits");
    assert_eq!(decoder.buffered_len(), 100);

    assert!(matches!(
        decoder.push(&[0; 29]),
        Err(FrameError::BufferLimitExceeded {
            attempted: 129,
            limit: 128
        })
    ));
    assert_eq!(decoder.buffered_len(), 100, "rejected push must not append");
}

#[test]
fn custom_buffer_limit_cannot_exceed_the_protocol_ceiling() {
    // A caller must not be able to widen the bound past what the protocol
    // guarantees, so the request is clamped and shows up through push.
    let mut clamped = FrameDecoder::with_max_buffer_len(usize::MAX);
    assert!(matches!(
        clamped.push(&vec![0; MAX_BUFFER_LEN + 1]),
        Err(FrameError::BufferLimitExceeded { limit, .. }) if limit == MAX_BUFFER_LEN
    ));
}

#[test]
fn a_framing_error_poisons_the_stream_until_reset() {
    let mut bytes = encoded(MessageType::Hello, Vec::new());
    bytes[0] = b'Z';

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");

    let first = decoder.next_frame().expect_err("magic is wrong");
    assert!(decoder.is_poisoned());
    // Desynchronised framing must not be guessed at: the same error repeats.
    assert_eq!(decoder.next_frame().expect_err("still poisoned"), first);
    assert_eq!(decoder.push(b"more").expect_err("still poisoned"), first);

    decoder.reset();
    assert!(!decoder.is_poisoned());
    assert_eq!(decoder.buffered_len(), 0);

    decoder
        .push(&encoded(MessageType::Hello, b"recovered".to_vec()))
        .expect("usable again");
    let frame = decoder.next_frame().expect("ok").expect("frame");
    assert_eq!(frame.plaintext(), Some(&b"recovered"[..]));
}

#[test]
fn reset_discards_a_partial_frame() {
    let bytes = encoded(MessageType::DataChunk, vec![3; 32]);
    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes[..HEADER_LEN + 5]).expect("partial");
    assert_eq!(decoder.buffered_len(), HEADER_LEN + 5);

    decoder.reset();
    assert_eq!(decoder.buffered_len(), 0);
    assert_eq!(decoder.next_frame().expect("ok"), None);
}

#[test]
fn arbitrary_bytes_never_panic() {
    // A cheap deterministic sweep; proptest covers the same property broadly.
    let mut decoder = FrameDecoder::new();
    for seed in 0u16..=u16::from(u8::MAX) {
        let byte = u8::try_from(seed & 0xFF).expect("masked");
        let noise = vec![byte; HEADER_LEN + usize::from(byte % 17)];
        decoder.reset();
        if decoder.push(&noise).is_err() {
            continue;
        }
        let _ = decoder.next_frame();
    }
}
