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
/// # Errors
///
/// [`SessionError::BadArgument`] when this build has no discovery backend. That
/// is a real state and not a bug: `mdns-sd` is `cfg(windows)` today and ADR-0043
/// says the in-tree implementation lands with the rest of phase 14.
pub fn browse(window: Duration) -> Result<Vec<FoundPeer>, SessionError> {
    browse_with(window)
}

#[cfg(windows)]
fn browse_with(window: Duration) -> Result<Vec<FoundPeer>, SessionError> {
    use qyro_net::{MdnsDiscovery, PeerDiscovery as _};

    let mut discovery = MdnsDiscovery::start().map_err(|_| SessionError::BadArgument)?;
    let found = discovery
        .browse(window)
        .map_err(|_| SessionError::PeerUnreachable)?;
    discovery.stop();

    let mut peers: Vec<FoundPeer> = Vec::new();
    for endpoint in found {
        let fingerprint = qyro_net::fingerprint_to_txt(endpoint.fingerprint());
        // Dedup by fingerprint. The first address a device announces is kept:
        // a second one for the same key is the same machine on another
        // interface, and offering both would ask a person to choose between two
        // spellings of the same answer.
        if peers.iter().any(|peer| peer.fingerprint == fingerprint) {
            continue;
        }
        peers.push(FoundPeer {
            address: endpoint.address(),
            fingerprint,
        });
    }
    Ok(peers)
}

#[cfg(not(windows))]
fn browse_with(_window: Duration) -> Result<Vec<FoundPeer>, SessionError> {
    // **Not a stub that pretends.** ADR-0043 §5 decides the in-tree mDNS
    // implementation, and until it lands this platform has no backend. Saying so
    // with a typed error is the difference between "nobody answered" and "this
    // build cannot ask" -- and a caller that cannot tell those apart will show a
    // person an empty list and let them conclude the other device is off.
    Err(SessionError::BadArgument)
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
