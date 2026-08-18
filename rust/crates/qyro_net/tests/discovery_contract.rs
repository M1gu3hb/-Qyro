//! Discovery: the TXT encoding everywhere, and the responder where it exists.
//!
//! ADR-0035 §§6–7. The mDNS half is `cfg(windows)` because on Android and iOS
//! the local-network gate sits below the socket API and a Rust socket does not
//! escape it — the mobile side is a platform channel, and it is not this file.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use qyro_net::{
    PAIRING_FINGERPRINT_LEN, PeerEndpoint, SERVICE_TYPE, TXT_FINGERPRINT_KEY, fingerprint_from_txt,
    fingerprint_to_txt,
};

fn a_fingerprint() -> [u8; PAIRING_FINGERPRINT_LEN] {
    let mut bytes = [0_u8; PAIRING_FINGERPRINT_LEN];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = (index as u8).wrapping_mul(31).wrapping_add(5);
    }
    bytes
}

#[test]
fn a_fingerprint_survives_the_txt_record_it_travels_in() {
    let original = a_fingerprint();
    let text = fingerprint_to_txt(&original);

    assert_eq!(text.len(), PAIRING_FINGERPRINT_LEN * 2);
    assert_eq!(fingerprint_from_txt(&text), Some(original));

    // One flipped bit changes the text and not its length, so the equality
    // above can see a different peer rather than a different shape.
    let mut altered = original;
    altered[3] ^= 0x01;
    let other = fingerprint_to_txt(&altered);
    assert_ne!(other, text);
    assert_eq!(other.len(), text.len());
}

#[test]
fn the_txt_spelling_is_the_same_one_the_pairing_string_uses() {
    // Two devices that spell a fingerprint differently cannot compare it, and a
    // person reading it out loud is the whole mechanism. Lowercase hex, no
    // separators, in both places.
    let text = fingerprint_to_txt(&a_fingerprint());
    assert!(
        text.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{text} is not lowercase hexadecimal"
    );
    // And the reader refuses the other spelling rather than accepting two.
    assert_eq!(fingerprint_from_txt(&text.to_uppercase()), None);
    assert_eq!(fingerprint_from_txt(""), None);
    assert_eq!(fingerprint_from_txt(&text[..text.len() - 1]), None);
    assert_eq!(fingerprint_from_txt(&format!("{text}0")), None);
}

#[test]
fn the_advertised_record_carries_a_fingerprint_and_nothing_else() {
    // ADR-0035 §6. What is announced is read by the whole network, including
    // the café. This asserts on the *constants*, because they are what a future
    // edit would widen — a record that gained a device name would do it here.
    assert_eq!(SERVICE_TYPE, "_qyro._tcp.local.");
    assert_eq!(TXT_FINGERPRINT_KEY, "fp");
    // Deliberately short: a TXT key is on the wire in every announcement.
    assert!(TXT_FINGERPRINT_KEY.len() <= 4);
}

#[test]
fn an_endpoint_keeps_the_address_and_the_fingerprint_it_was_given() {
    let address = "192.168.1.7:47001".parse().unwrap();
    let endpoint = PeerEndpoint::new(address, a_fingerprint());

    assert_eq!(endpoint.address(), address);
    assert_eq!(endpoint.fingerprint(), &a_fingerprint());
    // Two endpoints that differ only in fingerprint are different endpoints:
    // the same machine announcing a new key is not the same peer.
    let mut altered = a_fingerprint();
    altered[0] ^= 0xFF;
    assert_ne!(PeerEndpoint::new(address, altered), endpoint);
}

/// The responder runs and answers, on the platform where it exists.
///
/// Not asserting that it *finds itself*: mDNS on a CI runner depends on the
/// loopback interface carrying multicast, which is a property of the runner and
/// not of this code. What is asserted is that the daemon starts, registers and
/// browses without refusing — the three calls that can fail — and that a browse
/// that finds nothing is an empty list rather than an error.
///
/// A test that required self-discovery would be a test that fails for reasons
/// this repository does not control, and a flaky test is a test that gets
/// muted.
#[cfg(windows)]
#[test]
fn the_windows_responder_starts_registers_and_browses() {
    use std::time::Duration;

    use qyro_net::{MdnsDiscovery, PeerDiscovery};

    let Ok(mut discovery) = MdnsDiscovery::start() else {
        // A runner with no usable network stack is not a failure of this code,
        // and saying so out loud beats a green tick for something that did not
        // run.
        eprintln!("the mDNS daemon would not start on this host; nothing asserted");
        return;
    };

    let address = "127.0.0.1:47001".parse().unwrap();
    discovery
        .advertise(address, &a_fingerprint())
        .expect("registering a service refused");

    let found = discovery
        .browse(Duration::from_millis(750))
        .expect("browsing refused");
    // Whatever answered, every entry is well formed: an address that can be
    // dialled and a fingerprint of the right length. An empty list is a valid
    // answer on a host that filters multicast.
    for endpoint in &found {
        assert_ne!(endpoint.address().port(), 0);
        assert_eq!(endpoint.fingerprint().len(), PAIRING_FINGERPRINT_LEN);
    }

    discovery.stop();
    // Stopping twice is not an error, because a caller that already stopped
    // should not have to remember that it did.
    discovery.stop();
}
