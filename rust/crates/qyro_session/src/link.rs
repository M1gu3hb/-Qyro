//! Waiting for a direct cable to come up, and saying why it takes so long.
//!
//! Specification: ADR-0043 §2, and the measurement behind it is `R8` §8.
//!
//! # The thing this module exists to prevent
//!
//! A person plugs a cable between two machines and **nothing works for the best
//! part of a minute**. Windows has 169.254/16 enabled by default, but the DHCP
//! client tries first and has to fail before APIPA takes over, and `R8` §8
//! measures that as tens of seconds.
//!
//! Software that shows nothing during that window teaches people the cable does
//! not work. Software that fails at five seconds teaches them the same thing
//! faster. So this **counts out loud**: it reports every second, says the wait
//! is normal, and at sixty seconds it says what to try — not «error».

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
use std::net::{IpAddr, Ipv4Addr};

/// How long a direct cable is given before the advice changes.
///
/// `R8` §8: the DHCP client tries and fails before APIPA assigns, and in
/// practice that is tens of seconds. Sixty is the round number above the
/// measured range — chosen so the advice arrives after the normal case has had
/// its chance, not during it.
pub const APIPA_BUDGET: Duration = Duration::from_secs(60);

/// What the wait is doing right now, so an interface can draw it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinkState {
    /// A usable address exists. The wait is over.
    Ready(IpAddr),
    /// Still waiting, and how long it has been.
    Waiting { elapsed: Duration },
    /// Sixty seconds with nothing. **Not an error** — advice.
    ///
    /// Auto-MDI-X lives in IEEE 802.3 clause 40.4.4, which is the **1000BASE-T**
    /// clause: a 10/100-only NIC may not have it, and that is exactly the NIC in
    /// the machine this product was built for. So the advice is a crossover
    /// cable, and it is advice rather than a failure because the cable may still
    /// come up at sixty-one seconds.
    StillNothing,
}

/// Whether an address is one a peer on the same cable could reach.
///
/// Loopback is excluded because it reaches only oneself. **APIPA is
/// included** — `169.254/16` is precisely what a direct cable produces, and a
/// filter that dropped it would reject the one case this module is for.
#[must_use]
pub fn is_reachable_by_a_peer(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => !v4.is_loopback() && !v4.is_unspecified(),
        // IPv6 link-local is usable **inside** this machine and cannot be typed
        // into another one, because the zone-id is local to the node
        // (RFC 4007). It is excluded here because this function answers "can I
        // put this in a pairing code", and the answer for `fe80::` is no.
        IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_unspecified() && !is_link_local_v6(v6),
    }
}

/// `fe80::/10`, without waiting for the `std` method to stabilise.
#[must_use]
pub const fn is_link_local_v6(address: std::net::Ipv6Addr) -> bool {
    let [first, second, ..] = address.octets();
    first == 0xfe && (second & 0xc0) == 0x80
}

/// Whether this is an APIPA address, which means the cable came up without DHCP.
///
/// Worth naming rather than checking inline: seeing `169.254.x.x` is **good
/// news** on a direct cable and usually bad news on a corporate LAN, and an
/// interface that can tell the difference can say the right thing.
#[must_use]
pub const fn is_apipa(address: Ipv4Addr) -> bool {
    address.is_link_local()
}

/// Polls `probe` until it yields a reachable address or the budget runs out.
///
/// `on_state` is called **every tick**, which is what makes the wait visible.
/// Taking a closure rather than printing keeps this testable and lets the CLI
/// and the GUI draw it differently without two copies of the waiting logic.
///
/// The `probe`/`now` injection is what makes the sixty-second path a test that
/// runs in microseconds instead of a test nobody runs.
pub fn wait_for_link<P, N, S>(
    mut probe: P,
    mut now: N,
    mut on_state: S,
    budget: Duration,
    tick: Duration,
) -> LinkState
where
    P: FnMut() -> Vec<IpAddr>,
    N: FnMut() -> Duration,
    S: FnMut(&LinkState),
{
    let start = now();
    loop {
        if let Some(address) = probe().into_iter().find(|a| is_reachable_by_a_peer(*a)) {
            let state = LinkState::Ready(address);
            on_state(&state);
            return state;
        }

        let elapsed = now().saturating_sub(start);
        if elapsed >= budget {
            let state = LinkState::StillNothing;
            on_state(&state);
            return state;
        }

        on_state(&LinkState::Waiting { elapsed });
        std::thread::sleep(tick);
    }
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

    use super::{
        APIPA_BUDGET, LinkState, is_apipa, is_link_local_v6, is_reachable_by_a_peer, wait_for_link,
    };
    use core::time::Duration;
    use std::cell::RefCell;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV6};

    #[test]
    fn the_rust_trap_that_costs_a_day() {
        // `R8` §8, and it is one line that saves an afternoon: `SocketAddrV6`
        // accepts the scope-id **only as a decimal integer**, so the form a
        // person would write does not parse.
        assert!(
            "[fe80::1%eth0]:9000".parse::<SocketAddrV6>().is_err(),
            "if this ever starts parsing, the workaround below is dead code and \
             somebody should find out why"
        );

        // And the shape Qyro uses -- resolve the name to an index first, then
        // put the integer in -- does work.
        let numeric: SocketAddrV6 = "[fe80::1%3]:9000"
            .parse()
            .expect("a numeric scope-id is the form std accepts");
        assert_eq!(numeric.scope_id(), 3);
    }

    #[test]
    fn apipa_is_reachable_and_loopback_is_not() {
        // The one that matters: 169.254/16 is what a direct cable produces, so
        // a filter that dropped it would reject the only case this module is
        // for. The first draft of a "usable address" check is where that
        // mistake lives.
        assert!(is_reachable_by_a_peer(IpAddr::V4(Ipv4Addr::new(
            169, 254, 3, 7
        ))));
        assert!(is_apipa(Ipv4Addr::new(169, 254, 3, 7)));
        assert!(!is_reachable_by_a_peer(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(!is_reachable_by_a_peer(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    }

    #[test]
    fn an_ipv6_link_local_address_is_not_something_to_put_in_a_code() {
        // The zone-id is local to the node (RFC 4007): `fe80::1%3` names a
        // different interface on the machine that types it. Usable inside,
        // never in a pairing code.
        let link_local: Ipv6Addr = "fe80::1".parse().expect("a literal");
        assert!(is_link_local_v6(link_local));
        assert!(!is_reachable_by_a_peer(IpAddr::V6(link_local)));

        // The control: a global v6 address is not rejected by the same rule.
        let global: Ipv6Addr = "2001:db8::1".parse().expect("a literal");
        assert!(!is_link_local_v6(global));
        assert!(is_reachable_by_a_peer(IpAddr::V6(global)));
    }

    #[test]
    fn the_wait_counts_out_loud_and_ends_with_advice_not_an_error() {
        // The sixty-second path, in microseconds. A test that actually waited a
        // minute is a test nobody runs, and a path nobody runs is a path that
        // breaks.
        let clock = RefCell::new(Duration::ZERO);
        let seen: RefCell<Vec<LinkState>> = RefCell::new(Vec::new());

        let outcome = wait_for_link(
            Vec::new,
            || {
                let mut now = clock.borrow_mut();
                *now += Duration::from_secs(10);
                *now
            },
            |state| seen.borrow_mut().push(state.clone()),
            APIPA_BUDGET,
            Duration::ZERO,
        );

        assert_eq!(
            outcome,
            LinkState::StillNothing,
            "sixty seconds with no address must end in advice"
        );
        let seen = seen.borrow();
        assert!(
            seen.iter().any(|s| matches!(s, LinkState::Waiting { .. })),
            "the wait reported nothing while it waited, which is what teaches a \
             person the cable is broken: {seen:?}"
        );
        assert!(
            seen.len() >= 5,
            "the countdown ticked {} times in sixty seconds",
            seen.len()
        );
    }

    #[test]
    fn and_it_stops_the_moment_an_address_appears() {
        // The control for the test above. A waiter that always ran to the
        // budget would satisfy it perfectly and be useless.
        let seen: RefCell<Vec<LinkState>> = RefCell::new(Vec::new());
        let outcome = wait_for_link(
            || vec![IpAddr::V4(Ipv4Addr::new(169, 254, 9, 9))],
            || Duration::ZERO,
            |state| seen.borrow_mut().push(state.clone()),
            APIPA_BUDGET,
            Duration::ZERO,
        );

        assert_eq!(
            outcome,
            LinkState::Ready(IpAddr::V4(Ipv4Addr::new(169, 254, 9, 9)))
        );
        assert_eq!(
            seen.borrow().len(),
            1,
            "it kept waiting after the address arrived"
        );
    }
}
