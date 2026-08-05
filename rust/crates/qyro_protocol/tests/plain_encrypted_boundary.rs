//! The boundary between a plain frame and an encrypted envelope.
//!
//! Sprint-4A left one direction of that boundary unguarded. `EncryptedEnvelope`
//! exposes its `FrameHeader`, `FrameHeader` is `Copy`, and `Frame::from_parts`
//! accepted any header at all. So a caller could take the header off an
//! envelope, staple a body to it, and hold a `Frame` whose `payload()` answers
//! ciphertext as if it were plaintext.
//!
//! That is not a naming complaint. `Frame::encode` on such a value emits bytes
//! this crate's own decoder rejects, which contradicts the invariant `header.rs`
//! states outright: no public API may produce a header the decoder would refuse.

use qyro_protocol::{
    DecodedFrame, EncryptedEnvelope, Flags, Frame, FrameDecoder, FrameError, MessageType,
};

fn plain_frame() -> Frame {
    Frame::new(MessageType::DataChunk, b"plaintext".to_vec())
        .expect("payload within limits")
        .with_identifiers(11, 22, 33, 44)
        .with_sequence(55)
        .with_flags(Flags::END_OF_ITEM)
        .expect("transport flags are settable")
}

fn envelope() -> EncryptedEnvelope {
    EncryptedEnvelope::from_plain_frame(&plain_frame(), vec![0xCC; 9], vec![0x77; 16])
        .expect("ciphertext and trailer are within limits")
}

// ------------------------------------------------ an encrypted header is not plain

#[test]
fn an_encrypted_header_cannot_be_laundered_into_a_plain_frame() {
    let sealed = envelope();
    let stolen = *sealed.header();

    // A body of exactly the length the header declares, so the only thing left
    // to reject is the claim the header makes about its own protection.
    let body = vec![0xCC; stolen.payload_len() as usize];

    let error = Frame::from_parts(stolen, body)
        .expect_err("a header carrying ENCRYPTED does not describe a plain frame");

    assert_eq!(
        error,
        FrameError::ProtectedHeaderNotPlain {
            flags: stolen.flags().bits(),
            trailer_len: stolen.trailer_len(),
        },
        "the rejection must name what was wrong, not merely fail"
    );
}

#[test]
fn a_trailer_length_alone_is_enough_to_disqualify_a_plain_frame() {
    // Even without ENCRYPTED, a non-zero trailer means the body on the wire is
    // longer than the payload. A `Frame` has no trailer to hold those bytes, so
    // its `encode()` would silently drop them.
    let sealed = envelope();
    let stolen = *sealed.header();
    assert_ne!(
        stolen.trailer_len(),
        0,
        "fixture must actually carry a trailer"
    );

    assert!(
        Frame::from_parts(stolen, vec![0; stolen.payload_len() as usize]).is_err(),
        "a header declaring a trailer cannot describe a frame that has none"
    );
}

#[test]
fn every_frame_the_public_api_can_build_survives_its_own_decoder() {
    // The invariant `header.rs` claims. Before the fix, the laundered frame
    // above encoded to bytes the decoder refused, because the header promised a
    // 16-byte trailer that `Frame::encode` never wrote.
    let frame = plain_frame();
    let bytes = frame.encode();

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    let decoded = decoder
        .next_frame()
        .expect("a frame this crate built is not a framing failure")
        .expect("one whole frame");

    match decoded {
        DecodedFrame::Message(received) => assert_eq!(received, frame),
        other => panic!("a plain frame must decode as a plain frame, got {other:?}"),
    }
}

// ------------------------------------------------- a template must be plain by type

#[test]
fn an_envelope_can_never_be_re_wrapped_as_its_own_template() {
    // `from_plain` took a `&FrameHeader`, and an envelope hands out exactly
    // that. Wrapping an already-encrypted header produced an envelope whose
    // ENCRYPTED flag and trailer length described the second wrap while the
    // ciphertext was the first, with nothing in the type system objecting.
    //
    // `from_plain_frame` takes a `&Frame`, and the test above proves an
    // encrypted header cannot become one. The two together close the path: this
    // test fails at the only remaining step, obtaining the `Frame`.
    let sealed = envelope();
    let laundered = Frame::from_parts(*sealed.header(), sealed.ciphertext().to_vec());

    assert!(
        laundered.is_err(),
        "if this ever succeeds, from_plain_frame can be handed an encrypted template"
    );
}

#[test]
fn a_template_keeps_its_transport_flags_and_identifiers() {
    // Unchanged behaviour, restated against the new signature so the fix cannot
    // quietly drop the metadata a future AEAD has to authenticate.
    let template = plain_frame();
    let sealed = EncryptedEnvelope::from_plain_frame(&template, vec![1; 4], vec![2; 16])
        .expect("valid envelope");

    let header = sealed.header();
    assert_eq!(header.message_type(), MessageType::DataChunk);
    assert_eq!(header.session_id(), 11);
    assert_eq!(header.transfer_id(), 22);
    assert_eq!(header.stream_id(), 33);
    assert_eq!(header.item_id(), 44);
    assert_eq!(header.sequence(), 55);
    assert!(header.flags().contains(Flags::END_OF_ITEM));
    assert!(header.flags().contains(Flags::ENCRYPTED));
}
