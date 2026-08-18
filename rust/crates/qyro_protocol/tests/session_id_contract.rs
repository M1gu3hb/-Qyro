//! Contracts on the canonical session identifier.
//!
//! QYRO/1 reserves exactly eight bytes for `session_id`, and the header stored
//! them as a bare `u64`. The handshake in `qyro_crypto` derived a *32-byte*
//! session identifier from its key schedule. Two different sizes, two different
//! types, and no mapping between them — so the only ways to put a handshake's
//! identifier on the wire were to truncate it or to invent a conversion at the
//! call site, and both decisions would have been made by whoever wired the
//! transport up, silently, long after the format was frozen.
//!
//! One type, eight bytes, defined once.

use qyro_protocol::{Frame, FrameDecoder, MessageType, SESSION_ID_LEN, SessionId};

#[test]
fn a_session_id_is_exactly_eight_bytes() {
    assert_eq!(SESSION_ID_LEN, 8, "the QYRO/1 header reserves eight bytes");
    assert_eq!(size_of::<SessionId>(), SESSION_ID_LEN);
    assert_eq!(SessionId::from_u64(0).to_be_bytes().len(), SESSION_ID_LEN);
}

#[test]
fn the_u64_view_is_big_endian_and_matches_the_wire() {
    // Endianness is fixed by the format, not by the host. A `to_ne_bytes`
    // anywhere in this path would pass every test on x86 and corrupt every
    // session on a big-endian peer.
    let id = SessionId::from_u64(0x0102_0304_0506_0708);
    assert_eq!(
        id.to_be_bytes(),
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
    assert_eq!(SessionId::from_be_bytes(id.to_be_bytes()), id);
    assert_eq!(id.to_u64(), 0x0102_0304_0506_0708);
    assert_eq!(SessionId::from_u64(id.to_u64()), id);
}

#[test]
fn a_session_id_round_trips_through_a_frame_header() {
    let id = SessionId::from_be_bytes([0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
    let frame = Frame::new(MessageType::Hello, b"body".to_vec())
        .expect("valid")
        .with_identifiers(id, 7, 8, 9);

    assert_eq!(frame.header().session_id(), id);

    let bytes = frame.encode();
    assert_eq!(
        &bytes[16..24],
        &id.to_be_bytes(),
        "bytes 16..24 of the header are the session id, unchanged"
    );

    let mut decoder = FrameDecoder::new();
    decoder.push(&bytes).expect("within buffer");
    let decoded = decoder
        .next_frame()
        .expect("not a framing failure")
        .expect("one whole frame")
        .as_plain()
        .cloned()
        .expect("plain frame");

    assert_eq!(decoded.header().session_id(), id);
    assert_eq!(decoded.encode(), bytes);
}

#[test]
fn a_session_id_debug_shows_the_value_without_inventing_one() {
    // The identifier is public by design: it correlates frames, it is not a
    // secret. Printing it must still be unambiguous.
    let id = SessionId::from_be_bytes([0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
    let rendered = format!("{id:?}");
    assert!(
        rendered.contains("0011223344556677"),
        "Debug must render the eight bytes, got {rendered}"
    );
}

#[test]
fn distinct_values_stay_distinct() {
    let a = SessionId::from_u64(1);
    let b = SessionId::from_u64(2);
    assert_ne!(a, b);
    assert!(a < b, "Ord follows the big-endian value");

    let mut sorted = [b, a];
    sorted.sort_unstable();
    assert_eq!(sorted, [a, b]);
}
