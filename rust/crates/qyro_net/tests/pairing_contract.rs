//! The pairing string, exercised through the public API and nothing else.
//!
//! ADR-0035 §2. This is the path that works in every network, so it is the one
//! that has to be right: the QR encodes exactly this string, and phase 05 has
//! nothing else to fall back on when discovery is filtered.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use qyro_net::{
    PAIRING_FINGERPRINT_LEN, PAIRING_PREFIX, PAIRING_SEPARATOR, PairingEndpoint, PairingError,
};

/// Sixteen bytes that are not all the same, so a copy of one byte is visible.
fn a_fingerprint() -> [u8; PAIRING_FINGERPRINT_LEN] {
    let mut bytes = [0_u8; PAIRING_FINGERPRINT_LEN];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = (index as u8).wrapping_mul(17).wrapping_add(3);
    }
    bytes
}

fn v4(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::new(192, 168, 1, 7), port))
}

fn v6(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1), port))
}

#[test]
fn a_manual_endpoint_string_round_trips_through_a_qr_payload() {
    for address in [v4(47_001), v6(47_001), v4(65_535), v4(1)] {
        let original = PairingEndpoint::new(address, a_fingerprint()).unwrap();
        let payload = original.to_string();
        let read_back = PairingEndpoint::parse(&payload)
            .unwrap_or_else(|error| panic!("{payload} did not read back: {error}"));

        assert_eq!(read_back, original, "{payload} read back as something else");
        // The address specifically, and not only the whole struct: a Display
        // that dropped the port would still round trip if `parse` dropped it
        // too, and both sides of that bug live in this file's subject.
        assert_eq!(read_back.address(), address);
        assert_eq!(read_back.address().port(), address.port());
        assert_eq!(read_back.fingerprint(), &a_fingerprint());
    }
}

#[test]
fn a_changed_fingerprint_would_be_visible_to_that_round_trip() {
    // R2 §1.7. The equality above is only evidence if it can see a difference,
    // and the difference that matters here is one byte of fingerprint — the
    // whole point of putting it in the string.
    let mut altered = a_fingerprint();
    altered[9] ^= 0x01;

    let original = PairingEndpoint::new(v4(47_001), a_fingerprint()).unwrap();
    let tampered = PairingEndpoint::new(v4(47_001), altered).unwrap();

    assert_ne!(
        original.to_string(),
        tampered.to_string(),
        "one flipped bit of fingerprint produced the same string, so the round \
         trip above cannot see a swapped peer"
    );
    assert_ne!(original, tampered);
    // And the length is unchanged, so the difference is the value and not the
    // shape — a comparison that only noticed length would be a weaker test.
    assert_eq!(original.to_string().len(), tampered.to_string().len());
}

#[test]
fn the_string_has_the_shape_the_adr_froze() {
    let endpoint = PairingEndpoint::new(v6(47_001), a_fingerprint()).unwrap();
    let payload = endpoint.to_string();

    assert!(
        payload.starts_with(PAIRING_PREFIX),
        "{payload} does not start with the prefix"
    );
    // Exactly two separators, on an IPv6 address that is full of colons and
    // brackets. That is the property that makes splitting exact and escaping
    // unnecessary, and IPv6 is where a format designed around `:` would break.
    assert_eq!(
        payload.matches(PAIRING_SEPARATOR).count(),
        2,
        "{payload} does not have exactly two separators"
    );
    assert!(
        payload.contains("[fe80::1]:47001"),
        "{payload} does not carry the address in the form FromStr reads back"
    );
    // Thirty-two lowercase hex characters at the end, and not thirty-one.
    let tail = payload.rsplit(PAIRING_SEPARATOR).next().unwrap();
    assert_eq!(tail.len(), PAIRING_FINGERPRINT_LEN * 2);
    assert!(
        tail.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{tail} is not lowercase hexadecimal"
    );
}

#[test]
fn every_way_a_pairing_string_can_be_wrong_is_its_own_refusal() {
    let good = PairingEndpoint::new(v4(47_001), a_fingerprint())
        .unwrap()
        .to_string();
    let digest = good.rsplit(PAIRING_SEPARATOR).next().unwrap().to_owned();

    let cases: [(&str, PairingError); 7] = [
        (
            "NOTQYRO|192.168.1.7:47001|00112233445566778899aabbccddeeff",
            PairingError::NotAPairingString,
        ),
        (
            "QYRO1|192.168.1.7:47001",
            PairingError::WrongFieldCount { found: 2 },
        ),
        (
            "QYRO1|192.168.1.7:47001|00112233445566778899aabbccddeeff|extra",
            PairingError::WrongFieldCount { found: 4 },
        ),
        (
            "QYRO1|not-an-address|00112233445566778899aabbccddeeff",
            PairingError::UnreadableAddress,
        ),
        (
            "QYRO1|0.0.0.0:47001|00112233445566778899aabbccddeeff",
            PairingError::UnspecifiedAddress,
        ),
        (
            "QYRO1|192.168.1.7:0|00112233445566778899aabbccddeeff",
            PairingError::ZeroPort,
        ),
        (
            "QYRO1|192.168.1.7:47001|00112233445566778899aabbccddee",
            PairingError::FingerprintWrongLength { found: 30 },
        ),
    ];

    for (text, expected) in cases {
        assert_eq!(
            PairingEndpoint::parse(text),
            Err(expected),
            "{text} was not refused the way it should be"
        );
    }

    // The positive control: the same shape with nothing wrong is accepted, so
    // the seven refusals above are about what they say and not about the parser
    // refusing everything.
    assert!(
        PairingEndpoint::parse(&good).is_ok(),
        "a well-formed string was refused, so this test proves nothing"
    );
    assert_eq!(digest.len(), PAIRING_FINGERPRINT_LEN * 2);
}

#[test]
fn an_uppercase_fingerprint_is_refused_because_two_spellings_is_one_too_many() {
    let lower = "QYRO1|192.168.1.7:47001|00112233445566778899aabbccddeeff";
    let upper = "QYRO1|192.168.1.7:47001|00112233445566778899AABBCCDDEEFF";

    assert!(PairingEndpoint::parse(lower).is_ok());
    assert_eq!(
        PairingEndpoint::parse(upper),
        Err(PairingError::FingerprintNotLowercaseHex),
        "the same fingerprint had two accepted spellings, which is the exact \
         ambiguity ADR-0031 removed from the human fingerprint"
    );
    // A character that is not hex at all, so the refusal above is about case
    // and this one is about the alphabet — two different mistakes.
    assert_eq!(
        PairingEndpoint::parse("QYRO1|192.168.1.7:47001|00112233445566778899aabbccddeegg"),
        Err(PairingError::FingerprintNotLowercaseHex)
    );
}

#[test]
fn an_address_nothing_can_dial_never_becomes_an_endpoint() {
    // A listener binds `0.0.0.0:0` legitimately. A dialler cannot use either
    // half of it, so letting one into a pairing string would move the failure
    // three layers away from the mistake that caused it.
    assert_eq!(
        PairingEndpoint::new(
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 47_001)),
            a_fingerprint()
        ),
        Err(PairingError::UnspecifiedAddress)
    );
    assert_eq!(
        PairingEndpoint::new(
            SocketAddr::from((Ipv6Addr::UNSPECIFIED, 47_001)),
            a_fingerprint()
        ),
        Err(PairingError::UnspecifiedAddress)
    );
    assert_eq!(
        PairingEndpoint::new(v4(0), a_fingerprint()),
        Err(PairingError::ZeroPort)
    );
    // And the control: the same constructor with a real address works, so the
    // three refusals are about the addresses and not about the constructor.
    assert!(PairingEndpoint::new(v4(47_001), a_fingerprint()).is_ok());
}

#[test]
fn surrounding_whitespace_does_not_change_what_a_string_means() {
    // A pairing string arrives from a text field or a scanner, and both add
    // whitespace. Refusing it would be technically correct and would make a
    // person retype something that was already right.
    let endpoint = PairingEndpoint::new(v4(47_001), a_fingerprint()).unwrap();
    let payload = endpoint.to_string();

    assert_eq!(
        PairingEndpoint::parse(&format!("  {payload}\n")),
        Ok(endpoint)
    );
    // But whitespace *inside* is not forgiven: that is a different string, and
    // guessing which space was meant is how a parser starts inventing.
    assert!(
        PairingEndpoint::parse(&payload.replace('|', " | ")).is_err(),
        "spaces inside the fields were accepted, so the parser is guessing"
    );
}

#[test]
fn a_code_still_wrapped_in_the_quotes_it_was_printed_with_is_read() {
    // **QYR-0369.** The `|` in a pairing string is a pipe in PowerShell and in
    // `cmd`, so `qyro send x --to QYRO1|192.168.1.5:49517|abc` never reaches
    // Qyro: the console splits the line and complains about something that is
    // not a command, in a message that does not mention Qyro. The CLI therefore
    // prints the code **with double quotes already on it**, and tells the person
    // to copy the whole thing.
    //
    // Which puts the quotes into every other place that code gets pasted. A
    // shell strips them; a text field does not. The phone's «type the code»
    // field, the CLI's own menu prompt and a QR decoder that was handed a
    // quoted string all receive them literally — and this is the one parser all
    // three go through, so this is where they are understood.
    //
    // Same argument as `surrounding_whitespace_does_not_change_what_a_string_means`
    // directly above: refusing would be technically correct and would make a
    // person retype something that was already right.
    let endpoint = PairingEndpoint::new(v4(47_001), a_fingerprint()).unwrap();
    let payload = endpoint.to_string();

    assert_eq!(
        PairingEndpoint::parse(&format!("\"{payload}\"")),
        Ok(endpoint),
        "a code copied with the double quotes it was printed with was refused"
    );
    assert_eq!(
        PairingEndpoint::parse(&format!("'{payload}'")),
        Ok(endpoint),
        "a code copied with single quotes was refused"
    );
    // Quotes outside the whitespace and inside it, both: what a person copies
    // from a terminal carries either.
    assert_eq!(
        PairingEndpoint::parse(&format!("  \"{payload}\"  \n")),
        Ok(endpoint)
    );
}

#[test]
fn but_a_lone_quote_is_still_a_broken_code() {
    // The control, and it is the whole reason the stripping is a matched pair
    // rather than «remove quotes wherever they are». A code copied one character
    // short ends in a stray quote, and a truncated fingerprint that *parses* is
    // worse than one that does not: it would dial the right address expecting
    // the wrong key.
    let endpoint = PairingEndpoint::new(v4(47_001), a_fingerprint()).unwrap();
    let payload = endpoint.to_string();

    assert!(
        PairingEndpoint::parse(&format!("\"{payload}")).is_err(),
        "an opening quote with no closing one was accepted"
    );
    assert!(
        PairingEndpoint::parse(&format!("{payload}\"")).is_err(),
        "a closing quote with no opening one was accepted"
    );
    assert!(
        PairingEndpoint::parse(&format!("'{payload}\"")).is_err(),
        "mismatched quotes were accepted"
    );
    // And the degenerate inputs, which are where a naive slice panics.
    assert!(PairingEndpoint::parse("\"").is_err());
    assert!(PairingEndpoint::parse("\"\"").is_err());
    assert!(PairingEndpoint::parse("").is_err());
}
