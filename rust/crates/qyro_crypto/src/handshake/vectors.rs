//! Interoperable handshake vectors: generation, verification and schema.
//!
//! A test written in Rust that runs the Rust state machine and compares its
//! output with itself proves that Rust agrees with Rust. It does not show the
//! format is defined without ambiguity, and it is no help at all to someone
//! writing the Swift or Kotlin side.
//!
//! So this module does three separate things:
//!
//! 1. **Generates** `docs/security/test-vectors/handshake-v1.json` from a fixed
//!    seed and fixed entropy, deterministically.
//! 2. **Checks** that regenerating reproduces the committed file byte for byte,
//!    so the file cannot drift away from the code.
//! 3. **Verifies** every recorded value a second time from the underlying
//!    primitives — X25519, Ed25519, HKDF, HMAC, SHA-256 — without going through
//!    the state machine that produced it. If the state machine and the
//!    specification disagree, that is where it shows.

use hkdf::Hkdf;
use qyro_protocol::{SESSION_ID_LEN, SessionId};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

use super::schedule::hmac_sha256;
use super::transcript::{
    auth_transcript, base_transcript, initiator_signing_message, responder_signing_message,
};
use super::{
    CRYPTO_SUITE_ID, HANDSHAKE_VERSION, INITIATOR_FINISH_LEN, INITIATOR_HELLO_LEN, InitiatorStart,
    RESPONDER_FINISH_LEN, RESPONDER_HELLO_LEN, ResponderStart,
};
use crate::identity::DeviceIdentity;
use crate::schema::validate;
use crate::signature::{IdentitySignature, SignatureDomain};

/// The committed vectors.
const COMMITTED: &str = include_str!("../../../../../docs/security/test-vectors/handshake-v1.json");

/// The schema the committed vectors must satisfy.
const SCHEMA: &str =
    include_str!("../../../../../docs/security/test-vectors/handshake-v1.schema.json");

/// Bytes in the unsigned part of a hello.
const HELLO_UNSIGNED_LEN: usize = super::HELLO_UNSIGNED_LEN;

// TEST ONLY — NEVER PRODUCTION. Fixed and arbitrary; no structure is implied.
const INITIATOR_SEED: [u8; 32] = [0x11; 32];
const RESPONDER_SEED: [u8; 32] = [0x22; 32];

/// Deterministic handshake entropy. TEST ONLY — NEVER PRODUCTION.
///
/// Varying and non-zero: a constant buffer would hide a byte-order mistake, and
/// an all-zero one would hide a missing derivation.
fn entropy(tag: u8) -> [u8; 64] {
    let mut out = [0u8; 64];
    for (index, slot) in out.iter_mut().enumerate() {
        *slot = tag ^ (index as u8).wrapping_mul(31).wrapping_add(7);
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "hex must be byte-aligned");
    assert!(
        text.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "vectors use lowercase hex only"
    );
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex digit"))
        .collect()
}

/// Every value one handshake produces, recorded as it is produced.
struct Recorded {
    initiator_hello: [u8; INITIATOR_HELLO_LEN],
    responder_hello: [u8; RESPONDER_HELLO_LEN],
    initiator_finish: [u8; INITIATOR_FINISH_LEN],
    responder_finish: [u8; RESPONDER_FINISH_LEN],
    session_id: SessionId,
    initiator_sending: [u8; 32],
    initiator_receiving: [u8; 32],
}

/// Runs the handshake the ordinary way and records what it produced.
fn run() -> Recorded {
    let initiator = DeviceIdentity::from_test_seed(&INITIATOR_SEED);
    let responder = DeviceIdentity::from_test_seed(&RESPONDER_SEED);

    let (initiator_hello, awaiting_responder) = InitiatorStart::new(&initiator)
        .send_hello_with_entropy(entropy(0xA0))
        .expect("opens");
    let (responder_hello, awaiting_initiator_finish) = ResponderStart::new(&responder)
        .receive_initiator_hello(&initiator_hello, entropy(0xB0))
        .expect("accepts");
    let (initiator_finish, awaiting_responder_finish) = awaiting_responder
        .receive_responder_hello(&responder_hello)
        .expect("verifies");

    let pending = awaiting_initiator_finish
        .receive_initiator_finish(&initiator_finish)
        .expect("verifies");
    let responder_finish = *pending.encoded_finish();
    let established_responder = pending.confirm_sent();
    let established_initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .expect("verifies");

    assert_eq!(
        established_initiator.session_id(),
        established_responder.session_id()
    );

    Recorded {
        initiator_hello,
        responder_hello,
        initiator_finish,
        responder_finish,
        session_id: established_initiator.session_id(),
        initiator_sending: *established_initiator.sending_key_bytes(),
        initiator_receiving: *established_initiator.receiving_key_bytes(),
    }
}

/// Rebuilds the HKDF `info` for one label, exactly as ADR-0021 specifies.
fn info_for(label: &str, auth: &[u8; 32]) -> Vec<u8> {
    let mut info = Vec::new();
    info.extend_from_slice(b"QYRO-HS-V1/");
    info.extend_from_slice(label.as_bytes());
    info.push(0x00);
    info.extend_from_slice(auth);
    info
}

/// Builds the vector document from scratch, using primitives directly.
///
/// Deliberately does **not** ask the state machine for the intermediate values.
/// Everything below is recomputed from the seeds and the entropy, and only then
/// compared against what the state machine emitted.
fn build_document() -> Value {
    let recorded = run();

    let initiator = DeviceIdentity::from_test_seed(&INITIATOR_SEED);
    let responder = DeviceIdentity::from_test_seed(&RESPONDER_SEED);

    let initiator_entropy = entropy(0xA0);
    let responder_entropy = entropy(0xB0);

    // --- X25519, from the entropy, without the handshake module -------------
    let mut initiator_x_secret = [0u8; 32];
    initiator_x_secret.copy_from_slice(&initiator_entropy[..32]);
    let mut responder_x_secret = [0u8; 32];
    responder_x_secret.copy_from_slice(&responder_entropy[..32]);

    let initiator_static = StaticSecret::from(initiator_x_secret);
    let responder_static = StaticSecret::from(responder_x_secret);
    let initiator_x_public = X25519PublicKey::from(&initiator_static);
    let responder_x_public = X25519PublicKey::from(&responder_static);

    let shared = initiator_static.diffie_hellman(&responder_x_public);
    assert_eq!(
        shared.to_bytes(),
        responder_static
            .diffie_hellman(&initiator_x_public)
            .to_bytes(),
        "both directions of the exchange agree"
    );
    assert!(shared.was_contributory());

    let initiator_nonce = &initiator_entropy[32..];
    let responder_nonce = &responder_entropy[32..];

    // --- transcripts --------------------------------------------------------
    let responder_hello_unsigned: &[u8; HELLO_UNSIGNED_LEN] = recorded.responder_hello
        [..HELLO_UNSIGNED_LEN]
        .try_into()
        .expect("a hello's unsigned prefix is HELLO_UNSIGNED_LEN bytes");
    let base = base_transcript(&recorded.initiator_hello, responder_hello_unsigned);

    let responder_signing_input = responder_signing_message(&base);
    let responder_signature = responder
        .try_sign(
            SignatureDomain::HandshakeTranscript,
            &responder_signing_input,
        )
        .expect("the domain is available");

    let initiator_signing_input = initiator_signing_message(&base, responder_signature.as_bytes());
    let initiator_signature = initiator
        .try_sign(
            SignatureDomain::HandshakeTranscript,
            &initiator_signing_input,
        )
        .expect("the domain is available");

    let auth = auth_transcript(
        &base,
        responder_signature.as_bytes(),
        initiator_signature.as_bytes(),
    );

    // --- key schedule, run directly through hkdf ----------------------------
    let hkdf = Hkdf::<Sha256>::new(Some(&base), &shared.to_bytes());
    let expand = |label: &str, out: &mut [u8]| {
        hkdf.expand(&info_for(label, &auth), out)
            .expect("hkdf output length is valid");
    };

    let mut initiator_finished_key = [0u8; 32];
    let mut responder_finished_key = [0u8; 32];
    let mut initiator_to_responder = [0u8; 32];
    let mut responder_to_initiator = [0u8; 32];
    let mut session_id = [0u8; SESSION_ID_LEN];
    expand("initiator-finished", &mut initiator_finished_key);
    expand("responder-finished", &mut responder_finished_key);
    expand("initiator-to-responder", &mut initiator_to_responder);
    expand("responder-to-initiator", &mut responder_to_initiator);
    expand("session-id", &mut session_id);

    let initiator_mac = hmac_sha256(&initiator_finished_key, &auth);
    let responder_mac = hmac_sha256(&responder_finished_key, &auth);

    // --- the independent computation must match what the machine produced ---
    assert_eq!(
        SessionId::from_be_bytes(session_id),
        recorded.session_id,
        "the session id derived directly must match the state machine's"
    );
    assert_eq!(initiator_to_responder, recorded.initiator_sending);
    assert_eq!(responder_to_initiator, recorded.initiator_receiving);
    assert_eq!(
        &recorded.responder_hello[HELLO_UNSIGNED_LEN..],
        responder_signature.as_bytes(),
        "the responder hello carries the signature computed here"
    );
    assert_eq!(
        &recorded.initiator_finish[3..67],
        initiator_signature.as_bytes()
    );
    assert_eq!(&recorded.initiator_finish[67..], &initiator_mac);
    assert_eq!(&recorded.responder_finish[3..], &responder_mac);
    assert_eq!(
        &recorded.initiator_hello[3..35],
        initiator_x_public.as_bytes()
    );
    assert_eq!(
        &recorded.responder_hello[3..35],
        responder_x_public.as_bytes()
    );

    json!({
        "_warning": "TEST ONLY - NEVER PRODUCTION. Every seed and every entropy \
    buffer below is fixed and published. Any identity or session derived from them \
    is compromised by definition.",
        "schema_version": 1,
        "format": "qyro-handshake-v1",
        "specification": "docs/adr/ADR-0021-authenticated-handshake.md",
        "handshake_version": HANDSHAKE_VERSION,
        "crypto_suite_id": CRYPTO_SUITE_ID,
        "identities": {
            "initiator": {
                "seed": hex(&INITIATOR_SEED),
                "ed25519_public_key": hex(initiator.public_identity().as_bytes()),
                "public_identity_wire": hex(&initiator.public_identity().encode()),
                "fingerprint": initiator.fingerprint().to_hex()
            },
            "responder": {
                "seed": hex(&RESPONDER_SEED),
                "ed25519_public_key": hex(responder.public_identity().as_bytes()),
                "public_identity_wire": hex(&responder.public_identity().encode()),
                "fingerprint": responder.fingerprint().to_hex()
            }
        },
        "entropy": {
            "initiator_handshake_entropy": hex(&initiator_entropy),
            "responder_handshake_entropy": hex(&responder_entropy),
            "initiator_x25519_secret": hex(&initiator_x_secret),
            "responder_x25519_secret": hex(&responder_x_secret),
            "initiator_x25519_public": hex(initiator_x_public.as_bytes()),
            "responder_x25519_public": hex(responder_x_public.as_bytes()),
            "initiator_nonce": hex(initiator_nonce),
            "responder_nonce": hex(responder_nonce),
            "shared_secret": hex(&shared.to_bytes())
        },
        "messages": {
            "initiator_hello": hex(&recorded.initiator_hello),
            "responder_hello_unsigned": hex(responder_hello_unsigned),
            "responder_hello": hex(&recorded.responder_hello),
            "initiator_finish": hex(&recorded.initiator_finish),
            "responder_finish": hex(&recorded.responder_finish)
        },
        "transcript": {
            "base_transcript_hash": hex(&base),
            "responder_signing_input": hex(&responder_signing_input),
            "responder_signature": hex(responder_signature.as_bytes()),
            "initiator_signing_input": hex(&initiator_signing_input),
            "initiator_signature": hex(initiator_signature.as_bytes()),
            "auth_transcript_hash": hex(&auth)
        },
        "schedule": {
            "hkdf_salt": hex(&base),
            "hkdf_input_key_material": hex(&shared.to_bytes()),
            "info_initiator_finished": hex(&info_for("initiator-finished", &auth)),
            "info_responder_finished": hex(&info_for("responder-finished", &auth)),
            "info_initiator_to_responder": hex(&info_for("initiator-to-responder", &auth)),
            "info_responder_to_initiator": hex(&info_for("responder-to-initiator", &auth)),
            "info_session_id": hex(&info_for("session-id", &auth)),
            "initiator_finished_key": hex(&initiator_finished_key),
            "responder_finished_key": hex(&responder_finished_key),
            "initiator_to_responder_traffic_secret": hex(&initiator_to_responder),
            "responder_to_initiator_traffic_secret": hex(&responder_to_initiator),
            "session_id": hex(&session_id)
        },
        "finished": {
            "initiator_finished_input": hex(&auth),
            "initiator_finished_mac": hex(&initiator_mac),
            "responder_finished_input": hex(&auth),
            "responder_finished_mac": hex(&responder_mac)
        }
    })
}

/// Renders the document exactly as the committed file stores it.
///
/// `serde_json`'s map is a `BTreeMap` here, so key order is sorted and stable
/// across runs and platforms. No timestamps, no host state, no randomness.
fn render(document: &Value) -> String {
    let mut text = serde_json::to_string_pretty(document).expect("serializable");
    text.push('\n');
    text
}

// ------------------------------------------------------------------ generation

/// Prints the vector file. **Not run by default.**
///
/// ```text
/// cargo test -p qyro_crypto generate_handshake_vector -- --ignored --nocapture
/// ```
///
/// Ignored rather than a binary because a public generator would need a
/// deterministic constructor in `qyro_crypto`'s public API, which is exactly
/// what the crate must not export. Redirect the output over the committed file
/// to regenerate it deliberately; nothing rewrites it on its own.
#[test]
#[ignore = "regenerates the committed vector file; run explicitly"]
fn generate_handshake_vector() {
    print!("{}", render(&build_document()));
}

#[test]
fn the_committed_vector_is_exactly_what_regeneration_produces() {
    let regenerated = render(&build_document());
    assert_eq!(regenerated, COMMITTED, "{}", regeneration_advice());
}

/// What to say when the committed vector and the code disagree.
///
/// QYR-0044. This used to read "the committed handshake vector is stale;
/// regenerate it with …", unconditionally. Telling somebody to regenerate is
/// telling them to record whatever the code now produces. That is right when
/// the format changed on purpose and the ADR already says so. It is exactly
/// wrong when the code has drifted from the ADR — then the committed file is
/// the only thing still holding the specification, and regenerating destroys
/// the evidence.
///
/// So the advice is conditional on the thing that tells the two apart: whether
/// this build still computes the transcript ADR-0021 specifies, checked here
/// against SHA-256 over literal bytes rather than against the code that
/// produced them.
fn regeneration_advice() -> String {
    let initiator_hello = [0xA1u8; HELLO_UNSIGNED_LEN];
    let responder_unsigned = [0xB2u8; HELLO_UNSIGNED_LEN];
    let matches_the_adr = base_transcript(&initiator_hello, &responder_unsigned)
        == base_transcript_from_primitives(&initiator_hello, &responder_unsigned);

    if matches_the_adr {
        "the committed handshake vector does not match what this build \
         produces, and the transcript still agrees with ADR-0021. If the format \
         change was intended and the ADR already records it, regenerate with \
         `cargo test -p qyro_crypto generate_handshake_vector -- --ignored \
         --nocapture`. If it was not, the code changed and the file is right."
            .to_owned()
    } else {
        "the committed handshake vector does not match what this build \
         produces, **and this build no longer computes the transcript ADR-0021 \
         specifies**. Do not regenerate: the committed file is the only thing \
         still holding the specification, and recording the new output would \
         only record the disagreement. Fix the code, or amend the ADR first."
            .to_owned()
    }
}

// ------------------------------------------------------------------- the schema

#[test]
fn the_committed_vector_satisfies_the_schema() {
    let document: Value = serde_json::from_str(COMMITTED).expect("the vector file is valid JSON");
    let schema: Value = serde_json::from_str(SCHEMA).expect("the schema is valid JSON");
    validate(&document, &schema, "").expect("the committed vector satisfies its schema");
}

#[test]
fn the_schema_rejects_what_it_says_it_rejects() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("valid JSON");
    let original: Value = serde_json::from_str(COMMITTED).expect("valid JSON");

    // Uppercase hex: one value, one spelling.
    let mut uppercase = original.clone();
    uppercase["transcript"]["base_transcript_hash"] = json!(
        original["transcript"]["base_transcript_hash"]
            .as_str()
            .expect("string")
            .to_uppercase()
    );
    assert!(validate(&uppercase, &schema, "").is_err(), "uppercase hex");

    // A truncated session id. This is the exact defect the schema exists to
    // catch: eight bytes is the wire width, and sixteen hex characters is how
    // that looks in the file.
    let mut truncated = original.clone();
    truncated["schedule"]["session_id"] = json!("0011223344556677".get(..8).expect("prefix"));
    assert!(
        validate(&truncated, &schema, "").is_err(),
        "short session id"
    );

    // An unknown property.
    let mut extra = original.clone();
    extra["schedule"]["surprise"] = json!("00");
    assert!(validate(&extra, &schema, "").is_err(), "unknown property");

    // A missing property.
    let mut missing = original.clone();
    missing["schedule"]
        .as_object_mut()
        .expect("object")
        .remove("session_id");
    assert!(validate(&missing, &schema, "").is_err(), "missing property");

    // A changed version constant.
    let mut version = original.clone();
    version["handshake_version"] = json!(2);
    assert!(validate(&version, &schema, "").is_err(), "wrong version");

    // And the untouched document still passes, so the checks above are
    // rejecting the mutation rather than the document.
    assert!(validate(&original, &schema, "").is_ok());
}

#[test]
fn the_schema_uses_only_keywords_the_validator_enforces() {
    // The guarantee that makes the validator trustworthy: if someone adds a
    // `minLength` or a `oneOf` to the schema, this fails rather than silently
    // leaving the new constraint unchecked.
    let schema: Value = serde_json::from_str(SCHEMA).expect("valid JSON");
    let document: Value = serde_json::from_str(COMMITTED).expect("valid JSON");
    validate(&document, &schema, "").expect("no unsupported keyword anywhere in the schema");
}

// ------------------------------------------------------ independent verification

#[test]
fn every_recorded_value_verifies_against_the_primitives() {
    // Reads the committed file and rebuilds each value from the primitives.
    // Nothing here calls the state machine, so a change in the state machine
    // that also changed the vectors would still be caught: the file would stop
    // matching the specification even while matching the code.
    let document: Value = serde_json::from_str(COMMITTED).expect("valid JSON");

    let field = |path: &[&str]| -> String {
        let mut node = &document;
        for key in path {
            node = &node[*key];
        }
        node.as_str()
            .unwrap_or_else(|| panic!("{path:?} is a string"))
            .to_owned()
    };

    // --- identities ---------------------------------------------------------
    let initiator = DeviceIdentity::from_test_seed(
        &unhex(&field(&["identities", "initiator", "seed"]))
            .try_into()
            .expect("32-byte seed"),
    );
    let responder = DeviceIdentity::from_test_seed(
        &unhex(&field(&["identities", "responder", "seed"]))
            .try_into()
            .expect("32-byte seed"),
    );
    assert_eq!(
        hex(initiator.public_identity().as_bytes()),
        field(&["identities", "initiator", "ed25519_public_key"])
    );
    assert_eq!(
        hex(&responder.public_identity().encode()),
        field(&["identities", "responder", "public_identity_wire"])
    );
    assert_eq!(
        initiator.fingerprint().to_hex(),
        field(&["identities", "initiator", "fingerprint"])
    );

    // --- X25519 -------------------------------------------------------------
    let initiator_secret: [u8; 32] = unhex(&field(&["entropy", "initiator_x25519_secret"]))
        .try_into()
        .expect("32 bytes");
    let responder_secret: [u8; 32] = unhex(&field(&["entropy", "responder_x25519_secret"]))
        .try_into()
        .expect("32 bytes");
    let initiator_static = StaticSecret::from(initiator_secret);
    let responder_static = StaticSecret::from(responder_secret);

    assert_eq!(
        hex(X25519PublicKey::from(&initiator_static).as_bytes()),
        field(&["entropy", "initiator_x25519_public"])
    );
    let shared = initiator_static.diffie_hellman(&X25519PublicKey::from(&responder_static));
    assert_eq!(
        hex(&shared.to_bytes()),
        field(&["entropy", "shared_secret"])
    );

    // --- transcripts --------------------------------------------------------
    let initiator_hello = unhex(&field(&["messages", "initiator_hello"]));
    let responder_hello_unsigned = unhex(&field(&["messages", "responder_hello_unsigned"]));
    let base = base_transcript_from_primitives(&initiator_hello, &responder_hello_unsigned);
    assert_eq!(
        hex(&base),
        field(&["transcript", "base_transcript_hash"]),
        "the recorded base transcript is not what ADR-0021 specifies"
    );

    // The recorded signing inputs are checked against the ADR too, not merely
    // used. The responder signs the base transcript alone; the initiator signs
    // it followed by the responder's signature.
    assert_eq!(
        field(&["transcript", "responder_signing_input"]),
        hex(&base),
        "the responder signs the base transcript and nothing else"
    );

    let responder_signature =
        IdentitySignature::from_slice(&unhex(&field(&["transcript", "responder_signature"])))
            .expect("64 bytes");
    responder
        .public_identity()
        .verify(
            SignatureDomain::HandshakeTranscript,
            &unhex(&field(&["transcript", "responder_signing_input"])),
            &responder_signature,
        )
        .expect("the responder signature verifies over the recorded input");

    let initiator_signature =
        IdentitySignature::from_slice(&unhex(&field(&["transcript", "initiator_signature"])))
            .expect("64 bytes");
    initiator
        .public_identity()
        .verify(
            SignatureDomain::HandshakeTranscript,
            &unhex(&field(&["transcript", "initiator_signing_input"])),
            &initiator_signature,
        )
        .expect("the initiator signature verifies over the recorded input");

    let mut initiator_signed = Vec::new();
    initiator_signed.extend_from_slice(&base);
    initiator_signed.extend_from_slice(responder_signature.as_bytes());
    assert_eq!(
        field(&["transcript", "initiator_signing_input"]),
        hex(&initiator_signed),
        "the initiator signs the base transcript followed by the responder's \
         signature, which is what binds its signature to this answer"
    );

    let auth = auth_transcript_from_primitives(
        &base,
        responder_signature.as_bytes(),
        initiator_signature.as_bytes(),
    );
    assert_eq!(
        hex(&auth),
        field(&["transcript", "auth_transcript_hash"]),
        "the recorded auth transcript is not what ADR-0021 specifies"
    );

    // --- HKDF, run directly -------------------------------------------------
    let hkdf = Hkdf::<Sha256>::new(Some(&base), &shared.to_bytes());
    let check = |label: &str, recorded: &str, width: usize| {
        let mut out = vec![0u8; width];
        hkdf.expand(&info_for(label, &auth), &mut out)
            .expect("valid length");
        assert_eq!(hex(&out), recorded, "label {label}");
        out
    };

    let initiator_finished_key = check(
        "initiator-finished",
        &field(&["schedule", "initiator_finished_key"]),
        32,
    );
    let responder_finished_key = check(
        "responder-finished",
        &field(&["schedule", "responder_finished_key"]),
        32,
    );
    check(
        "initiator-to-responder",
        &field(&["schedule", "initiator_to_responder_traffic_secret"]),
        32,
    );
    check(
        "responder-to-initiator",
        &field(&["schedule", "responder_to_initiator_traffic_secret"]),
        32,
    );
    let session_id = check(
        "session-id",
        &field(&["schedule", "session_id"]),
        SESSION_ID_LEN,
    );
    assert_eq!(
        session_id.len(),
        SESSION_ID_LEN,
        "the session id is derived at wire width, not truncated"
    );

    // --- and the schedule this crate actually runs --------------------------
    //
    // Everything above re-derives the recorded values with HKDF driven by hand.
    // That checks the *file* against the specification and says nothing about
    // `Schedule::derive`, so rerouting its `info` into its salt left this test
    // green. Pinning the real schedule against the values just verified closes
    // that: the two now have to agree with each other and with the ADR.
    let derived = super::schedule::Schedule::derive(&base, &shared.to_bytes(), &auth)
        .expect("the recorded inputs derive a schedule");
    assert_eq!(
        hex(derived.initiator_to_responder.as_bytes()),
        field(&["schedule", "initiator_to_responder_traffic_secret"]),
        "the key schedule this crate runs disagrees with the primitives"
    );
    assert_eq!(
        hex(derived.responder_to_initiator.as_bytes()),
        field(&["schedule", "responder_to_initiator_traffic_secret"])
    );
    assert_eq!(
        hex(derived.initiator_finished.as_ref()),
        field(&["schedule", "initiator_finished_key"])
    );
    assert_eq!(
        hex(derived.responder_finished.as_ref()),
        field(&["schedule", "responder_finished_key"])
    );
    assert_eq!(
        hex(&derived.session_id.to_be_bytes()),
        field(&["schedule", "session_id"])
    );

    // --- HMAC ---------------------------------------------------------------
    assert_eq!(
        hex(&hmac_sha256_from_primitives(&initiator_finished_key, &auth)),
        field(&["finished", "initiator_finished_mac"])
    );
    assert_eq!(
        hex(&hmac_sha256_from_primitives(&responder_finished_key, &auth)),
        field(&["finished", "responder_finished_mac"])
    );

    // --- the messages carry exactly those values ----------------------------
    let responder_hello = unhex(&field(&["messages", "responder_hello"]));
    assert_eq!(
        &responder_hello[..HELLO_UNSIGNED_LEN],
        &responder_hello_unsigned[..],
        "the unsigned prefix is the signed hello's first bytes"
    );
    assert_eq!(
        hex(&responder_hello[HELLO_UNSIGNED_LEN..]),
        field(&["transcript", "responder_signature"])
    );

    let initiator_finish = unhex(&field(&["messages", "initiator_finish"]));
    assert_eq!(
        hex(&initiator_finish[3..67]),
        field(&["transcript", "initiator_signature"])
    );
    assert_eq!(
        hex(&initiator_finish[67..]),
        field(&["finished", "initiator_finished_mac"])
    );

    let responder_finish = unhex(&field(&["messages", "responder_finish"]));
    assert_eq!(
        hex(&responder_finish[3..]),
        field(&["finished", "responder_finished_mac"])
    );
}

#[test]
fn the_recorded_messages_replay_through_the_state_machine() {
    // The other direction: a peer that has only the committed bytes must be
    // able to drive a real handshake with them. This is what an implementation
    // in another language will actually do with the file.
    let document: Value = serde_json::from_str(COMMITTED).expect("valid JSON");
    let message = |name: &str| unhex(document["messages"][name].as_str().expect("string"));

    let initiator = DeviceIdentity::from_test_seed(&INITIATOR_SEED);
    let responder = DeviceIdentity::from_test_seed(&RESPONDER_SEED);

    let (hello, awaiting) = InitiatorStart::new(&initiator)
        .send_hello_with_entropy(entropy(0xA0))
        .expect("opens");
    assert_eq!(hello.to_vec(), message("initiator_hello"));

    let (responder_hello, awaiting_finish) = ResponderStart::new(&responder)
        .receive_initiator_hello(&hello, entropy(0xB0))
        .expect("accepts");
    assert_eq!(responder_hello.to_vec(), message("responder_hello"));

    let (finish, awaiting_responder_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    assert_eq!(finish.to_vec(), message("initiator_finish"));

    let pending = awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies");
    assert_eq!(
        pending.encoded_finish().to_vec(),
        message("responder_finish")
    );

    let responder_finish = *pending.encoded_finish();
    let established_responder = pending.confirm_sent();
    let established_initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .expect("verifies");

    let recorded_session_id: [u8; SESSION_ID_LEN] =
        unhex(document["schedule"]["session_id"].as_str().expect("string"))
            .try_into()
            .expect("eight bytes");
    assert_eq!(
        established_initiator.session_id(),
        SessionId::from_be_bytes(recorded_session_id)
    );
    assert_eq!(
        established_responder.session_id(),
        SessionId::from_be_bytes(recorded_session_id)
    );
}

// -------------------------------------------------------------- RFC 7748 KAT

/// RFC 7748's own X25519 vectors.
const RFC7748: &str = include_str!("../../../../../docs/security/test-vectors/rfc7748-x25519.json");

#[test]
fn the_x25519_implementation_matches_rfc7748() {
    // The handshake's key agreement is only meaningful if the X25519 underneath
    // it is the standard one. Section 5 pins the raw scalar multiplication
    // including clamping — which is the part this crate deliberately does not
    // implement — and section 6.1 pins a whole exchange.
    let parsed: Value = serde_json::from_str(RFC7748).expect("the KAT file is valid JSON");

    for case in parsed["scalar_multiplication"]
        .as_array()
        .expect("an array of cases")
    {
        let name = case["name"].as_str().expect("name");
        let scalar: [u8; 32] = unhex(case["input_scalar"].as_str().expect("scalar"))
            .try_into()
            .expect("32 bytes");
        let u: [u8; 32] = unhex(case["input_u_coordinate"].as_str().expect("u"))
            .try_into()
            .expect("32 bytes");

        // `StaticSecret::diffie_hellman` is X25519(scalar, u): it clamps the
        // scalar and multiplies the supplied u-coordinate.
        let produced = StaticSecret::from(scalar).diffie_hellman(&X25519PublicKey::from(u));
        assert_eq!(
            hex(&produced.to_bytes()),
            case["output_u_coordinate"].as_str().expect("output"),
            "{name}"
        );
    }

    let exchange = &parsed["diffie_hellman"];
    let alice: [u8; 32] = unhex(exchange["alice_private_key"].as_str().expect("key"))
        .try_into()
        .expect("32 bytes");
    let bob: [u8; 32] = unhex(exchange["bob_private_key"].as_str().expect("key"))
        .try_into()
        .expect("32 bytes");
    let alice_secret = StaticSecret::from(alice);
    let bob_secret = StaticSecret::from(bob);

    assert_eq!(
        hex(X25519PublicKey::from(&alice_secret).as_bytes()),
        exchange["alice_public_key"].as_str().expect("key"),
        "Alice's public key is X25519(a, 9)"
    );
    assert_eq!(
        hex(X25519PublicKey::from(&bob_secret).as_bytes()),
        exchange["bob_public_key"].as_str().expect("key")
    );

    let from_alice = alice_secret.diffie_hellman(&X25519PublicKey::from(&bob_secret));
    let from_bob = bob_secret.diffie_hellman(&X25519PublicKey::from(&alice_secret));
    assert_eq!(from_alice.to_bytes(), from_bob.to_bytes());
    assert_eq!(
        hex(&from_alice.to_bytes()),
        exchange["shared_secret"].as_str().expect("secret"),
        "the shared secret matches RFC 7748 byte for byte"
    );
    assert!(from_alice.was_contributory());
}

// -------------------------------------------- the primitives, spelled out
//
// QYR-0025. The test below claims to verify every recorded value "against the
// primitives, without going through the state machine that produced it". It
// did not: it called `base_transcript`, `auth_transcript` and `hmac_sha256`,
// which are the very functions whose output was recorded. Rerouting the HKDF
// `info` into the salt in `schedule.rs` left it passing.
//
// These three are written out from ADR-0021 and RFC 2104 with SHA-256 as the
// only shared ingredient, so a disagreement between the specification and the
// implementation shows up here instead of cancelling out.

/// `SHA-256( "QYRO-HANDSHAKE-BASE-V1" || 0x00 || len || hello || len || hello )`.
fn base_transcript_from_primitives(
    initiator_hello: &[u8],
    responder_hello_unsigned: &[u8],
) -> [u8; 32] {
    let mut input = Vec::new();
    input.extend_from_slice(b"QYRO-HANDSHAKE-BASE-V1");
    input.push(0x00);
    input.extend_from_slice(&(initiator_hello.len() as u32).to_be_bytes());
    input.extend_from_slice(initiator_hello);
    input.extend_from_slice(&(responder_hello_unsigned.len() as u32).to_be_bytes());
    input.extend_from_slice(responder_hello_unsigned);
    Sha256::digest(&input).into()
}

/// `SHA-256( "QYRO-HANDSHAKE-AUTH-V1" || 0x00 || base || r_sig || i_sig )`.
fn auth_transcript_from_primitives(
    base: &[u8; 32],
    responder_signature: &[u8; 64],
    initiator_signature: &[u8; 64],
) -> [u8; 32] {
    let mut input = Vec::new();
    input.extend_from_slice(b"QYRO-HANDSHAKE-AUTH-V1");
    input.push(0x00);
    input.extend_from_slice(base);
    input.extend_from_slice(responder_signature);
    input.extend_from_slice(initiator_signature);
    Sha256::digest(&input).into()
}

/// HMAC-SHA-256 as RFC 2104 defines it, built from SHA-256 alone.
///
/// Not `SimpleHmac`, and not the crate's `hmac_sha256`: the point is to reach
/// the recorded MAC without the code that produced it. This is the
/// `H((K' ^ opad) || H((K' ^ ipad) || m))` construction, with `K'` the key
/// padded to the 64-byte block, which is what an implementation in another
/// language will compute from its own standard library.
fn hmac_sha256_from_primitives(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK_LEN: usize = 64;

    let mut padded_key = [0u8; BLOCK_LEN];
    if key.len() > BLOCK_LEN {
        padded_key[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        padded_key[..key.len()].copy_from_slice(key);
    }

    let mut inner = Vec::new();
    inner.extend(padded_key.iter().map(|byte| byte ^ 0x36));
    inner.extend_from_slice(message);
    let inner_digest = Sha256::digest(&inner);

    let mut outer = Vec::new();
    outer.extend(padded_key.iter().map(|byte| byte ^ 0x5C));
    outer.extend_from_slice(&inner_digest);
    Sha256::digest(&outer).into()
}
