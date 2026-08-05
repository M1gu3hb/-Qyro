//! Contracts on what the public API is allowed to express.
//!
//! Each test corresponds to a sprint-4A audit finding: a place where the API
//! let a caller state something untrue, or where the crate substituted a value
//! it had no basis for.

use qyro_protocol::{
    DecodedFrame, Flags, Frame, FrameDecoder, FrameError, MAX_PAYLOAD_LEN, MessageType,
};

fn encoded(message_type: MessageType, payload: &[u8]) -> Vec<u8> {
    Frame::new(message_type, payload.to_vec())
        .expect("payload within limits")
        .encode()
}

fn decode_one(bytes: &[u8]) -> DecodedFrame {
    let mut decoder = FrameDecoder::new();
    decoder.push(bytes).expect("within buffer");
    decoder
        .next_frame()
        .expect("not a framing failure")
        .expect("one whole frame")
}

// ------------------------------------------- no substitution of unknown types

#[test]
fn an_unknown_type_never_becomes_a_known_one() {
    // The decoder used unwrap_or(MessageType::Hello) internally. Even though the
    // value was not surfaced on the Unsupported path, a parser that invents a
    // known type is one refactor away from leaking it.
    for raw in [0u8, 18, 99, 200, 255] {
        let mut bytes = encoded(MessageType::Hello, b"opaque");
        bytes[6] = raw;

        let decoded = decode_one(&bytes);
        assert_eq!(
            decoded.message_type(),
            None,
            "raw type {raw} must not resolve to any MessageType"
        );
        assert!(
            decoded.as_plain().is_none(),
            "raw type {raw} must not surface as a plain frame"
        );

        let DecodedFrame::Unsupported(event) = decoded else {
            panic!("raw type {raw} must decode as Unsupported");
        };
        assert_eq!(
            event.message_type_value(),
            raw,
            "the raw value is preserved"
        );
    }
}

#[test]
fn no_public_api_reports_hello_for_an_unknown_type() {
    let mut bytes = encoded(MessageType::Hello, b"payload");
    bytes[6] = 200;

    let decoded = decode_one(&bytes);
    // Whatever accessor a caller reaches for, none may claim Hello.
    assert_ne!(decoded.message_type(), Some(MessageType::Hello));
    assert!(decoded.as_plain().is_none());
    assert!(decoded.as_encrypted().is_none());
}

// -------------------------------------------------- fully bounded construction

#[test]
fn a_public_constructor_cannot_declare_an_out_of_range_payload() {
    // Frame::new is the only public way to reach a header, and it refuses.
    let over = vec![0u8; MAX_PAYLOAD_LEN + 1];
    assert!(matches!(
        Frame::new(MessageType::DataChunk, over),
        Err(FrameError::PayloadTooLarge { .. })
    ));

    // Exactly at the limit is fine and round-trips.
    let at = vec![0xA5; MAX_PAYLOAD_LEN];
    let frame = Frame::new(MessageType::DataChunk, at).expect("at the limit");
    let decoded = decode_one(&frame.encode());
    assert_eq!(decoded.plaintext().map(<[u8]>::len), Some(MAX_PAYLOAD_LEN));
}

#[test]
fn every_frame_a_public_api_can_build_survives_its_own_decoder() {
    for message_type in MessageType::ALL {
        for flags in [
            Flags::NONE,
            Flags::END_OF_ITEM,
            Flags::END_OF_TRANSFER,
            Flags::END_OF_ITEM.union(Flags::END_OF_TRANSFER),
        ] {
            let frame = Frame::new(message_type, b"body".to_vec())
                .expect("valid")
                .with_identifiers(u64::MAX, 3, u32::MAX, 5)
                .with_sequence(u64::MAX)
                .with_flags(flags)
                .expect("transport flags only");

            let bytes = frame.encode();
            let decoded = decode_one(&bytes);
            assert_eq!(
                decoded.try_encode().expect("a plain frame re-encodes"),
                bytes,
                "{message_type:?} with {flags:?} must round-trip byte-exactly"
            );
        }
    }
}

// ------------------------------------- no fabricated cryptographic guarantees

#[test]
fn no_public_type_claims_authentication_from_caller_supplied_bytes() {
    // An architecture contract. qyro_protocol performs no cryptography, so no
    // type it exposes may be named or documented as if it did. A caller handing
    // over an arbitrary byte vector is not evidence of authentication.
    //
    // The envelope is what the wire carries; verifying it belongs to
    // qyro_crypto, which does not exist yet. When it does, `SealedFrame` and
    // `AuthenticatedFrame` will live there with private constructors.
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is readable");

    assert!(
        !source.contains("pub use envelope::SealedFrame"),
        "qyro_protocol must not export a type named SealedFrame: it cannot seal anything"
    );
    assert!(
        source.contains("EncryptedEnvelope"),
        "the wire-level ciphertext carrier must be named as an envelope"
    );
}

#[test]
fn an_envelope_documents_itself_as_unverified() {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/envelope.rs"))
        .expect("envelope.rs is readable");
    let lowered = source.to_lowercase();
    assert!(
        lowered.contains("untrusted until"),
        "the envelope must state that it is untrusted until qyro_crypto verifies it"
    );
}

// ------------------------------------------------ transport metadata survives

#[test]
fn an_envelope_preserves_every_authenticated_metadata_field() {
    let template = Frame::new(MessageType::DataChunk, b"plaintext".to_vec())
        .expect("valid")
        .with_identifiers(
            0x0102_0304_0506_0708,
            0x1112_1314_1516_1718,
            0x2122_2324,
            0x3132_3334,
        )
        .with_sequence(0x4142_4344_4546_4748)
        .with_flags(Flags::END_OF_ITEM.union(Flags::END_OF_TRANSFER))
        .expect("transport flags");

    let ciphertext = vec![0xCC; 9];
    let tag = vec![0x77; 16];
    let envelope = qyro_protocol::EncryptedEnvelope::from_plain(
        template.header(),
        ciphertext.clone(),
        tag.clone(),
    )
    .expect("valid envelope");

    let header = envelope.header();
    assert_eq!(header.message_type(), MessageType::DataChunk);
    assert_eq!(header.session_id(), 0x0102_0304_0506_0708);
    assert_eq!(header.transfer_id(), 0x1112_1314_1516_1718);
    assert_eq!(header.stream_id(), 0x2122_2324);
    assert_eq!(header.item_id(), 0x3132_3334);
    assert_eq!(header.sequence(), 0x4142_4344_4546_4748);
    assert!(header.flags().contains(Flags::END_OF_ITEM));
    assert!(header.flags().contains(Flags::END_OF_TRANSFER));

    // Only these three are derived rather than carried over.
    assert!(header.flags().contains(Flags::ENCRYPTED));
    assert_eq!(header.payload_len() as usize, ciphertext.len());
    assert_eq!(usize::from(header.trailer_len()), tag.len());
}

#[test]
fn an_envelope_survives_the_wire_byte_exactly() {
    let template = Frame::new(MessageType::ItemStart, b"x".to_vec())
        .expect("valid")
        .with_identifiers(9, 8, 7, 6)
        .with_sequence(5)
        .with_flags(Flags::END_OF_ITEM)
        .expect("transport flags");

    let envelope = qyro_protocol::EncryptedEnvelope::from_plain(
        template.header(),
        vec![0xAB; 32],
        vec![0x01; 16],
    )
    .expect("valid envelope");

    let bytes = envelope.encode();
    let decoded = decode_one(&bytes);
    let received = decoded.as_encrypted().expect("decodes as an envelope");

    assert_eq!(received, &envelope);
    assert_eq!(received.encode(), bytes);
    assert_eq!(received.associated_data(), envelope.associated_data());
}

// ------------------------------------------------- distinct states, no sentinel

#[test]
fn empty_plaintext_and_ciphertext_and_unknown_are_distinct_states() {
    // An empty plain payload used to be indistinguishable from a sealed frame
    // and from an unknown type, because all three answered `&[]`.
    let empty_plain = decode_one(&encoded(MessageType::Heartbeat, b""));

    let template = Frame::new(MessageType::DataChunk, b"".to_vec()).expect("valid");
    let envelope =
        qyro_protocol::EncryptedEnvelope::from_plain(template.header(), Vec::new(), vec![9; 16])
            .expect("valid envelope");
    let encrypted = decode_one(&envelope.encode());

    let mut unknown_bytes = encoded(MessageType::Hello, b"");
    unknown_bytes[6] = 200;
    let unknown = decode_one(&unknown_bytes);

    // Plaintext is available only for the plain frame, and it is Some(empty).
    assert_eq!(empty_plain.plaintext(), Some(&[][..]));
    assert_eq!(
        encrypted.plaintext(),
        None,
        "ciphertext is not plaintext and must not be offered as one"
    );
    assert_eq!(unknown.plaintext(), None);

    assert!(empty_plain.as_plain().is_some());
    assert!(encrypted.as_encrypted().is_some());
    assert!(matches!(unknown, DecodedFrame::Unsupported(_)));
}

// --------------------------------------------------------------- no panics

#[test]
fn no_variant_the_decoder_can_produce_panics_on_the_normal_api() {
    let mut unknown_bytes = encoded(MessageType::Hello, b"body");
    unknown_bytes[6] = 200;

    let template = Frame::new(MessageType::DataChunk, b"x".to_vec()).expect("valid");
    let envelope =
        qyro_protocol::EncryptedEnvelope::from_plain(template.header(), vec![1; 4], vec![2; 16])
            .expect("valid envelope");

    let variants = vec![
        decode_one(&encoded(MessageType::Hello, b"plain")),
        decode_one(&envelope.encode()),
        decode_one(&unknown_bytes),
    ];

    for variant in variants {
        // None of these may panic for any variant the decoder produced from
        // legitimate peer input.
        let _ = variant.message_type();
        let _ = variant.plaintext();
        let _ = variant.as_plain();
        let _ = variant.as_encrypted();
        let encoded_result = variant.try_encode();
        match &variant {
            DecodedFrame::Message(_) | DecodedFrame::Encrypted(_) => {
                assert!(encoded_result.is_ok(), "retained frames re-encode");
            }
            DecodedFrame::Unsupported(_) => {
                // Its bytes were deliberately not retained, so this reports an
                // error rather than panicking.
                assert!(encoded_result.is_err());
            }
        }
    }
}
