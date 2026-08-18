//! Announcing on every interface, because the operating system picks the wrong one.
//!
//! Specification: ADR-0043 §5. The measurements are `R8` §8.
//!
//! # Why this is not `std`
//!
//! `std::net::UdpSocket::join_multicast_v4` takes an interface **address**, and
//! `join_multicast_v6` takes an index where **0 means "you choose"** — and the
//! operating system chooses badly on exactly the machine that needs this to
//! work: one with Wi-Fi, Ethernet, a VPN adapter, Hyper-V and Docker all
//! present. `std` exposes no `IPV6_MULTICAST_IF` at all.
//!
//! So `socket2` (ADR-0043 §5, pre-authorised in `R8` §7), and **one socket per
//! interface** rather than one socket and a hope.
//!
//! # Why broadcast runs alongside and not instead
//!
//! `R8` §8. mDNS works without a router — it is pure link-local multicast — but
//! **Windows ships no usable mDNS responder** ([MS-WPO] lists DNS, NetBIOS/WINS,
//! LLMNR and PNRP; mDNS is absent), and some switches and Wi-Fi stacks drop
//! multicast silently. Broadcast to `255.255.255.255` is the crudest thing on
//! the wire and RFC 1122 says it *«will be received by every host on the
//! connected physical network»*.
//!
//! Both fire, every round. Not a fallback chain: **a fallback that only runs
//! after a timeout is a fallback nobody waits for.**
//!
//! # The trap that gives no error
//!
//! On Android, anything that is not `NsdManager` needs
//! `WifiManager.MulticastLock`: the Wi-Fi stack filters multicast **below the
//! socket**, so `join_multicast_v4` **succeeds and receives nothing, with no
//! error at all**. Noted since phase 04 and still true. That is why Android
//! keeps `NsdManager` and this module is for the desktop side.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::{Duration, Instant};

use crate::error::{NetError, SocketOp};

/// The port every beacon speaks on.
///
/// 5353 is mDNS's (RFC 6762), and the broadcast half uses it too so that one
/// firewall permission covers both. A second port would be a second dialog on
/// the machine where dialogs are hardest to get (`R8` §9).
pub const BEACON_PORT: u16 = 5353;

/// The multicast group, RFC 6762.
pub const MDNS_GROUP_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// Where a round of announcements is sent.
///
/// All three, every round. `255.255.255.255` is the limited broadcast of
/// RFC 1122 — the one that does not need to know the subnet mask, which matters
/// because on a direct cable **there may not be a correct mask yet**.
#[must_use]
pub fn announcement_targets() -> Vec<SocketAddr> {
    vec![
        SocketAddr::V4(SocketAddrV4::new(MDNS_GROUP_V4, BEACON_PORT)),
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, BEACON_PORT)),
    ]
}

/// A socket bound to one interface, ready to shout and to listen.
pub struct Beacon {
    socket: UdpSocket,
    interface: Ipv4Addr,
}

impl Beacon {
    /// Binds to `interface` and joins the mDNS group **on that interface**.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with the operation that refused.
    pub fn bind(interface: Ipv4Addr) -> Result<Self, NetError> {
        use socket2::{Domain, Protocol, Socket, Type};

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|_| {
            NetError::SocketFailed {
                operation: SocketOp::Bind,
                kind: std::io::ErrorKind::Other,
            }
        })?;

        // Several Qyro instances on one machine, and other mDNS speakers, all
        // want this port. Without reuse the second one fails and the person is
        // told the network is broken when the truth is that Bonjour is running.
        socket
            .set_reuse_address(true)
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: std::io::ErrorKind::Other,
            })?;

        socket
            .bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, BEACON_PORT)).into())
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Bind,
                kind: std::io::ErrorKind::Other,
            })?;

        // **The whole reason socket2 is here.** Naming the interface rather than
        // passing UNSPECIFIED, so the packets leave the cable the person plugged
        // in and not the VPN adapter the operating system prefers.
        socket
            .set_multicast_if_v4(&interface)
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: std::io::ErrorKind::Other,
            })?;
        socket
            .join_multicast_v4(&MDNS_GROUP_V4, &interface)
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: std::io::ErrorKind::Other,
            })?;
        socket
            .set_broadcast(true)
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: std::io::ErrorKind::Other,
            })?;

        // TTL stays at its default of 1 **on purpose**: the packets must not
        // leave the local link. Raising it would make Qyro audible one router
        // away, which is not a feature this product wants.

        socket
            .set_read_timeout(Some(Duration::from_millis(250)))
            .map_err(|_| NetError::SocketFailed {
                operation: SocketOp::Configure,
                kind: std::io::ErrorKind::Other,
            })?;

        Ok(Self {
            socket: socket.into(),
            interface,
        })
    }

    /// The interface this beacon speaks on.
    #[must_use]
    pub const fn interface(&self) -> Ipv4Addr {
        self.interface
    }

    /// Sends `payload` to every target, and reports how many left.
    ///
    /// **A partial send is not an error.** A machine with IPv4 broadcast
    /// filtered still reaches the multicast group, and refusing the whole round
    /// because one of two targets bounced would turn a working network into a
    /// failure message.
    ///
    /// # Errors
    ///
    /// Only when **nothing** could be sent, because then the round really did
    /// achieve nothing.
    pub fn announce(&self, payload: &[u8]) -> Result<usize, NetError> {
        let mut sent = 0;
        for target in announcement_targets() {
            if self.socket.send_to(payload, target).is_ok() {
                sent += 1;
            }
        }
        if sent == 0 {
            return Err(NetError::SocketFailed {
                operation: SocketOp::Write,
                kind: std::io::ErrorKind::Other,
            });
        }
        Ok(sent)
    }

    /// Reads one datagram, or `None` when the read window expired.
    ///
    /// An expiry is **not** an error: on a quiet network most reads time out,
    /// and a caller that treated that as a failure would give up on the normal
    /// case.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] only for a genuine socket failure.
    pub fn listen_once(&self, buffer: &mut [u8]) -> Result<Option<(usize, SocketAddr)>, NetError> {
        match self.socket.recv_from(buffer) {
            Ok(received) => Ok(Some(received)),
            Err(error) if crate::stream::is_read_timeout(error.kind()) => Ok(None),
            Err(_) => Err(NetError::SocketFailed {
                operation: SocketOp::Read,
                kind: std::io::ErrorKind::Other,
            }),
        }
    }
}

/// Every IPv4 interface a beacon should speak on.
///
/// Loopback is dropped because a beacon on it reaches only this machine, and
/// **nothing else is dropped** — not the VPN adapter, not the Hyper-V switch,
/// not the interface that looks pointless. Filtering by name is how the one
/// cable that mattered gets excluded on somebody else's laptop, and an extra
/// datagram on a virtual switch costs a few hundred bytes.
///
/// An empty result is a true statement: no usable interface is up yet, which on
/// a direct cable is the normal state for tens of seconds (`R8` §8, and
/// `qyro_session::wait_for_link` is what waits it out).
#[must_use]
pub fn beacon_interfaces() -> Vec<Ipv4Addr> {
    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return Vec::new();
    };
    let mut found: Vec<Ipv4Addr> = Vec::new();
    for interface in interfaces {
        if let std::net::IpAddr::V4(address) = interface.ip()
            && !address.is_loopback()
            && !address.is_unspecified()
            && !found.contains(&address)
        {
            found.push(address);
        }
    }
    found
}

/// One beacon per interface, so a round really does leave by every cable.
///
/// **A partly-bound swarm is not a failure.** An interface can refuse the join
/// —a VPN adapter that is down, a virtual switch with no host binding— and the
/// rounds that do go out are the ones that find the peer. Refusing to start
/// because one of six adapters said no would be the operating system's opinion
/// deciding the product's behaviour.
pub struct BeaconSwarm {
    beacons: Vec<Beacon>,
}

impl BeaconSwarm {
    /// Binds a beacon on every interface that accepts one.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] only when **no** interface accepted, because
    /// then there is nothing to announce on and saying so beats pretending.
    pub fn bind_all() -> Result<Self, NetError> {
        let beacons: Vec<Beacon> = beacon_interfaces()
            .into_iter()
            .filter_map(|interface| Beacon::bind(interface).ok())
            .collect();
        if beacons.is_empty() {
            return Err(NetError::SocketFailed {
                operation: SocketOp::Bind,
                kind: std::io::ErrorKind::AddrNotAvailable,
            });
        }
        Ok(Self { beacons })
    }

    /// How many interfaces this swarm actually speaks on.
    #[must_use]
    pub fn width(&self) -> usize {
        self.beacons.len()
    }

    /// Announces from every interface, letting each one say **its own** address.
    ///
    /// `build` is handed the interface the datagram is about to leave by, and
    /// this is the point of the whole module: a machine with Wi-Fi and a direct
    /// cable has two addresses, and the peer on the cable can only reach one of
    /// them. Announcing a single payload everywhere would hand that peer an
    /// address it cannot route to and call it discovery.
    pub fn announce_each<B>(&self, build: B) -> usize
    where
        B: Fn(Ipv4Addr) -> Vec<u8>,
    {
        self.beacons
            .iter()
            .filter(|beacon| {
                let payload = build(beacon.interface());
                beacon.announce(&payload).is_ok()
            })
            .count()
    }

    /// Announces once, then gathers whatever arrives until `window` expires.
    ///
    /// Returns the raw payloads with their sender. Parsing belongs to the
    /// caller: this module moves datagrams and has no opinion about what a peer
    /// is, which is what keeps the wire format in exactly one place
    /// ([`crate::PairingEndpoint`]).
    #[must_use]
    pub fn announce_and_collect<B>(&self, build: B, window: Duration) -> Vec<(Vec<u8>, IpAddr)>
    where
        B: Fn(Ipv4Addr) -> Vec<u8>,
    {
        let _ = self.announce_each(build);

        let deadline = Instant::now() + window;
        let mut heard: Vec<(Vec<u8>, IpAddr)> = Vec::new();
        let mut buffer = [0_u8; MAX_BEACON_PAYLOAD];

        while Instant::now() < deadline {
            let mut quiet = true;
            for beacon in &self.beacons {
                if let Ok(Some((len, from))) = beacon.listen_once(&mut buffer)
                    && let Some(datagram) = buffer.get(..len)
                {
                    // Our own announcement comes straight back: the socket is a
                    // member of the group it sends to. Dropping it here by
                    // content would need to know the payload's shape, so it is
                    // left in and the caller drops itself by fingerprint --
                    // the one field that identifies a device on any interface.
                    heard.push((datagram.to_vec(), from.ip()));
                    quiet = false;
                }
            }
            // Every socket timed out, which is the normal state of a quiet
            // network. Yielding keeps this from becoming a spin on the machine
            // whose fan the user can hear.
            if quiet {
                std::thread::yield_now();
            }
        }
        heard
    }
}

/// The largest beacon datagram that is read.
///
/// A pairing string is under 80 bytes; 512 leaves room for a future field and
/// stays far below the 1 280-byte floor every link is required to carry, so a
/// beacon never depends on fragmentation working.
pub const MAX_BEACON_PAYLOAD: usize = 512;

/// 1 280 bytes is the floor every link is required to carry without
/// fragmenting. Asserted at compile time rather than in a test, because clippy
/// is right that a runtime comparison of two constants is optimised away and
/// proves nothing — this one cannot be optimised away, it simply refuses to
/// build.
const _: () = assert!(
    MAX_BEACON_PAYLOAD < 1280,
    "a beacon larger than the guaranteed MTU depends on reassembly working, \
     which fails as 'discovery is flaky on that one switch'"
);

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::{BEACON_PORT, MAX_BEACON_PAYLOAD, MDNS_GROUP_V4, announcement_targets};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[test]
    fn every_round_fires_multicast_and_broadcast_together() {
        // ADR-0043 §5: **simultaneous, not a fallback chain.** A fallback that
        // only runs after a timeout is a fallback nobody waits for, and the
        // machine that needs it most is the one where the person is already
        // deciding the software does not work.
        let targets = announcement_targets();
        assert_eq!(targets.len(), 2, "a round stopped firing on one of the two");

        let addresses: Vec<IpAddr> = targets.iter().map(SocketAddr::ip).collect();
        assert!(
            addresses.contains(&IpAddr::V4(MDNS_GROUP_V4)),
            "the mDNS group is gone: {addresses:?}"
        );
        assert!(
            addresses.contains(&IpAddr::V4(Ipv4Addr::BROADCAST)),
            "the limited broadcast is gone, and it is the one that works without \
             a correct subnet mask -- which a direct cable may not have yet"
        );
    }

    #[test]
    fn both_targets_share_one_port_so_one_permission_covers_them() {
        // `R8` §9: the firewall grants inbound **once per program and port**. A
        // second port would be a second dialog on the machine where dialogs are
        // hardest to obtain.
        for target in announcement_targets() {
            assert_eq!(
                target.port(),
                BEACON_PORT,
                "a target moved off the shared port, which costs a second \
                 firewall permission"
            );
        }
    }

    #[test]
    fn a_beacon_payload_fits_far_inside_what_every_link_must_carry() {
        // The MTU half is a `const _: () = assert!(...)` next to the constant,
        // where it cannot be optimised out. What is left here is the half a
        // compile-time assertion cannot express: that the buffer is comfortably
        // above a real pairing string, or it would silently truncate the thing
        // it exists to carry.
        let realistic = format!("QYRO1|192.168.100.136:49517|{}", "a".repeat(32));
        assert!(
            realistic.len() < MAX_BEACON_PAYLOAD,
            "a pairing string is {} bytes and the buffer is {MAX_BEACON_PAYLOAD}",
            realistic.len()
        );
    }

    #[test]
    fn the_interface_list_never_offers_loopback() {
        // Loopback reaches only this machine, so a beacon on it is a round that
        // cannot possibly find anybody. This runs on whatever interfaces the
        // machine really has -- including none, which is why the assertion is
        // over the list rather than about its length.
        for address in super::beacon_interfaces() {
            assert!(
                !address.is_loopback(),
                "loopback got into the beacon list: {address}"
            );
            assert!(!address.is_unspecified());
        }
    }

    #[test]
    fn the_group_is_the_one_rfc_6762_names() {
        // Transcribed constants get transcribed wrong. 224.0.0.251 is mDNS.
        assert_eq!(MDNS_GROUP_V4.octets(), [224, 0, 0, 251]);
        assert_eq!(BEACON_PORT, 5353);
        // And it really is a multicast address, not merely a number that looks
        // like one -- the control for the assertion above.
        assert!(MDNS_GROUP_V4.is_multicast());
        assert!(Ipv4Addr::BROADCAST.is_broadcast());
    }
}
