//! Contracts for the sprint-4B.1 closure findings.
//!
//! Each one names a way the handshake was easier to misuse than it looked. They
//! live in-crate for the same reason the rest do: the deterministic
//! constructors they need are `cfg(test)` and crate-private.

use qyro_protocol::{SESSION_ID_LEN, SessionId};

use super::{
    EstablishedInitiator, EstablishedResponder, HandshakeError, InitiatorStart,
    ResponderFinishPending, ResponderStart,
};
use crate::identity::DeviceIdentity;

fn identities() -> (DeviceIdentity, DeviceIdentity) {
    (
        DeviceIdentity::from_test_seed(&[0x11; 32]),
        DeviceIdentity::from_test_seed(&[0x22; 32]),
    )
}

fn entropy(tag: u8) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = tag ^ (index as u8).wrapping_mul(7).wrapping_add(1);
    }
    out
}

/// Drives a handshake up to the point where the responder holds its pending
/// state, without confirming delivery.
fn up_to_pending() -> (
    super::InitiatorAwaitResponderFinish,
    ResponderFinishPending,
    [u8; super::RESPONDER_FINISH_LEN],
) {
    let (alice, bob) = identities();

    let (hello, awaiting_responder) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(0xA0))
        .expect("opens");
    let (responder_hello, awaiting_initiator_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello, entropy(0xB0))
        .expect("accepts");
    let (initiator_finish, awaiting_responder_finish) = awaiting_responder
        .receive_responder_hello(&responder_hello)
        .expect("verifies");

    let pending = awaiting_initiator_finish
        .receive_initiator_finish(&initiator_finish)
        .expect("verifies");
    let finish = *pending.encoded_finish();

    (awaiting_responder_finish, pending, finish)
}

fn completed() -> (EstablishedInitiator, EstablishedResponder) {
    let (awaiting_responder_finish, pending, finish) = up_to_pending();
    let responder = pending.confirm_sent();
    let initiator = awaiting_responder_finish
        .receive_responder_finish(&finish)
        .expect("verifies");
    (initiator, responder)
}

// ------------------------------------------------- 3.1 one session identifier

#[test]
fn the_handshake_derives_the_wire_session_identifier() {
    // The schedule used to emit 32 bytes under the `session-id` label while the
    // header carried eight. Nothing converted between them, so whoever wired
    // the transport up would have had to choose a truncation — a decision about
    // a frozen wire format, made at a call site, by someone not writing an ADR.
    let (initiator, responder) = completed();

    let id: SessionId = initiator.session_id();
    assert_eq!(
        id,
        responder.session_id(),
        "both peers derive one identifier"
    );
    assert_eq!(id.to_be_bytes().len(), SESSION_ID_LEN);

    // Derived, not defaulted. An all-zero identifier would compare equal across
    // every session and would be the shape a stubbed derivation produces.
    assert_ne!(id, SessionId::from_u64(0));
}

#[test]
fn the_session_identifier_changes_with_every_input_that_defines_the_session() {
    let (alice, bob) = identities();
    let mallory = DeviceIdentity::from_test_seed(&[0x33; 32]);

    let run = |initiator: &DeviceIdentity,
               responder: &DeviceIdentity,
               initiator_entropy: [u8; 64],
               responder_entropy: [u8; 64]| {
        let (hello, awaiting) = InitiatorStart::new(initiator)
            .send_hello_with_entropy(initiator_entropy)
            .expect("opens");
        let (responder_hello, awaiting_finish) = ResponderStart::new(responder)
            .receive_initiator_hello(&hello, responder_entropy)
            .expect("accepts");
        let (finish, awaiting_responder_finish) = awaiting
            .receive_responder_hello(&responder_hello)
            .expect("verifies");
        let pending = awaiting_finish
            .receive_initiator_finish(&finish)
            .expect("verifies");
        let responder_finish = *pending.encoded_finish();
        let _ = pending.confirm_sent();
        awaiting_responder_finish
            .receive_responder_finish(&responder_finish)
            .expect("verifies")
            .session_id()
    };

    let base = run(&alice, &bob, entropy(1), entropy(2));

    // Ephemeral entropy, and therefore the nonces, differ.
    assert_ne!(base, run(&alice, &bob, entropy(3), entropy(2)));
    assert_ne!(base, run(&alice, &bob, entropy(1), entropy(4)));
    // A different peer identity is a different session.
    assert_ne!(base, run(&alice, &mallory, entropy(1), entropy(2)));
    assert_ne!(base, run(&mallory, &bob, entropy(1), entropy(2)));
    // Same everything reproduces it: the derivation is a function of its inputs.
    assert_eq!(base, run(&alice, &bob, entropy(1), entropy(2)));
}

// --------------------------------- 3.2 the responder must deliver before using

#[test]
fn the_responder_is_not_established_until_it_confirms_delivery() {
    // `receive_initiator_finish` used to hand back `EstablishedResponder`
    // alongside the bytes still waiting to be sent. A responder holding an
    // established session has, by every other signal this API gives, a session
    // it may use — while the peer has not yet seen the message that completes
    // the handshake and may never see it.
    let (_awaiting, pending, _finish) = up_to_pending();

    // The pending state is the only thing on offer, and it exposes exactly one
    // thing: the bytes to put on the wire.
    let bytes = pending.encoded_finish();
    assert_eq!(bytes.len(), super::RESPONDER_FINISH_LEN);

    // Establishing is a separate, explicit act.
    let established = pending.confirm_sent();
    assert_eq!(established.role(), super::Role::Responder);
}

#[test]
fn confirming_delivery_consumes_the_pending_state() {
    // `confirm_sent` takes `self`, so a second confirmation cannot be written.
    // This test documents the guarantee; the compiler enforces it.
    let source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/handshake/mod.rs"))
            .expect("mod.rs is readable");

    assert!(
        source.contains("pub fn confirm_sent(self)"),
        "confirm_sent must consume the pending state, not borrow it"
    );
}

#[test]
fn the_initiator_still_waits_for_the_responder_finish() {
    // The change must not accidentally let the initiator establish early
    // either. It has always required a verified ResponderFinish; it still does.
    let (awaiting, pending, finish) = up_to_pending();
    let _responder = pending.confirm_sent();

    let mut tampered = finish;
    tampered[super::RESPONDER_FINISH_LEN - 1] ^= 0x01;
    assert_eq!(
        awaiting.receive_responder_finish(&tampered).err(),
        Some(HandshakeError::FinishedVerificationFailed)
    );
}

// ------------------------------------------------ 3.3 no public key handles

#[test]
fn no_session_key_handle_is_reachable_from_the_public_api() {
    // An architecture contract. `SessionKey` was exported from the crate root
    // and both established states handed out `&SessionKey`. Nothing outside
    // this crate has any use for raw traffic secrets, and every additional
    // holder of one is another place that has to get zeroization, logging and
    // serialization right.
    let lib = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is readable");

    assert!(
        !lib.contains("SessionKey"),
        "qyro_crypto must not export SessionKey from its root"
    );

    let handshake =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/handshake/mod.rs"))
            .expect("mod.rs is readable");

    for forbidden in [
        "pub const fn sending_key(",
        "pub fn sending_key(",
        "pub const fn receiving_key(",
        "pub fn receiving_key(",
    ] {
        assert!(
            !handshake.contains(forbidden),
            "the established states must not expose {forbidden}"
        );
    }
}

#[test]
fn an_established_session_exposes_only_public_facts() {
    let (initiator, responder) = completed();
    let (alice, bob) = identities();

    assert_eq!(initiator.role(), super::Role::Initiator);
    assert_eq!(responder.role(), super::Role::Responder);
    assert_eq!(initiator.peer_fingerprint(), bob.fingerprint());
    assert_eq!(responder.peer_fingerprint(), alice.fingerprint());
    assert_eq!(initiator.peer_identity().fingerprint(), bob.fingerprint());
    assert_eq!(initiator.session_id(), responder.session_id());
}

// ----------------------------------------- 3.4 entropy cannot be fabricated

#[test]
fn no_code_path_can_substitute_bytes_for_entropy() {
    // The finding: the RNG adapter answered any read past its buffer, and any
    // oversized read, by zeroing the destination and returning success. The
    // comment defending it said a handshake with obviously dead keys beats one
    // that reuses entropy — but an all-zero X25519 secret is not obviously
    // dead. It clamps to a valid scalar and completes a perfectly ordinary
    // handshake containing no entropy at all.
    //
    // The fix is not a better adapter. `EphemeralSecret::random_from_rng` needs
    // a `CryptoRng`, whose `fill_bytes` is infallible, so *no* adapter feeding
    // it can report exhaustion — the fallback was forced by the shape of the
    // trait. The secret is now built straight from bytes, so there is no
    // adapter and nothing that could substitute anything.
    let source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/handshake/mod.rs"))
            .expect("mod.rs is readable");

    // Code only. The doc comments deliberately *name* the rejected constructors
    // to explain why they are rejected, so scanning the raw text would flag the
    // explanation as the defect.
    let code: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains("FixedRng"),
        "the fallible-looking RNG adapter must be gone, not repaired"
    );
    assert!(
        !code.contains("random_from_rng") && !code.contains("EphemeralSecret"),
        "no constructor that draws its own entropy may be on this path"
    );
    assert!(
        code.contains("StaticSecret::from(bytes)"),
        "the secret is built from bytes the caller already drew fallibly"
    );

    // And the panicking convenience constructor is unreachable: the feature
    // that provides it is off.
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("Cargo.toml is readable");
    let x25519 = manifest
        .lines()
        .find(|line| line.starts_with("x25519-dalek"))
        .expect("x25519-dalek is a dependency");
    assert!(
        !x25519.contains("\"getrandom\""),
        "x25519-dalek's getrandom feature panics on CSPRNG failure; it must stay off"
    );
}

#[test]
fn every_ephemeral_secret_is_used_exactly_once() {
    // `StaticSecret` is `Clone` and its `diffie_hellman` borrows. The wrapper
    // exists to take that back: it is not `Clone`, and its `diffie_hellman`
    // consumes `self`, so reusing an ephemeral secret does not compile.
    let source =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/handshake/mod.rs"))
            .expect("mod.rs is readable");

    let wrapper = source
        .split("pub(crate) struct EphemeralKeyPair")
        .nth(1)
        .expect("the wrapper exists");
    assert!(
        !wrapper[..wrapper
            .find("impl core::fmt::Debug")
            .unwrap_or(wrapper.len())]
            .contains("derive(Clone"),
        "an ephemeral secret must not be cloneable"
    );
    assert!(
        source.contains("pub(crate) fn diffie_hellman(\n        self,"),
        "the exchange must consume the secret"
    );
}

#[test]
fn different_entropy_produces_different_ephemeral_keys() {
    // The behavioural half: the bytes drawn actually reach the key. A
    // fabricating adapter would have made these identical.
    let (alice, bob) = identities();

    let (hello_one, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    let (hello_two, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(2))
        .expect("opens");

    let ephemeral =
        |hello: &[u8; super::INITIATOR_HELLO_LEN]| hello[3..3 + super::X25519_PUBLIC_LEN].to_vec();
    assert_ne!(ephemeral(&hello_one), ephemeral(&hello_two));
    assert_ne!(
        ephemeral(&hello_one),
        vec![0u8; super::X25519_PUBLIC_LEN],
        "an all-zero public key is what a zero secret would have produced"
    );

    // And the same entropy reproduces the same key, so nothing is drawn behind
    // the caller's back.
    let (hello_again, _) = InitiatorStart::new(&alice)
        .send_hello_with_entropy(entropy(1))
        .expect("opens");
    assert_eq!(ephemeral(&hello_one), ephemeral(&hello_again));

    // Both roles, not just the initiator.
    let (responder_hello, _) = ResponderStart::new(&bob)
        .receive_initiator_hello(&hello_one, entropy(9))
        .expect("accepts");
    assert_ne!(
        &responder_hello[3..3 + super::X25519_PUBLIC_LEN],
        &[0u8; super::X25519_PUBLIC_LEN][..]
    );
}

#[test]
fn the_system_entropy_path_still_produces_a_working_handshake() {
    // Removing the adapter must not have broken the real path, which draws from
    // getrandom and reports failure as EntropyUnavailable.
    let (alice, bob) = identities();

    let (hello, awaiting) = InitiatorStart::new(&alice)
        .send_hello()
        .expect("system entropy is available");
    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello_from_system(&hello)
        .expect("system entropy is available");
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

    assert_eq!(initiator.session_id(), responder.session_id());
    assert_ne!(initiator.session_id(), SessionId::from_u64(0));
}
