//! Contracts for the authenticated handshake.
//!
//! In-crate for the same reason the identity vectors are: the deterministic
//! constructors these tests need are `cfg(test)` and crate-private, and an
//! integration test is a separate crate that could only reach them through
//! public API.

use super::{
    CRYPTO_SUITE_ID, EstablishedInitiator, EstablishedResponder, FINISHED_MAC_LEN,
    HANDSHAKE_VERSION, HandshakeError, INITIATOR_FINISH_LEN, INITIATOR_HELLO_LEN, InitiatorStart,
    NONCE_LEN, RESPONDER_FINISH_LEN, RESPONDER_HELLO_LEN, ResponderStart, X25519_PUBLIC_LEN,
};
use crate::identity::{DeviceIdentity, PUBLIC_IDENTITY_WIRE_LEN};

/// Distinct fixed seeds. TEST ONLY — NEVER PRODUCTION.
fn identities() -> (DeviceIdentity, DeviceIdentity) {
    (
        DeviceIdentity::from_test_seed(&[0x11; 32]),
        DeviceIdentity::from_test_seed(&[0x22; 32]),
    )
}

/// Fixed handshake entropy so a run is reproducible. TEST ONLY.
fn entropy(tag: u8) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, slot) in out.iter_mut().enumerate() {
        // Varying, non-zero, and different per tag: an all-equal buffer would
        // make a byte-order mistake invisible.
        *slot = tag ^ (index as u8).wrapping_mul(7).wrapping_add(1);
    }
    out
}

struct Completed {
    initiator: EstablishedInitiator,
    responder: EstablishedResponder,
}

/// Runs a whole handshake with fixed entropy.
fn run() -> Completed {
    let (alice, bob) = identities();

    let (hello, awaiting_responder) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(0xA0))
        .expect("the initiator can open");

    let (responder_hello, awaiting_initiator_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(0xB0))
        .expect("the responder accepts a well-formed hello");

    let (initiator_finish, awaiting_responder_finish) = awaiting_responder
        .receive_responder_hello(&responder_hello)
        .expect("the responder hello verifies");

    let pending = awaiting_initiator_finish
        .receive_initiator_finish(&initiator_finish)
        .expect("the initiator finish verifies");
    let responder_finish = *pending.encoded_finish();
    // No transport exists; the tests stand in for one by confirming delivery.
    let responder = pending.confirm_sent();

    let initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .expect("the responder finish verifies");

    Completed {
        initiator,
        responder,
    }
}

// ------------------------------------------------------------- the happy path

#[test]
fn both_sides_agree_on_every_derived_value() {
    let done = run();

    assert_eq!(
        done.initiator.session_id(),
        done.responder.session_id(),
        "a session both sides believe in must have one identifier"
    );

    // Directional keys must cross over: what one writes is what the other reads.
    assert_eq!(
        done.initiator.sending_key_bytes(),
        done.responder.receiving_key_bytes(),
        "the initiator writes where the responder reads"
    );
    assert_eq!(
        done.responder.sending_key_bytes(),
        done.initiator.receiving_key_bytes(),
        "the responder writes where the initiator reads"
    );

    // The transcript is not a key, but every AEAD key is derived under it, so
    // two sides that disagreed here would agree on traffic secrets and still be
    // unable to read each other's frames.
    assert_eq!(
        done.initiator.auth_transcript_for_test(),
        done.responder.auth_transcript_for_test(),
        "the transcript the frame keys bind to must be one transcript"
    );
}

#[test]
fn the_two_directions_never_share_a_key() {
    let done = run();
    assert_ne!(
        done.initiator.sending_key_bytes(),
        done.initiator.receiving_key_bytes(),
        "one key in both directions lets a peer's own messages be reflected at it"
    );
    assert_ne!(
        &done.initiator.sending_key_bytes()[..8],
        &done.initiator.session_id().to_be_bytes()[..],
        "the session identifier is not a prefix of a key"
    );
}

#[test]
fn each_side_learns_the_others_identity() {
    let (alice, bob) = identities();
    let done = run();

    assert_eq!(
        done.initiator.peer_identity().fingerprint(),
        bob.fingerprint(),
        "the initiator must end up holding the responder's real identity"
    );
    assert_eq!(
        done.responder.peer_identity().fingerprint(),
        alice.fingerprint()
    );
}

#[test]
fn a_different_run_produces_different_keys() {
    // Same identities, different ephemeral entropy. Long-term keys must not
    // determine session keys, or one compromised session compromises all of
    // them.
    let (alice, bob) = identities();

    let mut sessions = Vec::new();
    for tag in [0u8, 1] {
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
        let _responder = pending.confirm_sent();
        let initiator = awaiting_responder_finish
            .receive_responder_finish(&responder_finish)
            .expect("verifies");
        sessions.push(initiator);
    }

    assert_ne!(sessions[0].session_id(), sessions[1].session_id());
    assert_ne!(
        sessions[0].sending_key_bytes(),
        sessions[1].sending_key_bytes()
    );
}

// ------------------------------------------------------------- message shapes

#[test]
fn every_message_has_the_length_the_adr_freezes() {
    let (alice, bob) = identities();

    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    assert_eq!(hello.len(), INITIATOR_HELLO_LEN);
    assert_eq!(
        INITIATOR_HELLO_LEN,
        3 + X25519_PUBLIC_LEN + NONCE_LEN + PUBLIC_IDENTITY_WIRE_LEN
    );

    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(2))
        .expect("accepts");
    assert_eq!(responder_hello.len(), RESPONDER_HELLO_LEN);

    let (finish, _) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    assert_eq!(finish.len(), INITIATOR_FINISH_LEN);

    let responder_finish = *awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies")
        .encoded_finish();
    assert_eq!(responder_finish.len(), RESPONDER_FINISH_LEN);
    assert_eq!(RESPONDER_FINISH_LEN, 3 + FINISHED_MAC_LEN);
}

#[test]
fn every_message_declares_its_version_suite_and_type() {
    let (alice, bob) = identities();
    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(2))
        .expect("accepts");
    let (finish, _) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    let responder_finish = *awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies")
        .encoded_finish();

    for (expected_type, message) in [
        (1u8, &hello[..]),
        (2, &responder_hello[..]),
        (3, &finish[..]),
        (4, &responder_finish[..]),
    ] {
        assert_eq!(message[0], HANDSHAKE_VERSION);
        assert_eq!(message[1], CRYPTO_SUITE_ID);
        assert_eq!(
            message[2], expected_type,
            "the type byte is inside the transcript and is what makes it role-asymmetric"
        );
    }
}

#[test]
fn a_message_of_the_wrong_length_is_refused() {
    let (alice, bob) = identities();
    let (hello, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    for length in [0usize, 1, INITIATOR_HELLO_LEN - 1] {
        let mut truncated = hello.to_vec();
        truncated.resize(length, 0);
        assert_eq!(
            ResponderStart::new(&bob)
                .receive_initiator_hello(&truncated, entropy(2))
                .err(),
            Some(HandshakeError::InvalidMessageLength {
                found: length,
                expected: INITIATOR_HELLO_LEN
            }),
            "{length} bytes is not an InitiatorHello"
        );
    }

    // Too long is a different failure from too short, and says so. A single
    // length comparison reported both as InvalidMessageLength and left
    // TrailingBytes as a variant nothing could ever produce.
    for extra in [1usize, 7, 64] {
        let mut padded = hello.to_vec();
        padded.resize(INITIATOR_HELLO_LEN + extra, 0);
        assert_eq!(
            ResponderStart::new(&bob)
                .receive_initiator_hello(&padded, entropy(2))
                .err(),
            Some(HandshakeError::TrailingBytes { extra }),
            "{extra} bytes past an InitiatorHello must be named as trailing"
        );
    }
}

#[test]
fn a_foreign_version_or_suite_is_refused_not_downgraded() {
    let (alice, bob) = identities();
    let (hello, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    let mut other_version = hello;
    other_version[0] = HANDSHAKE_VERSION + 1;
    assert_eq!(
        ResponderStart::new(&bob)
            .receive_initiator_hello(&other_version, entropy(2))
            .err(),
        Some(HandshakeError::UnsupportedHandshakeVersion {
            found: HANDSHAKE_VERSION + 1,
            supported: HANDSHAKE_VERSION
        })
    );

    let mut other_suite = hello;
    other_suite[1] = CRYPTO_SUITE_ID + 1;
    assert_eq!(
        ResponderStart::new(&bob)
            .receive_initiator_hello(&other_suite, entropy(2))
            .err(),
        Some(HandshakeError::UnsupportedCryptoSuite {
            found: CRYPTO_SUITE_ID + 1,
            supported: CRYPTO_SUITE_ID
        })
    );
}

#[test]
fn a_message_of_the_wrong_kind_is_refused() {
    // Right length is not enough: a peer replaying the wrong message must be
    // told so, not silently parsed as whatever was expected.
    let (alice, bob) = identities();
    let (hello, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    let mut wrong_kind = hello;
    wrong_kind[2] = 2; // claims to be a ResponderHello
    assert_eq!(
        ResponderStart::new(&bob)
            .receive_initiator_hello(&wrong_kind, entropy(2))
            .err(),
        Some(HandshakeError::UnexpectedMessage {
            found: 2,
            expected: 1
        })
    );
}

// --------------------------------------------------------- authentication bites

#[test]
fn a_tampered_responder_hello_is_refused() {
    let (alice, bob) = identities();

    // Every byte of the responder hello is either signed or inside the
    // transcript the signature covers, so flipping any of them must fail.
    for index in [3usize, 40, 70, 99, 100, 163] {
        let (hello, awaiting) = InitiatorStart::new(&alice)
            .send_hello_with_entropy(entropy(1))
            .expect("opens");
        let (mut responder_hello, _) = ResponderStart::new(&bob)
            .receive_initiator_hello(&hello, entropy(2))
            .expect("accepts");

        responder_hello[index] ^= 0x01;
        let result = awaiting.receive_responder_hello(&responder_hello);
        assert!(
            result.is_err(),
            "flipping byte {index} of the responder hello must not verify"
        );
    }
}

#[test]
fn a_tampered_initiator_finish_is_refused() {
    let (alice, bob) = identities();

    for index in [3usize, 66, 67, 98] {
        let (hello, awaiting) = InitiatorStart::new(&alice)
            .send_hello_with_entropy(entropy(1))
            .expect("opens");
        let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
            .receive_initiator_hello(&hello, entropy(2))
            .expect("accepts");
        let (mut finish, _) = awaiting
            .receive_responder_hello(&responder_hello)
            .expect("verifies");

        finish[index] ^= 0x01;
        assert!(
            awaiting_finish.receive_initiator_finish(&finish).is_err(),
            "flipping byte {index} of the initiator finish must not verify"
        );
    }
}

#[test]
fn a_tampered_responder_finish_is_refused() {
    let (alice, bob) = identities();
    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(2))
        .expect("accepts");
    let (finish, awaiting_responder_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    let mut responder_finish = *awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies")
        .encoded_finish();

    responder_finish[RESPONDER_FINISH_LEN - 1] ^= 0x01;
    assert_eq!(
        awaiting_responder_finish
            .receive_responder_finish(&responder_finish)
            .err(),
        Some(HandshakeError::FinishedVerificationFailed)
    );
}

#[test]
fn an_impostor_cannot_answer_a_hello_it_did_not_receive() {
    // Mallory signs a transcript built from Alice's hello but with her own
    // identity in it. The signature is valid for Mallory, so this is not a
    // signature test: what must stop it is that Alice sees an identity she was
    // not talking to. Alice has no prior knowledge of Bob here, so the handshake
    // completes and Alice *learns* it was Mallory — the authentication is that
    // the identity is bound to the transcript, not that it is the one she hoped
    // for. Trust decisions come later and are explicit.
    let (alice, _bob) = identities();
    let mallory = DeviceIdentity::from_test_seed(&[0x33; 32]);

    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    let (responder_hello, _) = ResponderStart::new(&mallory)
        .receive_initiator_hello(&hello, entropy(2))
        .expect("accepts");

    let (_finish, awaiting_responder_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("a valid handshake with the wrong peer still completes");

    assert_eq!(
        awaiting_responder_finish.peer_identity().fingerprint(),
        mallory.fingerprint(),
        "the peer identity must be whoever actually signed, never who was hoped for"
    );
}

#[test]
fn the_initiator_signs_over_the_responder_signature() {
    // ADR-0021 has the initiator sign `base_transcript || responder_signature`
    // rather than the base transcript alone, so its signature is bound to this
    // particular answer.
    //
    // The honest limit of that claim, found by deleting the binding and
    // watching every end-to-end test still pass: Ed25519 here is deterministic
    // and the responder identity is inside `base_transcript`, so the responder
    // signature is a pure function of the base transcript. Same transcript,
    // same signature, always — the binding adds nothing the base transcript
    // does not already provide, and no end-to-end attack distinguishes the two
    // constructions.
    //
    // It is kept as defence in depth for a signer that is not deterministic —
    // a hardware token, or a future variant that randomises — where several
    // valid signatures exist over one transcript. What can be tested is the
    // construction itself: the responder signature must actually be part of
    // what the initiator signs.
    let base = [0x5A; 32];
    let signature_one = [0x01; 64];
    let signature_two = [0x02; 64];

    let message_one = super::initiator_signing_message(&base, &signature_one);
    let message_two = super::initiator_signing_message(&base, &signature_two);

    assert_ne!(
        message_one, message_two,
        "the responder signature must change what the initiator signs"
    );
    assert_eq!(&message_one[..32], &base, "the base transcript comes first");
    assert_eq!(
        &message_one[32..],
        &signature_one,
        "the responder signature follows it"
    );

    // And the two roles sign different lengths, which is what keeps a responder
    // signature from ever verifying as an initiator one: the identity signing
    // input carries the message length, and 32 is never 96.
    assert_eq!(super::responder_signing_message(&base).len(), 32);
    assert_eq!(message_one.len(), 96);
}

#[test]
fn a_responder_signature_cannot_be_replayed_into_another_session() {
    let (alice, bob) = identities();

    let (hello_one, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    let (responder_hello_one, _) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello_one, entropy(2))
        .expect("accepts");

    // A second session with different entropy.
    let (hello_two, awaiting_two) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(3))
        .expect("opens");
    let (mut responder_hello_two, _) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello_two, entropy(4))
        .expect("accepts");

    // Graft session one's signature onto session two's hello.
    responder_hello_two[RESPONDER_HELLO_LEN - 64..]
        .copy_from_slice(&responder_hello_one[RESPONDER_HELLO_LEN - 64..]);

    assert_eq!(
        awaiting_two
            .receive_responder_hello(&responder_hello_two)
            .err(),
        Some(HandshakeError::SignatureVerificationFailed),
        "a signature is bound to its own transcript, not to the responder alone"
    );
}

// --------------------------------------------------------------- rejected keys

/// X25519 public keys that force a non-contributory exchange.
///
/// Determined by probe, not from memory: every candidate was run through
/// `diffie_hellman` against four different local secrets and kept only if
/// `was_contributory()` was false for all of them. Five further encodings that
/// circulate in low-order blocklists turned out to be perfectly contributory in
/// this implementation and were dropped rather than asserted.
const LOW_ORDER_X25519: [[u8; 32]; 7] = [
    [0x00; 32],
    [
        0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0,
    ],
    [
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ],
    [
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ],
    [
        0xec, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ],
    [
        0xed, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ],
    [
        0xee, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff,
    ],
];

#[test]
fn a_low_order_ephemeral_key_is_refused() {
    // A peer sending one of these forces an all-zero shared secret: both sides
    // "agree" without either having contributed anything.
    let (alice, bob) = identities();
    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    for encoding in LOW_ORDER_X25519 {
        let mut poisoned = hello;
        poisoned[3..3 + X25519_PUBLIC_LEN].copy_from_slice(&encoding);

        assert_eq!(
            ResponderStart::new(&bob)
                .receive_initiator_hello(&poisoned, entropy(2))
                .err(),
            Some(HandshakeError::NonContributorySharedSecret),
            "a non-contributory exchange is not an exchange"
        );
    }

    // And in the other direction. The initiator verifies the responder's
    // signature before exchanging, so the responder hello has to be re-signed
    // for the exchange to be reached at all — which is the point: a tampered
    // key is caught by the signature first, and only a genuinely low-order key
    // from an authenticated peer reaches the contributory check.
    let (mut responder_hello, _) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(2))
        .expect("accepts");
    responder_hello[3..3 + X25519_PUBLIC_LEN].copy_from_slice(&LOW_ORDER_X25519[0]);
    assert_eq!(
        awaiting.receive_responder_hello(&responder_hello).err(),
        Some(HandshakeError::SignatureVerificationFailed),
        "swapping the ephemeral key breaks the signature over the transcript"
    );
}

// ------------------------------------------------------- HMAC-SHA-256 vectors

#[test]
fn the_hmac_implementation_matches_every_rfc4231_vector() {
    // The confirmation MACs are only meaningful if the HMAC underneath them is
    // the standard one. RFC 2104 hashes a key longer than the block size rather
    // than truncating it, and cases 6 and 7 use a 131-byte key, which is where a
    // non-conforming implementation diverges.
    let parsed: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../docs/security/test-vectors/rfc4231-hmac-sha256.json"
    ))
    .expect("the KAT file is valid JSON");
    let cases = parsed["vectors"].as_array().expect("vectors is an array");
    assert_eq!(
        cases.len(),
        7,
        "RFC 4231 section 4 defines seven test cases"
    );

    for case in cases {
        let name = case["name"].as_str().expect("name");
        let key = unhex(case["key"].as_str().expect("key"));
        let data = unhex(case["data"].as_str().expect("data"));
        let expected = case["hmac_sha256"].as_str().expect("hmac_sha256");
        let width = case["truncated_to_bytes"].as_u64().expect("width") as usize;

        let produced = super::schedule::hmac_sha256(&key, &data);
        let rendered: String = produced[..width]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        assert_eq!(
            rendered, expected,
            "{name}: HMAC-SHA-256 must match RFC 4231"
        );
    }
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex digit"))
        .collect()
}

#[test]
fn a_low_order_identity_key_is_refused() {
    let (alice, bob) = identities();
    let (mut hello, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    // Byte 67 is the version, 68..100 the Ed25519 key. All-zeros is a valid
    // encoding of a small-order point.
    hello[68..100].copy_from_slice(&[0u8; 32]);
    assert_eq!(
        ResponderStart::new(&bob)
            .receive_initiator_hello(&hello, entropy(2))
            .err(),
        Some(HandshakeError::WeakPublicIdentity)
    );
}

#[test]
fn a_malformed_identity_in_a_hello_is_refused() {
    let (alice, bob) = identities();
    let (mut hello, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");

    hello[67] = 99; // an identity version this build does not implement
    assert_eq!(
        ResponderStart::new(&bob)
            .receive_initiator_hello(&hello, entropy(2))
            .err(),
        Some(HandshakeError::InvalidPublicIdentity)
    );
}

// ----------------------------------------------------------- secret handling

#[test]
fn debug_never_prints_key_material() {
    let done = run();

    for rendered in [
        format!("{:?}", done.initiator),
        format!("{:?}", done.responder),
    ] {
        for key in [
            done.initiator.sending_key_bytes(),
            done.initiator.receiving_key_bytes(),
        ] {
            let as_hex: String = key.iter().map(|byte| format!("{byte:02x}")).collect();
            assert!(
                !rendered.contains(&as_hex),
                "Debug printed a whole session key"
            );
            for window in as_hex.as_bytes().chunks(16) {
                let fragment = String::from_utf8_lossy(window);
                assert!(
                    !rendered.contains(fragment.as_ref()),
                    "Debug leaked a fragment of a session key"
                );
            }
        }
        assert!(rendered.contains("redacted"));
    }
}

// ---------------------------------------------------- sprint 4C.2 (QYR-0022)

#[test]
fn an_unsigned_peer_cannot_present_another_identity() {
    // QYR-0022. `receive_initiator_finish` is the only place that authenticates
    // the initiator, and deleting its `verify_transcript` call left
    // `cargo test --package qyro_crypto` at 124 passed, 0 failed. The mirror
    // control on the initiator's side is covered by three tests; this one had
    // none, so the property "the peer that finishes is the peer whose identity
    // is in the hello" was documented and not held.
    //
    // Nothing in an `InitiatorHello` is signed — it cannot be, since the
    // transcript it would sign does not exist until the responder answers. So
    // anyone can put anyone's `PublicIdentity` in one. What must be impossible
    // is *finishing*: that requires a signature over the transcript under the
    // key that identity names.
    use super::{
        EphemeralKeyPair, PREFIX_LEN, TYPE_INITIATOR_FINISH, TYPE_INITIATOR_HELLO,
        write_hello_unsigned,
    };
    use crate::signature::SIGNATURE_LEN;

    let responder = DeviceIdentity::from_test_seed(&[0x22; 32]);
    let victim = DeviceIdentity::from_test_seed(&[0x33; 32]);
    let attacker = DeviceIdentity::from_test_seed(&[0x44; 32]);

    // The attacker's own ephemeral key and nonce, and somebody else's identity.
    let ephemeral = EphemeralKeyPair::from_secret_bytes([0x55; 32]);
    let hello = write_hello_unsigned(
        TYPE_INITIATOR_HELLO,
        ephemeral.public(),
        &[0x66; NONCE_LEN],
        victim.public_identity(),
    );

    let (_responder_hello, awaiting) = ResponderStart::new(&responder)
        .receive_initiator_hello(&hello, entropy(0xB0))
        .expect("the hello is well formed, and nothing in it is signed yet");

    // Arbitrary bytes where the signature belongs, and arbitrary bytes where
    // the confirmation MAC belongs. The signature is checked first, so the MAC
    // never gets a say.
    let mut finish = [0u8; INITIATOR_FINISH_LEN];
    finish[0] = HANDSHAKE_VERSION;
    finish[1] = CRYPTO_SUITE_ID;
    finish[2] = TYPE_INITIATOR_FINISH;
    finish[PREFIX_LEN..PREFIX_LEN + SIGNATURE_LEN].copy_from_slice(&[0x5A; SIGNATURE_LEN]);
    finish[PREFIX_LEN + SIGNATURE_LEN..].copy_from_slice(&[0x6B; FINISHED_MAC_LEN]);

    assert_eq!(
        awaiting.receive_initiator_finish(&finish).err(),
        Some(HandshakeError::SignatureVerificationFailed),
        "a peer that cannot sign for the identity it presented must be refused \
         as a signature failure, before any key is derived"
    );

    // And it is not merely that *those* bytes are wrong: a real signature, made
    // correctly by the attacker's own key, is refused just the same, because
    // the identity in the hello is not the attacker's.
    let (_responder_hello, awaiting) = ResponderStart::new(&responder)
        .receive_initiator_hello(&hello, entropy(0xB0))
        .expect("same hello, same answer");

    let genuine = attacker
        .try_sign(
            crate::signature::SignatureDomain::HandshakeTranscript,
            &[0x77; 96],
        )
        .expect("the attacker can sign for itself");
    let mut finish = [0u8; INITIATOR_FINISH_LEN];
    finish[0] = HANDSHAKE_VERSION;
    finish[1] = CRYPTO_SUITE_ID;
    finish[2] = TYPE_INITIATOR_FINISH;
    finish[PREFIX_LEN..PREFIX_LEN + SIGNATURE_LEN].copy_from_slice(genuine.as_bytes());
    finish[PREFIX_LEN + SIGNATURE_LEN..].copy_from_slice(&[0x6B; FINISHED_MAC_LEN]);

    assert_eq!(
        awaiting.receive_initiator_finish(&finish).err(),
        Some(HandshakeError::SignatureVerificationFailed),
        "signing correctly with the wrong key is still the wrong key"
    );
}
