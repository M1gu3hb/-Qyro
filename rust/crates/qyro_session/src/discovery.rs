//! Finding the other device, and telling the truth when it cannot be found.
//!
//! Specification: ADR-0043, and ADR-0035 §7 for the service itself.
//!
//! # Why this module is one function and a lot of prose
//!
//! `qyro_net::MdnsDiscovery` has existed since phase 04b **with zero
//! consumers**, and `DiscoveryChannel.kt` since the same phase with no Dart
//! opening its channel. Two implementations, nobody calling either. This is the
//! seam that gives them a caller, and it is deliberately thin: ADR-0043 §5 says
//! **connect what exists, do not rewrite it.**
//!
//! # What it will not do
//!
//! It will not invent a peer. `browse` returns what answered inside a window,
//! and an empty list is a **true statement about this network** — routers with
//! client isolation are the common case, not the exception. A discovery layer
//! that retried until it found something would turn "nobody is there" into a
//! hang, and the typed pairing code exists precisely because this can fail.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::time::Duration;
use std::net::SocketAddr;

use crate::error::SessionError;

/// A device that answered, with everything needed to reach it.
///
/// The fingerprint is **hex text**, the same spelling the pairing code uses, so
/// the two halves of the product cannot disagree about what a fingerprint looks
/// like.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoundPeer {
    address: SocketAddr,
    fingerprint: String,
}

impl FoundPeer {
    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// The pairing code this peer would show, composed from what it announced.
    ///
    /// Same format as ADR-0035, built here rather than by the caller: two places
    /// that assemble the same string are two places that drift.
    #[must_use]
    pub fn pairing_string(&self) -> String {
        format!("QYRO1|{}|{}", self.address, self.fingerprint)
    }
}

/// Listens for `window` and returns whoever answered.
///
/// **Deduplicated by fingerprint, never by address** (ADR-0043 §5). An address
/// changes the moment a DHCP server appears or leaves, and the same device would
/// be listed twice — which on a peers screen reads as "someone is impersonating
/// them" rather than "the lease renewed".
///
/// **Two channels, both every time** (ADR-0043 §5). The platform's mDNS
/// responder where there is one, and the in-tree beacon on every interface —
/// simultaneously, not as a fallback chain, because a fallback that only starts
/// after a timeout is a fallback nobody waits for.
///
/// # Errors
///
/// None today, and the `Result` is kept rather than removed: every failure
/// either channel can have is one this function is right to survive. Until
/// phase 14 a non-Windows build returned [`SessionError::BadArgument`] because
/// it genuinely had no backend; it has one now.
///
/// An empty list still means **nobody answered**, which is a true statement
/// about this network and the reason the typed pairing code exists.
pub fn browse(window: Duration) -> Result<Vec<FoundPeer>, SessionError> {
    browse_with(window)
}

fn browse_with(window: Duration) -> Result<Vec<FoundPeer>, SessionError> {
    let mut peers: Vec<FoundPeer> = Vec::new();

    // Both channels run, and **neither one's failure ends the browse.** mDNS is
    // the one routers and phones already speak; the beacon is the one that
    // works on a direct cable where no responder exists at all. A machine with
    // only one of them working is the common case, not the exception.
    collect_mdns(window, &mut peers);
    collect_beacons(window, &mut peers);

    Ok(peers)
}

/// Adds whoever the platform's mDNS responder heard.
#[cfg(windows)]
fn collect_mdns(window: Duration, peers: &mut Vec<FoundPeer>) {
    use qyro_net::{MdnsDiscovery, PeerDiscovery as _};

    let Ok(mut discovery) = MdnsDiscovery::start() else {
        return;
    };
    let found = discovery.browse(window).unwrap_or_default();
    discovery.stop();

    for endpoint in found {
        let fingerprint = qyro_net::fingerprint_to_txt(endpoint.fingerprint());
        push_unique(peers, endpoint.address(), fingerprint);
    }
}

/// No platform responder on this target, so the beacon is the whole story.
///
/// **Not an error any more.** Until phase 14 this returned
/// [`SessionError::BadArgument`] because there was genuinely no backend; now
/// there is one, and the honest thing is to contribute nothing here rather than
/// to fail a browse that the beacon can answer.
#[cfg(not(windows))]
fn collect_mdns(_window: Duration, _peers: &mut Vec<FoundPeer>) {}

/// Adds whoever answered the in-tree beacon, on every interface at once.
///
/// **This is the production caller `Beacon` was written for**, and the reason
/// it is worth naming: the release binary was byte-identical with and without
/// `socket2` while nothing called it, because the linker discarded the whole
/// module. A capability with no caller does not ship — it just compiles.
fn collect_beacons(window: Duration, peers: &mut Vec<FoundPeer>) {
    let Ok(swarm) = qyro_net::BeaconSwarm::bind_all() else {
        // No interface accepted a beacon. On a cable still negotiating APIPA
        // that is the normal state for tens of seconds, not a fault.
        return;
    };

    // What this device says about itself, once per interface, each naming the
    // address that is reachable **on that interface**. Without a fingerprint
    // there is nothing meaningful to announce, so it listens only — still
    // useful, because the other device is announcing.
    let mine = crate::fingerprint().ok().map(|text| text.replace('-', ""));

    let heard = swarm.announce_and_collect(
        |interface| match &mine {
            Some(fingerprint) => format!(
                "{}|{interface}:{}|{fingerprint}",
                qyro_net::PAIRING_PREFIX,
                qyro_net::QYRO_PORT
            )
            .into_bytes(),
            None => Vec::new(),
        },
        window,
    );

    for (payload, _from) in heard {
        let Ok(text) = core::str::from_utf8(&payload) else {
            // A datagram that is not UTF-8 is somebody else's protocol sharing
            // the mDNS group, which is exactly what that group is for. Ignored
            // in silence, because a warning here would fire on every network
            // that has a printer.
            continue;
        };
        let Ok(endpoint) = qyro_net::PairingEndpoint::parse(text.trim()) else {
            continue;
        };
        // Ourselves included: the socket is a member of the group it sends to,
        // so our own announcement comes straight back. Dropped by fingerprint
        // rather than by address, because the address differs per interface and
        // the fingerprint does not.
        let fingerprint = qyro_net::fingerprint_to_txt(endpoint.fingerprint());
        if mine.as_deref() == Some(fingerprint.as_str()) {
            continue;
        }
        push_unique(peers, endpoint.address(), fingerprint);
    }
}

/// Keeps the first address seen for a fingerprint, and drops later ones.
///
/// **Dedup by fingerprint, never by address** (ADR-0043 §5). An address changes
/// the moment a DHCP lease renews, and the same device listed twice reads on a
/// peers screen as "someone is impersonating them" rather than "the lease
/// renewed". It also collapses the two channels: a device heard by both mDNS
/// and the beacon is one device.
fn push_unique(peers: &mut Vec<FoundPeer>, address: SocketAddr, fingerprint: String) {
    if peers.iter().any(|peer| peer.fingerprint == fingerprint) {
        return;
    }
    peers.push(FoundPeer {
        address,
        fingerprint,
    });
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::FoundPeer;

    fn peer(address: &str, fingerprint: &str) -> FoundPeer {
        FoundPeer {
            address: address.parse().expect("a literal address"),
            fingerprint: fingerprint.to_owned(),
        }
    }

    #[test]
    fn what_a_beacon_announces_is_what_the_other_side_parses() {
        // The round trip that decides whether discovery works at all: this side
        // composes a datagram per interface, the other side hands it to
        // `PairingEndpoint::parse`. They are written in two different modules
        // and there is nothing but this test making them the same language.
        //
        // Composed exactly as `collect_beacons` composes it -- the same
        // constants from the same crate, not a literal copied into a test that
        // would keep passing after production drifted.
        let interface: std::net::Ipv4Addr = "169.254.7.3".parse().expect("a literal");
        let fingerprint = "ab12cd34ab12cd34ab12cd34ab12cd34";
        let announced = format!(
            "{}|{interface}:{}|{fingerprint}",
            qyro_net::PAIRING_PREFIX,
            qyro_net::QYRO_PORT
        );

        let parsed = qyro_net::PairingEndpoint::parse(&announced)
            .expect("the beacon composes a code the engine's own parser refuses");
        assert_eq!(
            parsed.address().to_string(),
            format!("169.254.7.3:{}", qyro_net::QYRO_PORT),
            "an APIPA address survives the round trip -- it is the address a \
             direct cable produces, so a parser that dropped it would break the \
             one case the beacon exists for"
        );
        assert_eq!(
            qyro_net::fingerprint_to_txt(parsed.fingerprint()),
            fingerprint
        );

        // The control: the same string missing the port is not silently
        // accepted with a default. A code that parses when it should not is how
        // a device gets dialled on the wrong port and reported as offline.
        assert!(
            qyro_net::PairingEndpoint::parse(&format!(
                "{}|{interface}|{fingerprint}",
                qyro_net::PAIRING_PREFIX
            ))
            .is_err()
        );
    }

    #[test]
    fn the_beacon_payload_fits_the_buffer_that_receives_it() {
        // Two constants in two crates, and the failure if they disagree is a
        // truncated datagram that parses as garbage -- which looks exactly like
        // "the other device is not there".
        let longest = format!(
            "{}|255.255.255.255:{}|{}",
            qyro_net::PAIRING_PREFIX,
            qyro_net::QYRO_PORT,
            "f".repeat(32)
        );
        assert!(
            longest.len() < qyro_net::MAX_BEACON_PAYLOAD,
            "the widest pairing string is {} bytes and the beacon buffer is {}",
            longest.len(),
            qyro_net::MAX_BEACON_PAYLOAD
        );
    }

    #[test]
    fn a_found_peer_composes_the_same_code_a_person_would_type() {
        // The format is ADR-0035's, and it is assembled here rather than by
        // every caller: two places building the same string are two places that
        // drift, which is how the v1.0 ended up with a code nobody could show.
        let found = peer("192.168.1.9:49517", "ab12cd34ab12cd34ab12cd34ab12cd34");
        assert_eq!(
            found.pairing_string(),
            "QYRO1|192.168.1.9:49517|ab12cd34ab12cd34ab12cd34ab12cd34"
        );
    }

    #[test]
    fn and_that_code_is_the_one_the_engine_parses_back() {
        // Falsifiability: a string that merely looks right proves nothing. This
        // hands it to the same parser the other device uses.
        //
        // The fingerprint is thirty-two hex characters because that is what a
        // fingerprint is. The first draft used eight and the parser refused it,
        // which was the parser doing its job -- a code with a short fingerprint
        // is not a code.
        let found = peer("192.168.1.9:49517", "ab12cd34ab12cd34ab12cd34ab12cd34");
        let parsed = crate::parse_pairing(&found.pairing_string());
        assert_eq!(
            parsed.as_deref(),
            Ok("192.168.1.9:49517"),
            "the code this device composes is not one the engine accepts"
        );
    }
}
