//! Contracts for the QYRO/1 frame AEAD.
//!
//! In-crate, like every other test that needs the deterministic constructors:
//! they are `cfg(test)` and crate-private, and an integration test is a separate
//! crate that could only reach them through public API.

use qyro_protocol::{
    DecodedFrame, EncryptedEnvelope, Flags, Frame, FrameDecoder, MessageType, SessionId,
};

use super::{
    AUTH_TRANSCRIPT_LEN, AeadError, Direction, DirectionalKeys, FrameOpener, FrameSealer,
    NONCE_LEN, PURPOSE_KEY, PURPOSE_NONCE_PREFIX, REPLAY_WINDOW, TAG_LEN, TRAFFIC_SECRET_LEN,
    info_for,
};
use crate::handshake::{InitiatorStart, ResponderStart};
use crate::identity::DeviceIdentity;

fn entropy(tag: u8) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = tag ^ (index as u8).wrapping_mul(13).wrapping_add(5);
    }
    out
}

/// A completed handshake, reduced to the four things this module needs.
struct Session {
    initiator_sealer: FrameSealer,
    initiator_opener: FrameOpener,
    responder_sealer: FrameSealer,
    responder_opener: FrameOpener,
    session_id: SessionId,
}

fn session_with(tag: u8) -> Session {
    let alice = DeviceIdentity::from_test_seed(&[0x11; 32]);
    let bob = DeviceIdentity::from_test_seed(&[0x22; 32]);

    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(0xA0 ^ tag))
        .expect("opens");
    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(0xB0 ^ tag))
        .expect("accepts");
    let (finish, awaiting_responder_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    let pending = awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies");
    let responder_finish = *pending.encoded_finish();
    let responder = pending.confirm_sent();
    let initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .expect("verifies");

    let session_id = initiator.session_id();
    let (initiator_sealer, initiator_opener) = initiator.into_frame_crypto().expect("derives");
    let (responder_sealer, responder_opener) = responder.into_frame_crypto().expect("derives");

    Session {
        initiator_sealer,
        initiator_opener,
        responder_sealer,
        responder_opener,
        session_id,
    }
}

fn session() -> Session {
    session_with(0)
}

fn plain(payload: &[u8]) -> Frame {
    Frame::new(MessageType::DataChunk, payload.to_vec()).expect("within limits")
}

// --------------------------------------------------------------- happy path

#[test]
fn what_the_initiator_seals_the_responder_opens() {
    let mut s = session();
    let sealed = s
        .initiator_sealer
        .seal(&plain(b"hello over the wire"))
        .expect("seals");

    let opened = s
        .responder_opener
        .open(sealed.envelope())
        .expect("the responder opens what the initiator sealed");

    assert_eq!(opened.payload(), b"hello over the wire");
    assert_eq!(opened.message_type(), MessageType::DataChunk);
    assert_eq!(opened.session_id(), s.session_id);
    assert_eq!(opened.sequence(), 0, "the first frame is sequence zero");
}

#[test]
fn what_the_responder_seals_the_initiator_opens() {
    let mut s = session();
    let sealed = s
        .responder_sealer
        .seal(&plain(b"the other way"))
        .expect("seals");
    let opened = s.initiator_opener.open(sealed.envelope()).expect("opens");
    assert_eq!(opened.payload(), b"the other way");
}

#[test]
fn the_two_directions_do_not_open_each_others_frames() {
    // Directional keys. A frame sealed for one direction must not open with the
    // key for the other, or a peer's own traffic could be reflected at it.
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"outbound")).expect("seals");

    assert_eq!(
        s.initiator_opener.open(sealed.envelope()).err(),
        Some(AeadError::AuthenticationFailed),
        "the initiator must not open its own outbound frame"
    );
}

#[test]
fn an_empty_payload_round_trips() {
    // Zero-length plaintext still produces a tag over the header, so the frame
    // is authenticated even though it carries nothing.
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"")).expect("seals");
    assert_eq!(sealed.envelope().ciphertext().len(), 0);
    assert_eq!(sealed.envelope().tag().len(), TAG_LEN);

    let opened = s.responder_opener.open(sealed.envelope()).expect("opens");
    assert_eq!(opened.payload(), b"");
}

#[test]
fn a_large_payload_round_trips() {
    let mut s = session();
    let payload = vec![0x5A; 64 * 1024];
    let sealed = s.initiator_sealer.seal(&plain(&payload)).expect("seals");
    assert_eq!(
        sealed.envelope().ciphertext().len(),
        payload.len(),
        "ChaCha20 is a stream cipher: ciphertext is the same length as plaintext"
    );
    let opened = s.responder_opener.open(sealed.envelope()).expect("opens");
    assert_eq!(opened.payload(), &payload[..]);
}

#[test]
fn caller_metadata_survives_and_is_authenticated() {
    let mut s = session();
    let frame = Frame::new(MessageType::ItemStart, b"metadata".to_vec())
        .expect("valid")
        .with_identifiers(SessionId::from_u64(0xDEAD), 0x1122, 0x3344, 0x5566)
        .with_sequence(999)
        .with_flags(Flags::END_OF_ITEM.union(Flags::END_OF_TRANSFER))
        .expect("transport flags");

    let sealed = s.initiator_sealer.seal(&frame).expect("seals");
    let opened = s.responder_opener.open(sealed.envelope()).expect("opens");

    assert_eq!(opened.message_type(), MessageType::ItemStart);
    assert_eq!(opened.transfer_id(), 0x1122);
    assert_eq!(opened.stream_id(), 0x3344);
    assert_eq!(opened.item_id(), 0x5566);
    assert!(opened.flags().contains(Flags::END_OF_ITEM));
    assert!(opened.flags().contains(Flags::END_OF_TRANSFER));

    // The two fields the caller does not get to choose are overwritten.
    assert_eq!(
        opened.session_id(),
        s.session_id,
        "the sealer assigns the session id, not the caller"
    );
    assert_eq!(
        opened.sequence(),
        0,
        "the sealer assigns the sequence, not the caller"
    );
}

#[test]
fn a_sealed_frame_survives_the_wire() {
    // Encode, decode through the ordinary decoder, then open. This is the path
    // a transport will take, and it is the only one that proves the envelope
    // the decoder rebuilds is byte-identical to the one that was sealed.
    let mut s = session();
    let sealed = s
        .initiator_sealer
        .seal(&plain(b"through the wire"))
        .expect("seals");
    let bytes = sealed.encode();

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    let decoded = decoder
        .next_frame()
        .expect("not a framing failure")
        .expect("one whole frame");

    let received = match decoded {
        DecodedFrame::Encrypted(envelope) => envelope,
        other => panic!("a sealed frame decodes as an envelope, got {other:?}"),
    };
    assert_eq!(received.encode(), bytes);

    let opened = s.responder_opener.open(&received).expect("opens");
    assert_eq!(opened.payload(), b"through the wire");
}

// ------------------------------------------------------------- tampering

#[test]
fn every_byte_of_the_header_is_authenticated() {
    // The whole 48-byte header is the associated data, so flipping any bit in
    // any field must break the tag. Testing the fields individually would miss
    // a field nobody thought to name.
    let mut s = session();
    let sealed = s
        .initiator_sealer
        .seal(&plain(b"authenticated"))
        .expect("seals");
    let bytes = sealed.encode();

    for index in 0..48 {
        let mut tampered = bytes.clone();
        tampered[index] ^= 0x01;

        let mut decoder = FrameDecoder::new();
        // Some header bytes break framing before the AEAD ever sees them —
        // magic, version, a reserved byte. Those are rejected by the decoder,
        // which is also a rejection. What must never happen is that a tampered
        // byte produces an authenticated frame.
        let opened = match decoder.push(&tampered).and_then(|()| decoder.next_frame()) {
            Ok(Some(DecodedFrame::Encrypted(envelope))) => {
                s.responder_opener.open(&envelope).is_ok()
            }
            _ => false,
        };
        assert!(
            !opened,
            "flipping header byte {index} produced an authenticated frame"
        );
    }
}

#[test]
fn tampering_with_ciphertext_or_tag_fails() {
    let mut s = session();
    let sealed = s
        .initiator_sealer
        .seal(&plain(b"0123456789abcdef"))
        .expect("seals");
    let bytes = sealed.encode();

    // Body is header (48) + ciphertext (16) + tag (16).
    for index in 48..bytes.len() {
        let mut tampered = bytes.clone();
        tampered[index] ^= 0x01;

        let mut decoder = FrameDecoder::new();
        decoder.push(&tampered).expect("within buffer");
        let envelope = match decoder.next_frame() {
            Ok(Some(DecodedFrame::Encrypted(envelope))) => envelope,
            other => panic!("body tampering must not break framing, got {other:?}"),
        };
        assert_eq!(
            s.responder_opener.open(&envelope).err(),
            Some(AeadError::AuthenticationFailed),
            "flipping body byte {index} must not authenticate"
        );
    }
}

#[test]
fn a_frame_from_another_session_is_refused() {
    let mut first = session_with(1);
    let mut second = session_with(2);
    assert_ne!(first.session_id, second.session_id, "fixtures differ");

    let sealed = first
        .initiator_sealer
        .seal(&plain(b"other session"))
        .expect("seals");
    assert_eq!(
        second.responder_opener.open(sealed.envelope()).err(),
        Some(AeadError::WrongSession),
        "a frame naming another session is rejected before the AEAD runs"
    );
}

#[test]
fn a_tag_of_the_wrong_length_is_refused() {
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"payload")).expect("seals");

    for length in [1usize, 15, 17, 32] {
        // Built by hand: an envelope may carry any trailer the wire declared,
        // and only the opener knows this suite requires exactly sixteen.
        let template = plain(b"payload");
        let forged = EncryptedEnvelope::from_plain_frame(
            &template,
            sealed.envelope().ciphertext().to_vec(),
            vec![0u8; length],
        )
        .expect("the envelope accepts any trailer length");

        assert_eq!(
            s.responder_opener.open(&forged).err(),
            Some(AeadError::InvalidTagLength {
                found: length,
                expected: TAG_LEN
            }),
            "{length} bytes is not a ChaCha20-Poly1305 tag"
        );
    }
}

// ---------------------------------------------------------------- nonces

#[test]
fn sequences_are_monotonic_and_assigned_by_the_sealer() {
    let mut s = session();
    for expected in 0..8u64 {
        let sealed = s.initiator_sealer.seal(&plain(b"x")).expect("seals");
        assert_eq!(sealed.envelope().header().sequence(), expected);
        assert_eq!(sealed.nonce()[..4], s.initiator_sealer.nonce_prefix()[..]);
        assert_eq!(
            sealed.nonce()[4..],
            expected.to_be_bytes()[..],
            "the nonce is prefix || sequence, big-endian"
        );
        assert_eq!(sealed.nonce().len(), NONCE_LEN);
    }
}

#[test]
fn no_nonce_is_ever_produced_twice() {
    let mut s = session();
    let mut seen = std::collections::HashSet::new();
    for _ in 0..256 {
        let sealed = s.initiator_sealer.seal(&plain(b"y")).expect("seals");
        assert!(seen.insert(sealed.nonce()), "a nonce was reused");
    }

    // And the other direction never collides with this one, because the prefix
    // differs even though both sequences start at zero.
    let other = s.responder_sealer.seal(&plain(b"y")).expect("seals");
    assert!(!seen.contains(&other.nonce()));
}

#[test]
fn a_discarded_frame_does_not_release_its_sequence() {
    // Dropping a sealed frame must not let the next one reuse its nonce. The
    // counter advances when the frame is produced, not when it is sent.
    let mut s = session();
    let first = s.initiator_sealer.seal(&plain(b"dropped")).expect("seals");
    let first_nonce = first.nonce();
    drop(first);

    let second = s.initiator_sealer.seal(&plain(b"kept")).expect("seals");
    assert_ne!(second.nonce(), first_nonce);
    assert_eq!(second.envelope().header().sequence(), 1);
}

#[test]
fn an_exhausted_sequence_is_a_terminal_error() {
    // u64 does not wrap here. Wrapping would repeat a nonce, which for a stream
    // cipher means the two plaintexts XOR to a known value.
    let mut s = session();
    s.initiator_sealer.set_sequence_for_test(u64::MAX);

    let last = s
        .initiator_sealer
        .seal(&plain(b"final"))
        .expect("the last one seals");
    assert_eq!(last.envelope().header().sequence(), u64::MAX);

    assert_eq!(
        s.initiator_sealer.seal(&plain(b"one too many")).err(),
        Some(AeadError::SequenceExhausted),
        "there is no sequence after u64::MAX"
    );
    // And it stays exhausted rather than recovering.
    assert_eq!(
        s.initiator_sealer.seal(&plain(b"still no")).err(),
        Some(AeadError::SequenceExhausted)
    );
}

// ----------------------------------------------------------------- replay

#[test]
fn an_exact_duplicate_is_rejected() {
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"once")).expect("seals");

    assert!(s.responder_opener.open(sealed.envelope()).is_ok());
    assert_eq!(
        s.responder_opener.open(sealed.envelope()).err(),
        Some(AeadError::ReplayDetected { sequence: 0 }),
        "the same frame must not open twice"
    );
}

#[test]
fn frames_out_of_order_inside_the_window_are_accepted_once_each() {
    let mut s = session();
    let sealed: Vec<_> = (0..5)
        .map(|_| s.initiator_sealer.seal(&plain(b"z")).expect("seals"))
        .collect();

    // Deliver 4, 1, 3, 0, 2 — a reordering a real network produces.
    for index in [4usize, 1, 3, 0, 2] {
        assert!(
            s.responder_opener.open(sealed[index].envelope()).is_ok(),
            "frame {index} arrived out of order but is not a replay"
        );
    }
    // Every one of them is now a duplicate.
    for (index, frame) in sealed.iter().enumerate() {
        assert_eq!(
            s.responder_opener.open(frame.envelope()).err(),
            Some(AeadError::ReplayDetected {
                sequence: index as u64
            })
        );
    }
}

#[test]
fn a_frame_older_than_the_window_is_rejected() {
    let mut s = session();
    let first = s.initiator_sealer.seal(&plain(b"old")).expect("seals");

    // Move the window well past it.
    s.initiator_sealer
        .set_sequence_for_test(REPLAY_WINDOW as u64 + 10);
    let recent = s.initiator_sealer.seal(&plain(b"new")).expect("seals");
    assert!(s.responder_opener.open(recent.envelope()).is_ok());

    assert_eq!(
        s.responder_opener.open(first.envelope()).err(),
        Some(AeadError::SequenceTooOld {
            sequence: 0,
            window: REPLAY_WINDOW as u64
        }),
        "a frame that fell out of the window is refused rather than re-accepted"
    );
}

#[test]
fn a_large_forward_jump_is_accepted_and_resets_the_window() {
    // A gap is not an attack: frames get lost. What must not happen is that the
    // jump leaves stale bits set, letting an old sequence through.
    let mut s = session();
    let early = s.initiator_sealer.seal(&plain(b"a")).expect("seals");
    assert!(s.responder_opener.open(early.envelope()).is_ok());

    s.initiator_sealer.set_sequence_for_test(1_000_000);
    let far = s.initiator_sealer.seal(&plain(b"b")).expect("seals");
    assert!(s.responder_opener.open(far.envelope()).is_ok());
    assert_eq!(
        s.responder_opener.open(far.envelope()).err(),
        Some(AeadError::ReplayDetected {
            sequence: 1_000_000
        })
    );
}

#[test]
fn a_failed_authentication_does_not_move_the_replay_window() {
    // The property that keeps an attacker without the key from burning the
    // window: a frame is only recorded once its tag verifies. Otherwise anyone
    // could send sequence u64::MAX-1 with a garbage tag and lock the session.
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"genuine")).expect("seals");

    let mut forged_bytes = sealed.encode();
    let last = forged_bytes.len() - 1;
    forged_bytes[last] ^= 0xFF;

    let mut decoder = FrameDecoder::new();
    decoder.push(&forged_bytes).expect("within buffer");
    let forged = match decoder.next_frame() {
        Ok(Some(DecodedFrame::Encrypted(envelope))) => envelope,
        other => panic!("expected an envelope, got {other:?}"),
    };

    assert_eq!(
        s.responder_opener.open(&forged).err(),
        Some(AeadError::AuthenticationFailed)
    );
    // The genuine frame with the same sequence still opens.
    assert!(
        s.responder_opener.open(sealed.envelope()).is_ok(),
        "a forged frame must not consume the sequence of a real one"
    );
}

#[test]
fn a_frame_from_another_session_does_not_move_the_replay_window() {
    let mut first = session_with(3);
    let mut second = session_with(4);

    let foreign = first
        .initiator_sealer
        .seal(&plain(b"foreign"))
        .expect("seals");
    assert_eq!(
        second.responder_opener.open(foreign.envelope()).err(),
        Some(AeadError::WrongSession)
    );

    // Sequence 0 of the real session still works.
    let genuine = second
        .initiator_sealer
        .seal(&plain(b"genuine"))
        .expect("seals");
    assert!(second.responder_opener.open(genuine.envelope()).is_ok());
}

#[test]
fn replaying_one_direction_does_not_affect_the_other() {
    let mut s = session();
    let outbound = s.initiator_sealer.seal(&plain(b"out")).expect("seals");
    let inbound = s.responder_sealer.seal(&plain(b"in")).expect("seals");

    assert!(s.responder_opener.open(outbound.envelope()).is_ok());
    assert_eq!(
        s.responder_opener.open(outbound.envelope()).err(),
        Some(AeadError::ReplayDetected { sequence: 0 })
    );

    // The other direction's window is untouched, even though the sequence is
    // also zero.
    assert!(s.initiator_opener.open(inbound.envelope()).is_ok());
}

// ------------------------------------------------------------- derivation

fn keys_for(
    secret: &[u8; TRAFFIC_SECRET_LEN],
    direction: Direction,
    transcript: &[u8; AUTH_TRANSCRIPT_LEN],
    session: SessionId,
) -> DirectionalKeys {
    DirectionalKeys::derive(secret, &direction, transcript, session).expect("derives")
}

#[test]
fn the_direction_is_inside_the_label_not_only_inside_the_secret() {
    // Deleting `direction.label()` from `info_for` breaks nothing end to end:
    // the two traffic secrets already differ, because the handshake schedule
    // derived them under separate labels of its own. Checked by removing it and
    // watching all thirty-three tests still pass.
    //
    // So the property ADR-0022 actually states — the two directions cannot
    // produce the same key *even from the same secret* — has to be checked here
    // or not at all, and without it the only thing separating the directions
    // would be one layer up.
    let secret = [0x42u8; TRAFFIC_SECRET_LEN];
    let transcript = [0x17u8; AUTH_TRANSCRIPT_LEN];
    let session = SessionId::from_u64(0x0102_0304_0506_0708);

    let i2r = keys_for(
        &secret,
        Direction::InitiatorToResponder,
        &transcript,
        session,
    );
    let r2i = keys_for(
        &secret,
        Direction::ResponderToInitiator,
        &transcript,
        session,
    );

    assert_ne!(*i2r.key, *r2i.key, "one secret, two directions, two keys");
    assert_ne!(
        i2r.nonce_prefix, r2i.nonce_prefix,
        "and two nonce prefixes, so the shared sequence space is not a shared nonce space"
    );
}

#[test]
fn the_session_and_the_transcript_bind_every_derived_value() {
    // Both are in every `info` so that two sessions derive different keys even
    // if some future defect ever repeated a traffic secret. Same reasoning as
    // above: removing either one from `info_for` passes the end-to-end tests,
    // because two fixtures differ in everything at once.
    let secret = [0x42u8; TRAFFIC_SECRET_LEN];
    let transcript = [0x17u8; AUTH_TRANSCRIPT_LEN];
    let session = SessionId::from_u64(1);

    let base = keys_for(
        &secret,
        Direction::InitiatorToResponder,
        &transcript,
        session,
    );
    let other_session = keys_for(
        &secret,
        Direction::InitiatorToResponder,
        &transcript,
        SessionId::from_u64(2),
    );
    let other_transcript = keys_for(
        &secret,
        Direction::InitiatorToResponder,
        &[0x18u8; AUTH_TRANSCRIPT_LEN],
        session,
    );

    assert_ne!(
        *base.key, *other_session.key,
        "the session id binds the key"
    );
    assert_ne!(
        *base.key, *other_transcript.key,
        "the transcript binds it too"
    );
    assert_ne!(base.nonce_prefix, other_session.nonce_prefix);
    assert_ne!(base.nonce_prefix, other_transcript.nonce_prefix);
}

#[test]
fn the_nonce_prefix_is_not_a_slice_of_the_key() {
    // Separate labels, separate expansions. A prefix cut from the key would leak
    // four key bytes into every nonce, and a nonce is not a secret.
    let keys = keys_for(
        &[0x42u8; TRAFFIC_SECRET_LEN],
        Direction::InitiatorToResponder,
        &[0x17u8; AUTH_TRANSCRIPT_LEN],
        SessionId::from_u64(7),
    );
    assert_ne!(keys.nonce_prefix[..], keys.key[..keys.nonce_prefix.len()]);
    assert_ne!(
        keys.nonce_prefix[..],
        keys.key[keys.key.len() - keys.nonce_prefix.len()..]
    );
}

#[test]
fn the_derivation_labels_are_the_ones_the_adr_freezes() {
    // Pinned against the ADR rather than against the code that produces them: a
    // silent relabelling is a silent change of every key, and an implementation
    // in another language reads these strings, not this function.
    let transcript = [0xABu8; AUTH_TRANSCRIPT_LEN];
    let session = SessionId::from_u64(0x1122_3344_5566_7788);

    for (direction, purpose, label) in [
        (
            Direction::InitiatorToResponder,
            PURPOSE_KEY,
            &b"QYRO-AEAD-V1/i2r/key"[..],
        ),
        (
            Direction::InitiatorToResponder,
            PURPOSE_NONCE_PREFIX,
            &b"QYRO-AEAD-V1/i2r/nonce-prefix"[..],
        ),
        (
            Direction::ResponderToInitiator,
            PURPOSE_KEY,
            &b"QYRO-AEAD-V1/r2i/key"[..],
        ),
        (
            Direction::ResponderToInitiator,
            PURPOSE_NONCE_PREFIX,
            &b"QYRO-AEAD-V1/r2i/nonce-prefix"[..],
        ),
    ] {
        let info = info_for(&direction, purpose, &transcript, session);
        let expected: Vec<u8> = label
            .iter()
            .copied()
            .chain(core::iter::once(0x00))
            .chain(transcript)
            .chain(session.to_be_bytes())
            .collect();
        assert_eq!(
            info,
            expected,
            "info for {} is label || 0x00 || transcript || session",
            String::from_utf8_lossy(label)
        );
    }
}

// ---------------------------------------------------------------- secrets

#[test]
fn no_key_material_is_reachable_or_printable() {
    let s = session();

    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is readable");
    assert!(
        !lib.contains("AeadKey") && !lib.contains("SessionKey"),
        "no key type may be exported from the crate root"
    );

    for rendered in [
        format!("{:?}", s.initiator_sealer),
        format!("{:?}", s.responder_opener),
    ] {
        assert!(rendered.contains("redacted"), "secrets must be marked");
    }

    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/aead/mod.rs"))
        .expect("mod.rs is readable");
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "pub fn key(",
        "pub const fn key(",
        "derive(Clone",
        "Serialize",
    ] {
        assert!(
            !code.contains(forbidden),
            "the AEAD module must not contain {forbidden}"
        );
    }
}

// --------------------------------------------------------------- robustness

#[test]
fn arbitrary_bytes_never_panic_the_opener() {
    let mut s = session();
    let sealed = s.initiator_sealer.seal(&plain(b"baseline")).expect("seals");
    let good = sealed.encode();

    let mut cases: Vec<Vec<u8>> = vec![Vec::new(), vec![0u8; 47], vec![0xFF; 200]];
    for cut in [0usize, 1, 47, 48, 49, 63, 64, 79] {
        cases.push(good[..cut.min(good.len())].to_vec());
    }
    for pad in [1usize, 17, 64] {
        let mut longer = good.clone();
        longer.extend(std::iter::repeat_n(0xAB, pad));
        cases.push(longer);
    }

    for bytes in cases {
        let mut decoder = FrameDecoder::new();
        // Whatever happens, it is a Result — never a panic and never plaintext.
        if let Ok(Some(DecodedFrame::Encrypted(envelope))) =
            decoder.push(&bytes).and_then(|()| decoder.next_frame())
        {
            let _ = s.responder_opener.open(&envelope);
        }
    }
}
