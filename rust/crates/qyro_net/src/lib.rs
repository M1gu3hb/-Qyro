//! The Qyro network transport: QYRO/1 frames over a blocking TCP socket.
//!
//! Specification: `docs/adr/ADR-0028-network-transport.md`.
//!
//! # What this does
//!
//! It carries frames between two processes over `std::net`. A [`Listener`]
//! accepts under a budget, [`dial`] connects with a deadline, and a
//! [`FrameStream`] turns a byte stream into whole frames and back.
//!
//! # What it does not do
//!
//! - **It does not reassemble frames.** `qyro_protocol::FrameDecoder` does, and
//!   re-deciding its ceiling or its poisoning rules here would mean two answers
//!   to one question. This crate feeds it and respects what it says.
//! - **It invents no cryptography and cannot fabricate a sealed frame.**
//!   [`FrameStream::write_frame`] takes bytes that are already encoded, so the
//!   only thing that can produce something claiming to be sealed is
//!   `qyro_crypto`.
//! - **It does not discover peers.** The address is the caller's to supply.
//!   There is no mDNS, no broadcast and no default port.
//! - **It does not reconnect.** A connection that ends, ends.
//! - **It is not async.** One connection, two blocking threads. ADR-0028 §6.
//!
//! # The one rule worth carrying out of here
//!
//! Endings are typed, and only one kind poisons: **what lied, not what
//! stopped**. A close, a reset and a silence are three different facts and none
//! of them is an attack; a tag that fails to verify is. [`NetError`] keeps them
//! apart, and no variant is called `Io`.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod beacon;
mod discovery;
mod error;
mod handshake;
mod limits;
mod listener;
mod pairing;
mod stream;

#[cfg(test)]
mod guards;
#[cfg(test)]
mod tests;

// **El beacon NO lleva `cfg`, y es lo contrario de un descuido.** Es la
// implementación en el árbol que existe precisamente para las plataformas sin
// responder de mDNS (ADR-0043 §5): sólo usa `std`, `socket2` e `if-addrs`, los
// tres multiplataforma.
//
// Aquí había un `#[cfg(windows)]` que pertenecía a la línea de abajo y que
// `dab9fa3` dejó pegado a este bloque al insertarlo **entre el atributo y el
// elemento que guardaba**. Resultado: el beacon desapareció fuera de Windows y
// `MdnsDiscovery` se exportó en todas partes sin existir. La misma forma que el
// `#[cfg(test)]` separado de `mod tests` el mismo día.
pub use beacon::{
    BEACON_PORT, Beacon, BeaconSwarm, MAX_BEACON_PAYLOAD, MDNS_GROUP_V4, announcement_targets,
    beacon_interfaces,
};

/// Sólo Windows: `mdns-sd` está bajo `cfg(windows)` en `discovery.rs`.
#[cfg(windows)]
pub use discovery::MdnsDiscovery;
pub use discovery::{
    PeerDiscovery, PeerEndpoint, SERVICE_TYPE, TXT_FINGERPRINT_KEY, fingerprint_from_txt,
    fingerprint_to_txt,
};
pub use error::{NetError, SocketOp};
pub use handshake::{Session, initiate, initiate_within, respond, respond_within};
pub use limits::{
    CONNECT_TIMEOUT, DECISION_DEADLINE, HANDSHAKE_DEADLINE, IDLE_TIMEOUT, MAX_ESTABLISHED_SESSIONS,
    MAX_PENDING_HANDSHAKES, MAX_PREAUTH_BYTES, QYRO_PORT, READ_BUFFER_LEN, READ_TIMEOUT,
};
pub use listener::{Listener, PendingSlot, REFUSAL_TOO_MANY_PENDING, dial, refusal_of};
pub use pairing::{
    PAIRING_FINGERPRINT_LEN, PAIRING_PREFIX, PAIRING_SEPARATOR, PairingEndpoint, PairingError,
};
pub use stream::FrameStream;
