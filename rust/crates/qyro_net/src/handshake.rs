//! The four-message handshake of ADR-0021, carried over a real socket.
//!
//! **No handshake is invented here.** Every cryptographic step belongs to
//! `qyro_crypto`: this module moves four byte arrays across a `FrameStream` in
//! the right order, enforces the deadline, and hands back a [`Session`] once
//! `qyro_crypto` says the peer proved who it is. If something in `qyro_crypto`
//! had to change to make this fit, that would be a finding to record, not a
//! reason to write a second handshake.
//!
//! The four messages travel as the payload of plain `Hello` frames (ADR-0028
//! §1). Their own first three bytes are the version, the suite and the message
//! number, and `qyro_crypto` validates all three, so no new message type is
//! needed and neither `message.rs` nor `header.rs` is touched.
//!
//! # The order that matters
//!
//! The responder must put its `ResponderFinish` on the wire **before**
//! `confirm_sent()` turns it into a session. That is not a style preference:
//! `confirm_sent` consumes the pending state, and a responder that established
//! a session it had not yet told the initiator about would be a responder ready
//! to send sealed frames the initiator cannot open.

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
use std::time::Instant;

use qyro_crypto::aead::{AuthenticatedFrame, FrameOpener, FrameSealer};
use qyro_crypto::handshake::{InitiatorStart, ResponderStart};
use qyro_crypto::{DeviceIdentity, IdentityFingerprint};
use qyro_protocol::{DecodedFrame, Frame, MessageType, SessionId};

use crate::error::NetError;
use crate::limits::HANDSHAKE_DEADLINE;
use crate::listener::refusal_of;
use crate::stream::FrameStream;

/// An authenticated connection: sealed frames in, sealed frames out.
///
/// Holds the stream, so there is no way to keep using the socket around the
/// session's back — which is what stops anything from writing plaintext onto a
/// connection that is supposed to be sealed.
#[derive(Debug)]
pub struct Session {
    stream: FrameStream,
    sealer: FrameSealer,
    opener: FrameOpener,
    session_id: SessionId,
    peer_fingerprint: IdentityFingerprint,
    poisoned: bool,
}

impl Session {
    /// The session identifier both ends derived.
    ///
    /// Each end computes it from its own view of the transcript, so two ends
    /// agreeing on it is evidence. Comparing one end's value against *itself*
    /// would not be.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// The fingerprint of the identity the peer proved it holds.
    #[must_use]
    pub const fn peer_fingerprint(&self) -> &IdentityFingerprint {
        &self.peer_fingerprint
    }

    /// Whether a frame has failed to authenticate on this session.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// The underlying connection, for a caller that must shut it down.
    #[must_use]
    pub const fn stream(&self) -> &FrameStream {
        &self.stream
    }

    /// The underlying connection, mutably.
    ///
    /// For the settings a caller owns rather than the session does — the idle
    /// deadline, principally. It cannot be used to write around the sealer:
    /// `FrameStream::write_frame` takes bytes, and the only thing that produces
    /// bytes worth writing here is [`Self::seal`].
    pub const fn stream_mut(&mut self) -> &mut FrameStream {
        &mut self.stream
    }

    /// Seals a frame and returns the bytes, without writing them.
    ///
    /// Split from [`Self::send`] because the two halves belong to different
    /// threads: ADR-0028 §6 puts sealing on the reader thread, which owns the
    /// engine, and writing on the writer thread. Sealing must also stay in one
    /// place because the sequence number that goes into the nonce is the
    /// sealer's to assign — a caller that could choose it could repeat a nonce.
    ///
    /// # Errors
    ///
    /// [`NetError::Sealing`] if the frame cannot be sealed.
    pub fn seal(&mut self, frame: &Frame) -> Result<Vec<u8>, NetError> {
        let sealed = self.sealer.seal(frame).map_err(NetError::Sealing)?;
        Ok(sealed.encode())
    }

    /// Writes bytes that were already sealed by [`Self::seal`].
    ///
    /// # Errors
    ///
    /// Whatever the write reports: [`NetError::PeerVanished`] or
    /// [`NetError::SocketFailed`].
    pub fn write_sealed(&mut self, bytes: &[u8]) -> Result<(), NetError> {
        self.stream.write_frame(bytes)
    }

    /// Seals a frame and writes it.
    ///
    /// # Errors
    ///
    /// [`NetError::Sealing`], or whatever the write reports.
    pub fn send(&mut self, frame: &Frame) -> Result<(), NetError> {
        let bytes = self.seal(frame)?;
        self.write_sealed(&bytes)
    }

    /// Reads the next authenticated frame, or `Ok(None)` on a heartbeat.
    ///
    /// A plain frame arriving on an established session is refused: after the
    /// handshake, everything is sealed, and accepting an unsealed frame would
    /// mean accepting bytes nothing authenticated.
    ///
    /// # Errors
    ///
    /// [`NetError::NotAuthenticated`] when a tag does not verify, which also
    /// **poisons** this session permanently. Otherwise any ending the read can
    /// report.
    pub fn recv(&mut self) -> Result<Option<AuthenticatedFrame>, NetError> {
        if self.poisoned {
            return Err(NetError::NotAuthenticated);
        }
        let Some(frame) = self.stream.read_frame().inspect_err(|error| {
            if error.poisons() {
                self.poisoned = true;
            }
        })?
        else {
            return Ok(None);
        };

        let DecodedFrame::Encrypted(envelope) = frame else {
            // Not poisoned by the AEAD's rules — nothing failed to verify —
            // but a peer sending plaintext after the handshake is not speaking
            // this protocol, so the session stops here all the same.
            self.poisoned = true;
            return Err(NetError::NotAuthenticated);
        };

        match self.opener.open(&envelope) {
            Ok(authenticated) => Ok(Some(authenticated)),
            Err(_) => {
                self.poisoned = true;
                Err(NetError::NotAuthenticated)
            }
        }
    }

    /// Unblocks both directions of the underlying socket.
    ///
    /// # Errors
    ///
    /// [`NetError::SocketFailed`] with `SocketOp::Shutdown`.
    pub fn shutdown(&self) -> Result<(), NetError> {
        self.stream.shutdown()
    }

    /// Breaks the session into the stream and the two AEAD contexts.
    ///
    /// This is the seam the transfer engine needs: `qyro_transfer::Sender` and
    /// `Receiver` are constructed from a `FrameSealer` and a `FrameOpener`, and
    /// the handshake is the only thing that can produce a matched pair.
    ///
    /// It **consumes** the session, which is the point. While a `Session`
    /// exists, every byte leaving it has gone through the sealer; handing the
    /// parts out without consuming it would leave two owners of one sealer, and
    /// two sealers on one direction both start at sequence zero and reissue
    /// every nonce. After this call there is no `Session`, so there is no
    /// second path to the wire.
    #[must_use]
    pub fn into_parts(self) -> (FrameStream, FrameSealer, FrameOpener) {
        (self.stream, self.sealer, self.opener)
    }
}

/// Runs the initiator half over `stream`, under [`HANDSHAKE_DEADLINE`].
///
/// The dialling end is always the initiator (ADR-0028 §7).
///
/// # Errors
///
/// [`NetError::HandshakeDeadlineExceeded`], [`NetError::Handshake`],
/// [`NetError::TooManyPendingConnections`] if the listener turned us away, or
/// any ending the socket reports.
pub fn initiate(stream: FrameStream, identity: &DeviceIdentity) -> Result<Session, NetError> {
    initiate_within(stream, identity, HANDSHAKE_DEADLINE)
}

/// Runs the responder half over `stream`, under [`HANDSHAKE_DEADLINE`].
///
/// The listening end is always the responder (ADR-0028 §7).
///
/// # Errors
///
/// The same set as [`initiate`].
pub fn respond(stream: FrameStream, identity: &DeviceIdentity) -> Result<Session, NetError> {
    respond_within(stream, identity, HANDSHAKE_DEADLINE)
}

/// [`initiate`] with the deadline supplied.
///
/// Exposed so the deadline can be reached in a test without waiting ten
/// seconds. A deadline nothing ever reaches is a deadline nothing tests.
///
/// # Errors
///
/// The same set as [`initiate`].
pub fn initiate_within(
    mut stream: FrameStream,
    identity: &DeviceIdentity,
    within: Duration,
) -> Result<Session, NetError> {
    let deadline = Instant::now() + within;

    let (hello, awaiting) = InitiatorStart::new(identity)
        .send_hello()
        .map_err(NetError::Handshake)?;
    write_handshake(&mut stream, &hello)?;

    let responder_hello = read_handshake(&mut stream, deadline, within)?;
    let (finish, awaiting_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .map_err(NetError::Handshake)?;
    write_handshake(&mut stream, &finish)?;

    let responder_finish = read_handshake(&mut stream, deadline, within)?;
    let established = awaiting_finish
        .receive_responder_finish(&responder_finish)
        .map_err(NetError::Handshake)?;

    let session_id = established.session_id();
    let peer_fingerprint = *established.peer_fingerprint();
    let (sealer, opener) = established.into_frame_crypto().map_err(NetError::Sealing)?;

    stream.mark_authenticated();
    Ok(Session {
        stream,
        sealer,
        opener,
        session_id,
        peer_fingerprint,
        poisoned: false,
    })
}

/// [`respond`] with the deadline supplied.
///
/// # Errors
///
/// The same set as [`initiate`].
pub fn respond_within(
    mut stream: FrameStream,
    identity: &DeviceIdentity,
    within: Duration,
) -> Result<Session, NetError> {
    let deadline = Instant::now() + within;

    let initiator_hello = read_handshake(&mut stream, deadline, within)?;
    let (responder_hello, awaiting) = ResponderStart::new(identity)
        .receive_initiator_hello_from_system(&initiator_hello)
        .map_err(NetError::Handshake)?;
    write_handshake(&mut stream, &responder_hello)?;

    let initiator_finish = read_handshake(&mut stream, deadline, within)?;
    let pending = awaiting
        .receive_initiator_finish(&initiator_finish)
        .map_err(NetError::Handshake)?;

    // On the wire first, established second. See the module comment.
    let finish = *pending.encoded_finish();
    write_handshake(&mut stream, &finish)?;
    let established = pending.confirm_sent();

    let session_id = established.session_id();
    let peer_fingerprint = *established.peer_fingerprint();
    let (sealer, opener) = established.into_frame_crypto().map_err(NetError::Sealing)?;

    stream.mark_authenticated();
    Ok(Session {
        stream,
        sealer,
        opener,
        session_id,
        peer_fingerprint,
        poisoned: false,
    })
}

/// Puts one handshake message on the wire as a plain `Hello` frame.
fn write_handshake(stream: &mut FrameStream, message: &[u8]) -> Result<(), NetError> {
    let frame = Frame::new(MessageType::Hello, message.to_vec()).map_err(NetError::Framing)?;
    stream.write_frame(&frame.encode())?;
    stream.flush()
}

/// Waits for one handshake message, giving up at `deadline`.
///
/// The deadline is checked on every heartbeat, which is what makes it a bound
/// on the **whole** handshake rather than on one message: the caller computes
/// `deadline` once, before the first message, and passes the same instant to
/// every read. A peer that dribbles one message just before each expiry
/// therefore buys nothing.
fn read_handshake(
    stream: &mut FrameStream,
    deadline: Instant,
    within: Duration,
) -> Result<Vec<u8>, NetError> {
    loop {
        if Instant::now() >= deadline {
            return Err(NetError::HandshakeDeadlineExceeded { limit: within });
        }
        let Some(frame) = stream.read_frame()? else {
            continue;
        };

        // A full listener answers with a refusal rather than silence, so say so
        // instead of reporting a confusing "unexpected message".
        if let Some(refusal) = refusal_of(&frame) {
            return Err(refusal);
        }

        let Some(plain) = frame.as_plain() else {
            return Err(NetError::UnexpectedHandshakeMessage {
                got: frame.message_type(),
            });
        };
        if plain.message_type() != MessageType::Hello {
            return Err(NetError::UnexpectedHandshakeMessage {
                got: Some(plain.message_type()),
            });
        }
        return Ok(plain.payload().to_vec());
    }
}
