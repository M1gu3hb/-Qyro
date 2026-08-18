//! Focal contracts for peer-controlled framing boundaries found by mutation.

use qyro_protocol::{
    DecodedFrame, EncryptedEnvelope, Flags, Frame, FrameDecoder, FrameError, FrameHeader,
    HEADER_LEN, MAX_HEADER_LEN, MAX_PAYLOAD_LEN, MAX_TRAILER_LEN, MessageType, SessionId,
};

fn plain(payload: Vec<u8>) -> Frame {
    Frame::new(MessageType::DataChunk, payload).expect("the fixture is within protocol limits")
}

#[test]
fn ciphertext_accepts_the_exact_limit_and_refuses_one_byte_more() {
    let template = plain(Vec::new());
    EncryptedEnvelope::from_plain_frame(&template, vec![0xA5; MAX_PAYLOAD_LEN], vec![7])
        .expect("the exact ciphertext limit is valid");
    assert!(matches!(
        EncryptedEnvelope::from_plain_frame(
            &template,
            vec![0xA5; MAX_PAYLOAD_LEN + 1],
            vec![7]
        ),
        Err(FrameError::PayloadTooLarge {
            declared,
            limit
        }) if declared == (MAX_PAYLOAD_LEN + 1) as u32 && limit == MAX_PAYLOAD_LEN as u32
    ));
}

#[test]
fn authentication_trailer_distinguishes_zero_maximum_and_one_too_many() {
    let template = plain(Vec::new());
    assert!(matches!(
        EncryptedEnvelope::from_plain_frame(&template, Vec::new(), Vec::new()),
        Err(FrameError::AuthenticationTrailerInvalid { declared: 0, .. })
    ));
    EncryptedEnvelope::from_plain_frame(&template, Vec::new(), vec![7; MAX_TRAILER_LEN])
        .expect("the exact trailer limit is valid");
    assert!(matches!(
        EncryptedEnvelope::from_plain_frame(&template, Vec::new(), vec![7; MAX_TRAILER_LEN + 1]),
        Err(FrameError::AuthenticationTrailerInvalid { .. })
    ));
}

#[test]
fn associated_data_is_the_exact_header_and_not_a_placeholder() {
    let template = plain(vec![1, 2, 3])
        .with_identifiers(SessionId::from_u64(0x0102_0304_0506_0708), 9, 10, 11)
        .with_sequence(12);
    let envelope = EncryptedEnvelope::from_plain_frame(&template, vec![4, 5, 6], vec![7, 8])
        .expect("valid envelope");

    assert_eq!(envelope.associated_data(), envelope.header().encode());
    assert_ne!(envelope.associated_data(), [0; HEADER_LEN]);
    assert_ne!(envelope.associated_data(), [1; HEADER_LEN]);
}

#[test]
fn every_fixed_width_identifier_decodes_from_its_own_wire_offset() {
    let frame = plain(vec![0xAB])
        .with_identifiers(
            SessionId::from_u64(0x0102_0304_0506_0708),
            0x1112_1314_1516_1718,
            0x2122_2324,
            0x3132_3334,
        )
        .with_sequence(0x4142_4344_4546_4748);
    let decoded = FrameHeader::decode(&frame.encode()[..HEADER_LEN]).expect("valid header");

    assert_eq!(decoded.session_id().to_u64(), 0x0102_0304_0506_0708);
    assert_eq!(decoded.transfer_id(), 0x1112_1314_1516_1718);
    assert_eq!(decoded.stream_id(), 0x2122_2324);
    assert_eq!(decoded.item_id(), 0x3132_3334);
    assert_eq!(decoded.sequence(), 0x4142_4344_4546_4748);
}

#[test]
fn exact_maximum_header_length_is_an_extension_not_an_out_of_range_length() {
    let mut bytes = plain(Vec::new()).encode();
    bytes[8..10].copy_from_slice(&(MAX_HEADER_LEN as u16).to_be_bytes());

    assert_eq!(
        FrameHeader::decode(&bytes[..HEADER_LEN]),
        Err(FrameError::UnsupportedHeaderExtension {
            declared: MAX_HEADER_LEN as u16,
            supported: HEADER_LEN as u16,
        })
    );
}

#[test]
fn encrypted_wire_trailer_distinguishes_zero_maximum_and_one_too_many() {
    let template = plain(Vec::new());
    let maximum =
        EncryptedEnvelope::from_plain_frame(&template, Vec::new(), vec![9; MAX_TRAILER_LEN])
            .expect("the exact maximum is constructible");
    FrameHeader::decode(&maximum.encode()[..HEADER_LEN]).expect("maximum header is accepted");

    let mut zero = maximum.encode();
    zero[10] = 0;
    assert_eq!(
        FrameHeader::decode(&zero[..HEADER_LEN]),
        Err(FrameError::EncryptedWithoutTrailer { declared: 0 })
    );

    let mut over = maximum.encode();
    over[10] = (MAX_TRAILER_LEN + 1) as u8;
    assert_eq!(
        FrameHeader::decode(&over[..HEADER_LEN]),
        Err(FrameError::EncryptedWithoutTrailer {
            declared: (MAX_TRAILER_LEN + 1) as u8,
        })
    );
}

#[test]
fn an_unknown_empty_type_consumes_exactly_header_plus_payload() {
    let payload = vec![1, 2, 3, 4, 5];
    let mut bytes = plain(payload.clone()).encode();
    bytes[6] = 0xFE;
    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("one frame fits");

    let DecodedFrame::Unsupported(frame) = decoder
        .next_frame()
        .expect("well-formed unknown frame")
        .expect("one frame is ready")
    else {
        panic!("the unknown type must stay unknown");
    };
    assert_eq!(frame.payload_len(), payload.len() as u32);
    assert_eq!(frame.total_len(), HEADER_LEN + payload.len());
    assert_eq!(decoder.buffered_len(), 0);
}

#[test]
fn an_unknown_encrypted_type_counts_its_trailer_when_it_consumes() {
    let template = plain(Vec::new());
    let mut bytes = EncryptedEnvelope::from_plain_frame(&template, vec![1, 2, 3, 4, 5], vec![6, 7])
        .expect("valid encrypted envelope")
        .encode();
    bytes[6] = 0xFE;
    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("one frame fits");

    let DecodedFrame::Unsupported(frame) = decoder
        .next_frame()
        .expect("well-formed unknown frame")
        .expect("one frame is ready")
    else {
        panic!("the unknown type must stay unknown");
    };
    assert_eq!(frame.payload_len(), 5);
    assert_eq!(frame.total_len(), HEADER_LEN + 5 + 2);
    assert_eq!(decoder.buffered_len(), 0);
}

#[test]
fn protected_and_union_bits_keep_their_exact_meaning() {
    assert_eq!(Flags::END_OF_ITEM.bits(), 0b0000_0001);
    assert_eq!(Flags::END_OF_TRANSFER.bits(), 0b0000_0010);
    assert_eq!(Flags::ENCRYPTED.protected_bits(), Flags::ENCRYPTED.bits());
    assert_eq!(
        Flags::ENCRYPTED.union(Flags::ENCRYPTED),
        Flags::ENCRYPTED,
        "set union is idempotent even when the operands overlap"
    );
}
