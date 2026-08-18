//! Finding a peer without being told where it is.
//!
//! Specification: `docs/adr/ADR-0035-discovery-and-pairing.md` §§6–7 and
//! amendment 1.
//!
//! # The trait is the point, and the platforms are the reason
//!
//! Discovery is the one part of this product that **cannot** be Rust on mobile.
//! Android and iOS both put the local-network gate *below* the socket API, so a
//! `UdpSocket` opened from Rust does not escape it; the way through is
//! `NsdManager` and `NWBrowser`, in Kotlin and in Swift. On Windows it is
//! Rust's territory and `mdns-sd` does it.
//!
//! So the shape is a trait with two implementations that share nothing: one
//! here under `cfg(windows)`, one behind a platform channel. What they agree on
//! is [`PeerEndpoint`], which is a socket address and a fingerprint and nothing
//! else.
//!
//! # What is advertised is what the whole network reads
//!
//! ADR-0035 §6: the TXT record carries **the public fingerprint and nothing
//! more**. Not the user's name, not the device's, not the operating system, not
//! the version. The fingerprint is already public by design — it travels in
//! clear inside the handshake — and it is what lets an interface show who it is
//! about to connect to *before* connecting.
//!
//! # And the trap that gives no error
//!
//! Anything that is not `NsdManager` needs `WifiManager.MulticastLock` on
//! Android: the Wi-Fi stack filters multicast **below** the socket, so
//! `join_multicast_v4` succeeds and receives nothing, with no error at all. That
//! is why the mobile side is a platform channel and not a socket.

use std::net::SocketAddr;
use std::time::Duration;

use crate::error::NetError;
use crate::pairing::PAIRING_FINGERPRINT_LEN;

/// The service type every Qyro device advertises under.
pub const SERVICE_TYPE: &str = "_qyro._tcp.local.";

/// The TXT key the fingerprint travels in.
pub const TXT_FINGERPRINT_KEY: &str = "fp";

/// A peer that answered, and the identity it claims.
///
/// **Claims.** Nothing here is authenticated: a browse result is a machine on
/// the network saying a name and a number. It becomes a fact only when the
/// handshake verifies it (ADR-0035 §3), and the fingerprint here exists so an
/// interface can show *who it is about to talk to* and so the pairing rule of
/// §2.1 has something to compare against.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerEndpoint {
    address: SocketAddr,
    fingerprint: [u8; PAIRING_FINGERPRINT_LEN],
}

impl PeerEndpoint {
    #[must_use]
    pub const fn new(address: SocketAddr, fingerprint: [u8; PAIRING_FINGERPRINT_LEN]) -> Self {
        Self {
            address,
            fingerprint,
        }
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &[u8; PAIRING_FINGERPRINT_LEN] {
        &self.fingerprint
    }
}

/// Announcing and finding, behind one name.
///
/// Zero dependencies in this trait on purpose: it is what the mobile channels
/// implement too, and a trait that named a crate would make the mobile side
/// depend on something it cannot use.
pub trait PeerDiscovery {
    /// Announces this device at `address` under `fingerprint`.
    ///
    /// # Errors
    ///
    /// Whatever the platform's responder reports, as a [`NetError`].
    fn advertise(
        &mut self,
        address: SocketAddr,
        fingerprint: &[u8; PAIRING_FINGERPRINT_LEN],
    ) -> Result<(), NetError>;

    /// Collects what answers within `window`.
    ///
    /// A window and not a callback, because the engine is synchronous and a
    /// callback would need a runtime this project does not have (ADR-0028).
    ///
    /// # Errors
    ///
    /// As [`Self::advertise`].
    fn browse(&mut self, window: Duration) -> Result<Vec<PeerEndpoint>, NetError>;

    /// Stops announcing. Idempotent.
    fn stop(&mut self);
}

/// Renders a fingerprint for the TXT record: lowercase hex, no separators.
///
/// The same spelling the pairing string uses, so the two cannot disagree about
/// what a fingerprint looks like on the wire.
#[must_use]
pub fn fingerprint_to_txt(fingerprint: &[u8; PAIRING_FINGERPRINT_LEN]) -> String {
    let mut out = String::with_capacity(PAIRING_FINGERPRINT_LEN * 2);
    for byte in fingerprint {
        out.push(nibble_char(byte >> 4));
        out.push(nibble_char(byte & 0x0F));
    }
    out
}

/// Reads one back, or nothing if it is not thirty-two lowercase hex characters.
///
/// Total and strict, for the same reason the pairing string is: a fingerprint
/// with two accepted spellings is a fingerprint two devices can render
/// differently, and comparing it out loud is the only thing it is for.
#[must_use]
pub fn fingerprint_from_txt(text: &str) -> Option<[u8; PAIRING_FINGERPRINT_LEN]> {
    let bytes = text.as_bytes();
    if bytes.len() != PAIRING_FINGERPRINT_LEN * 2 {
        return None;
    }
    let mut out = [0_u8; PAIRING_FINGERPRINT_LEN];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let (Some(high), Some(low)) = (pair.first(), pair.get(1)) else {
            return None;
        };
        let (Some(high), Some(low)) = (nibble_value(*high), nibble_value(*low)) else {
            return None;
        };
        *slot = (high << 4) | low;
    }
    Some(out)
}

const fn nibble_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => '0',
    }
}

const fn nibble_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// The Windows responder, over `mdns-sd`.
#[cfg(windows)]
pub use windows_mdns::MdnsDiscovery;

#[cfg(windows)]
mod windows_mdns {
    use std::net::SocketAddr;
    use std::time::{Duration, Instant};

    use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

    use super::{
        PAIRING_FINGERPRINT_LEN, PeerDiscovery, PeerEndpoint, SERVICE_TYPE, TXT_FINGERPRINT_KEY,
        fingerprint_from_txt, fingerprint_to_txt,
    };
    use crate::error::{NetError, SocketOp};

    /// The one refusal this module can produce.
    ///
    /// `mdns-sd` has its own error type and none of its variants mean anything
    /// to a caller of this crate: what a caller can do about "the responder
    /// would not start" is the same whatever the daemon's reason was. Collapsed
    /// deliberately, and named so the operation still says which step failed.
    fn responder_failed(operation: SocketOp) -> NetError {
        NetError::SocketFailed {
            operation,
            kind: std::io::ErrorKind::Other,
        }
    }

    /// mDNS on Windows, with its own thread and channels.
    ///
    /// No async runtime: `mdns-sd` runs a thread and talks over channels, which
    /// is the shape this engine already has (ADR-0028). That was the deciding
    /// property, not the download count.
    pub struct MdnsDiscovery {
        daemon: ServiceDaemon,
        registered: Option<String>,
    }

    impl MdnsDiscovery {
        /// Starts the responder daemon.
        ///
        /// # Errors
        ///
        /// [`NetError::SocketFailed`] when the daemon cannot start — which on
        /// Windows means the network stack refused, not that mDNS is absent.
        pub fn start() -> Result<Self, NetError> {
            let daemon = ServiceDaemon::new().map_err(|_| responder_failed(SocketOp::Bind))?;
            Ok(Self {
                daemon,
                registered: None,
            })
        }
    }

    impl PeerDiscovery for MdnsDiscovery {
        fn advertise(
            &mut self,
            address: SocketAddr,
            fingerprint: &[u8; PAIRING_FINGERPRINT_LEN],
        ) -> Result<(), NetError> {
            let text = fingerprint_to_txt(fingerprint);
            // The instance name is derived from the fingerprint and nothing
            // else. A device name would leak into every café this thing is ever
            // switched on in (ADR-0035 §6).
            let instance = text.get(..12).unwrap_or(text.as_str()).to_owned();
            let properties = [(TXT_FINGERPRINT_KEY, text.as_str())];

            let info = ServiceInfo::new(
                SERVICE_TYPE,
                &instance,
                &format!("{instance}.local."),
                address.ip(),
                address.port(),
                &properties[..],
            )
            .map_err(|_| responder_failed(SocketOp::Bind))?;

            let full = info.get_fullname().to_owned();
            self.daemon
                .register(info)
                .map_err(|_| responder_failed(SocketOp::Bind))?;
            self.registered = Some(full);
            Ok(())
        }

        fn browse(&mut self, window: Duration) -> Result<Vec<PeerEndpoint>, NetError> {
            let receiver = self
                .daemon
                .browse(SERVICE_TYPE)
                .map_err(|_| responder_failed(SocketOp::Read))?;

            let deadline = Instant::now() + window;
            let mut found = Vec::new();
            while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
                let Ok(event) = receiver.recv_timeout(remaining) else {
                    break;
                };
                let ServiceEvent::ServiceResolved(info) = event else {
                    continue;
                };
                let Some(text) = info.get_property_val_str(TXT_FINGERPRINT_KEY) else {
                    // No fingerprint is not a Qyro service, whatever it says it
                    // is. Skipped rather than shown with a blank identity: an
                    // entry a person cannot verify is one they should not see.
                    continue;
                };
                let Some(fingerprint) = fingerprint_from_txt(text) else {
                    continue;
                };
                // `get_addresses` yields `ScopedIp`, because a link-local IPv6
                // address is only meaningful with the interface it arrived on.
                // Qyro dials with `std::net`, which has no scope, so a scoped
                // address is dropped rather than dialled without its scope --
                // that would connect to whatever answers on the default
                // interface, which is not the peer that was found.
                for scoped in info.get_addresses() {
                    let Ok(ip) = scoped.to_string().parse() else {
                        continue;
                    };
                    found.push(PeerEndpoint::new(
                        SocketAddr::new(ip, info.get_port()),
                        fingerprint,
                    ));
                }
            }
            found.sort_by_key(PeerEndpoint::address);
            found.dedup();
            Ok(found)
        }

        fn stop(&mut self) {
            if let Some(full) = self.registered.take() {
                let _ = self.daemon.unregister(&full);
            }
        }
    }

    impl Drop for MdnsDiscovery {
        fn drop(&mut self) {
            self.stop();
            let _ = self.daemon.shutdown();
        }
    }
}
