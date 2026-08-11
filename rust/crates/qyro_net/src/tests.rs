//! Phase 2 of sprint 6A: real sockets on 127.0.0.1, no doubles.
//!
//! Every test here binds a real listener, dials a real connection and moves
//! real bytes. Nothing is simulated, because the whole problem this crate
//! exists to solve — that a read is not a message — only exists on a socket. A
//! double that hands over neat frame-sized slices would test the double.
//!
//! Two habits, both from traps this project has actually fallen into:
//!
//! - Where a test claims something was *measured*, the number comes from a
//!   counter that the operation incremented, and it is compared against a value
//!   the test computed some other way. A counter compared against the constant
//!   it was set from proves nothing.
//! - Where a test's name states a property, the body provokes that property.
//!   "Split across three reads" therefore asserts that three reads happened,
//!   rather than assuming it because the test wrote three times.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use qyro_protocol::{Frame, HEADER_LEN, MAX_PAYLOAD_LEN, MessageType};

use crate::error::NetError;
use crate::limits::MAX_PREAUTH_BYTES;
use crate::listener::{Listener, dial};
use crate::stream::FrameStream;

// ------------------------------------------------------------------ harness

/// Port 0: the system picks. A fixed port makes a test suite intermittent the
/// moment two of them run at once, and the reflex fix for that is a `sleep`.
fn loopback() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
}

/// One `FrameStream` under test, and a raw socket to misbehave from.
///
/// The far end stays raw on purpose: half these tests need to write a partial
/// frame, three frames at once, or nothing at all, and a `FrameStream` is
/// exactly the thing that would not let them.
fn stream_and_raw_peer() -> (FrameStream, TcpStream) {
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();
    let dialler = thread::spawn(move || {
        let socket = TcpStream::connect(addr).unwrap();
        // Matches what `FrameStream::new` does on the other side, so a write in
        // this test leaves immediately instead of waiting for Nagle. Without
        // it, "three writes" and "three reads" stop corresponding.
        socket.set_nodelay(true).unwrap();
        socket
    });
    let accepted = listener.accept().unwrap();
    let raw = dialler.join().unwrap();
    (accepted, raw)
}

/// Two real `FrameStream`s, as production would have them.
fn framed_pair() -> (FrameStream, FrameStream) {
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();
    let dialler = thread::spawn(move || dial(addr).unwrap());
    let accepted = listener.accept().unwrap();
    let dialled = dialler.join().unwrap();
    (accepted, dialled)
}

/// A frame whose payload is `len` bytes with a recognisable pattern.
///
/// The pattern is a function of the index, so a payload that comes back
/// shifted, truncated or interleaved with another frame's is visible rather
/// than merely "different length".
fn frame_of(message: MessageType, len: usize, tag: u8) -> Vec<u8> {
    let payload: Vec<u8> = (0..len)
        .map(|index| (index as u8).wrapping_mul(31).wrapping_add(tag))
        .collect();
    Frame::new(message, payload).unwrap().encode()
}

/// The payload `frame_of` would have produced, computed independently.
fn payload_of(len: usize, tag: u8) -> Vec<u8> {
    (0..len)
        .map(|index| (index as u8).wrapping_mul(31).wrapping_add(tag))
        .collect()
}

// -------------------------------------------------------------------- tests

#[test]
fn a_frame_split_across_three_reads_is_reassembled() {
    let (mut stream, mut raw) = stream_and_raw_peer();
    let encoded = frame_of(MessageType::Manifest, 600, 7);
    assert_eq!(encoded.len(), HEADER_LEN + 600);

    // Three writes with real gaps. The gap has to be comfortably longer than
    // the reader takes to come back around, or the kernel coalesces them and
    // the test silently stops testing a split.
    let cuts = [140_usize, 300, encoded.len()];
    let writer = thread::spawn(move || {
        let mut sent = 0;
        for cut in cuts {
            raw.write_all(&encoded[sent..cut]).unwrap();
            raw.flush().unwrap();
            sent = cut;
            thread::sleep(Duration::from_millis(80));
        }
        raw
    });

    let frame = stream.next_frame().unwrap();
    let _raw = writer.join().unwrap();

    // The frame is whole, and its payload is the one the test built — not the
    // one the stream happened to hand back.
    assert_eq!(frame.message_type(), Some(MessageType::Manifest));
    assert_eq!(frame.plaintext(), Some(payload_of(600, 7).as_slice()));

    // And it genuinely arrived in pieces. Measured: `read_calls` counts reads
    // that returned bytes.
    assert!(
        stream.read_calls() >= 3,
        "the frame was meant to arrive across three reads, but {} read(s) delivered it; \
         the writes were coalesced and this test proved nothing about reassembly",
        stream.read_calls()
    );
    assert_eq!(stream.bytes_read(), HEADER_LEN + 600);
}

#[test]
fn three_frames_in_one_read_are_all_delivered() {
    let (mut stream, mut raw) = stream_and_raw_peer();

    let mut batch = Vec::new();
    batch.extend_from_slice(&frame_of(MessageType::Hello, 12, 1));
    batch.extend_from_slice(&frame_of(MessageType::Capabilities, 40, 2));
    batch.extend_from_slice(&frame_of(MessageType::Manifest, 64, 3));
    let total = batch.len();

    let writer = thread::spawn(move || {
        raw.write_all(&batch).unwrap();
        raw.flush().unwrap();
        raw
    });

    let first = stream.next_frame().unwrap();
    let second = stream.next_frame().unwrap();
    let third = stream.next_frame().unwrap();
    let _raw = writer.join().unwrap();

    // Three distinct frames, in order, each with its own payload. Comparing
    // against independently computed payloads rather than against each other:
    // three frames that were all the same would otherwise pass.
    assert_eq!(first.message_type(), Some(MessageType::Hello));
    assert_eq!(first.plaintext(), Some(payload_of(12, 1).as_slice()));
    assert_eq!(second.message_type(), Some(MessageType::Capabilities));
    assert_eq!(second.plaintext(), Some(payload_of(40, 2).as_slice()));
    assert_eq!(third.message_type(), Some(MessageType::Manifest));
    assert_eq!(third.plaintext(), Some(payload_of(64, 3).as_slice()));

    // Fewer reads than frames is the property: the decoder was drained before
    // the socket was touched again. Asserting exactly one read would be a
    // stronger claim than loopback guarantees, and a test that is right about
    // the property and occasionally wrong about the kernel is worse than one
    // that is right about both.
    assert!(
        stream.read_calls() < 3,
        "three frames arrived in {} reads, so they were not batched and this test \
         proved nothing about draining",
        stream.read_calls()
    );
    assert_eq!(stream.bytes_read(), total);
}

#[test]
fn a_peer_that_sends_nothing_times_out_and_says_so() {
    let (mut stream, _raw) = stream_and_raw_peer();
    // Sixty seconds is the production number and is unreachable in a suite.
    // Shortened here so the ending is actually produced rather than assumed.
    let deadline = Duration::from_millis(600);
    stream.set_idle_timeout(deadline);

    let started = Instant::now();
    let mut heartbeats = 0_usize;
    let ending = loop {
        match stream.read_frame() {
            Ok(None) => heartbeats += 1,
            Ok(Some(_)) => panic!("the peer sent nothing, so nothing can have been decoded"),
            Err(error) => break error,
        }
    };
    let waited = started.elapsed();

    match ending {
        NetError::PeerSilent { idle_secs: _ } => {}
        other => panic!("silence must be PeerSilent, got {other:?}"),
    }
    // Not an `Io`, and not confused with a peer that closed.
    assert!(ending.is_peer_gone());
    assert!(
        !ending.poisons(),
        "silence is not a lie, so it must not poison"
    );

    // The wakeups happened and were *not* endings. This is the distinction the
    // whole design rests on: a read timeout is the heartbeat, not a death.
    assert!(
        heartbeats >= 1,
        "the read timeout must surface as Ok(None); with none, PeerSilent was \
         reached without ever waking up and cancellation could never be checked"
    );
    // Waited at least the deadline. Two independent clocks: one the test set,
    // one the test measured.
    assert!(
        waited >= deadline,
        "gave up after {waited:?}, before the {deadline:?} deadline"
    );
}

#[test]
fn a_peer_that_disconnects_mid_frame_is_a_typed_end() {
    let (mut stream, mut raw) = stream_and_raw_peer();

    // A header that honestly declares 100 payload bytes, followed by 10 of
    // them. The framing is valid; it is simply unfinished.
    let encoded = frame_of(MessageType::Manifest, 100, 5);
    let partial = HEADER_LEN + 10;
    raw.write_all(&encoded[..partial]).unwrap();
    raw.flush().unwrap();
    drop(raw);

    let ending = stream.next_frame().unwrap_err();

    match ending {
        NetError::PeerClosedMidFrame { buffered } => assert_eq!(
            buffered, partial,
            "the stranded bytes are the ones that were sent"
        ),
        other => panic!("a close mid-frame must be PeerClosedMidFrame, got {other:?}"),
    }
    assert!(ending.is_peer_gone());
    assert!(
        !ending.poisons(),
        "a peer that hung up said nothing false, so this must not poison"
    );
}

#[test]
fn a_peer_that_disconnects_on_a_boundary_is_a_different_typed_end() {
    let (mut stream, raw) = stream_and_raw_peer();
    // Nothing at all is sent, so nothing is half-decoded.
    drop(raw);

    let ending = stream.next_frame().unwrap_err();

    match ending {
        NetError::PeerClosedEarly => {}
        other => panic!("a clean close must be PeerClosedEarly, got {other:?}"),
    }
    // The point of having two variants: they must not collapse into one.
    assert_ne!(ending, NetError::PeerClosedMidFrame { buffered: 0 });
}

#[test]
fn a_peer_cannot_make_us_buffer_more_than_the_declared_limit() {
    let (mut stream, mut raw) = stream_and_raw_peer();
    assert!(!stream.is_authenticated(), "the limit is for strangers");

    // A *legitimate* header that declares a one-megabyte payload — built by
    // `Frame::new`, so no byte was patched and no header layout was assumed —
    // followed by a trickle of that payload and then a flood.
    let declared = MAX_PAYLOAD_LEN;
    let huge = frame_of(MessageType::DataChunk, declared, 9);
    let head = huge[..HEADER_LEN + 64].to_vec();

    let writer = thread::spawn(move || {
        if raw.write_all(&head).is_err() {
            return;
        }
        let _ = raw.flush();
        // Keep pushing. The reader must stop taking bytes at the allowance, so
        // these writes are expected to fail once it does; that failure is the
        // point and is deliberately ignored.
        let filler = vec![0_u8; 8192];
        for _ in 0..64 {
            if raw.write_all(&filler).is_err() {
                return;
            }
        }
    });

    let ending = stream.next_frame().unwrap_err();
    let _ = writer.join();

    match ending {
        NetError::PreAuthByteLimitExceeded { attempted, limit } => {
            assert_eq!(limit, MAX_PREAUTH_BYTES);
            assert!(
                attempted <= MAX_PREAUTH_BYTES,
                "reported {attempted} taken against an allowance of {MAX_PREAUTH_BYTES}"
            );
        }
        other => panic!("an unauthenticated flood must be PreAuthByteLimitExceeded, got {other:?}"),
    }

    // The measurement that matters, and the one this project got wrong before:
    // both numbers below come out of the operation, and both are compared
    // against values arrived at some other way.
    //
    // 1. The process never accepted more than the allowance. `bytes_read` is
    //    summed from what `read` returned, so deleting the clamp in
    //    `read_window` makes this fail rather than pass quietly.
    assert!(
        stream.bytes_read() <= MAX_PREAUTH_BYTES,
        "took {} bytes from a stranger against an allowance of {MAX_PREAUTH_BYTES}",
        stream.bytes_read()
    );
    // 2. The declared megabyte was never reserved. `peak_decoder_capacity` is
    //    read back out of the decoder after each push — it is what the `Vec`
    //    actually holds, not the limit this test hoped for.
    let reserved = stream.peak_decoder_capacity();
    assert!(
        reserved < declared,
        "a peer declaring {declared} bytes got {reserved} reserved for it"
    );
    assert!(
        reserved <= MAX_PREAUTH_BYTES,
        "reserved {reserved} for a peer whose whole allowance is {MAX_PREAUTH_BYTES}"
    );
    // And the staging buffer stayed small too: the 64 KiB one is not allocated
    // until a peer has earned it.
    assert_eq!(stream.read_buffer_len(), MAX_PREAUTH_BYTES);
}

#[test]
fn a_legitimate_frame_still_round_trips() {
    // The check that the refusals above do not work by refusing everything.
    let (mut listening, mut dialling) = framed_pair();

    let encoded = frame_of(MessageType::Capabilities, 128, 42);
    dialling.write_frame(&encoded).unwrap();
    dialling.flush().unwrap();

    let frame = listening.next_frame().unwrap();
    assert_eq!(frame.message_type(), Some(MessageType::Capabilities));
    assert_eq!(frame.plaintext(), Some(payload_of(128, 42).as_slice()));

    // And back the other way, so neither direction is accidentally special.
    let answer = frame_of(MessageType::Hello, 32, 43);
    listening.write_frame(&answer).unwrap();
    listening.flush().unwrap();

    let echoed = dialling.next_frame().unwrap();
    assert_eq!(echoed.message_type(), Some(MessageType::Hello));
    assert_eq!(echoed.plaintext(), Some(payload_of(32, 43).as_slice()));
}

#[test]
fn a_listener_reports_the_port_the_system_chose() {
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();
    // Port 0 is the request, never the answer. A test that dialled port 0 would
    // fail, which is the whole reason `local_addr` has to be asked.
    assert_ne!(addr.port(), 0);
    assert_eq!(listener.pending(), 0);
}

#[test]
fn authenticating_releases_the_listener_budget_and_grows_the_buffer() {
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();
    let dialler = thread::spawn(move || dial(addr).unwrap());
    let mut accepted = listener.accept().unwrap();
    let _dialled = dialler.join().unwrap();

    // While the peer is a stranger it costs the listener a slot and gets the
    // small buffer.
    assert_eq!(listener.pending(), 1);
    assert_eq!(accepted.read_buffer_len(), MAX_PREAUTH_BYTES);

    accepted.mark_authenticated();

    // Authenticating is what returns the slot; a busy receiver must not refuse
    // new peers because of connections that already proved who they are.
    assert_eq!(listener.pending(), 0);
    assert_eq!(accepted.read_buffer_len(), crate::limits::READ_BUFFER_LEN);
    assert!(accepted.is_authenticated());
}

#[test]
fn a_read_timeout_is_a_heartbeat_on_both_platforms() {
    // Found by the Phase 2 mutation sweep: deleting `TimedOut` from
    // `is_read_timeout` broke nothing, because Linux raises `WouldBlock` for an
    // expired `SO_RCVTIMEO` and Windows raises `TimedOut`. Nothing on Linux can
    // produce the Windows kind, so nothing on Linux was defending it.
    //
    // Be clear about what this closes and what it does not. It proves the
    // **mapping** treats both kinds as the heartbeat, so removing either arm now
    // fails a named test. It does **not** prove Windows behaves as described,
    // and it is not a substitute for running this crate there — which CI does
    // not do at all today. See the report, finding 6A-3.
    assert!(crate::stream::is_read_timeout(
        std::io::ErrorKind::WouldBlock
    ));
    assert!(crate::stream::is_read_timeout(std::io::ErrorKind::TimedOut));

    // And the kinds that must never be mistaken for one. A heartbeat that
    // swallowed a reset would turn a dead peer into an eternal wait.
    assert!(!crate::stream::is_read_timeout(
        std::io::ErrorKind::ConnectionReset
    ));
    assert!(!crate::stream::is_read_timeout(
        std::io::ErrorKind::UnexpectedEof
    ));

    // The other mapping, for the same reason: three kinds mean the socket is
    // gone, and only those three.
    assert!(crate::stream::is_peer_gone(
        std::io::ErrorKind::ConnectionReset
    ));
    assert!(crate::stream::is_peer_gone(
        std::io::ErrorKind::ConnectionAborted
    ));
    assert!(crate::stream::is_peer_gone(std::io::ErrorKind::BrokenPipe));
    assert!(!crate::stream::is_peer_gone(std::io::ErrorKind::WouldBlock));
}

#[test]
fn a_dial_to_a_closed_port_is_typed_and_is_not_a_generic_io_error() {
    // Bind, learn the port, then drop the listener so nothing is there.
    let addr = {
        let listener = Listener::bind(loopback()).unwrap();
        listener.local_addr().unwrap()
    };

    let error = dial(addr).unwrap_err();
    match error {
        NetError::PeerVanished { .. } | NetError::ConnectTimedOut { .. } => {}
        other => panic!("dialling a closed port must be typed, got {other:?}"),
    }
    assert!(!error.poisons());
}
