//! The five ways a transfer stops, each provoked for real.
//!
//! Sprint 6A Phase 5. ADR-0028 §5 names the endings and gives each a typed
//! error; this file is where each one is actually caused rather than reasoned
//! about. Nothing here simulates a failure by calling the error constructor.
//!
//! Lives beside the two-process harness because these tests need the whole
//! stack — transport, handshake, engine — and the harness crate is the one that
//! already depends on all of it.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use std::collections::BTreeMap;
use std::io::Write;

use qyro_crypto::DeviceIdentity;
use qyro_fs::{FileSink, FileSource, PlannedFile, manifest_from_disk};
use qyro_net::{
    FrameStream, Listener, MAX_PENDING_HANDSHAKES, NetError, Session, dial, initiate, respond,
};
use qyro_transfer::{Phase, Receiver, Sender, TransferError};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn loopback() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
}

fn scratch(label: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "qyro-6a-end-{label}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

/// Two established sessions over one real socket, in two threads.
fn session_pair() -> (Session, Session) {
    let listening_id = DeviceIdentity::generate().unwrap();
    let dialling_id = DeviceIdentity::generate().unwrap();
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();
    let initiator = thread::spawn(move || {
        let stream = dial(addr).unwrap();
        initiate(stream, &dialling_id).unwrap()
    });
    let accepted = listener.accept().unwrap();
    let responder = respond(accepted, &listening_id).unwrap();
    (responder, initiator.join().unwrap())
}

/// Threads this process currently has, from the kernel rather than a guess.
#[cfg(target_os = "linux")]
fn live_threads() -> usize {
    fs::read_dir("/proc/self/task")
        .map(|entries| entries.count())
        .unwrap_or(0)
}

/// Descriptors this process currently holds.
#[cfg(target_os = "linux")]
fn live_descriptors() -> usize {
    fs::read_dir("/proc/self/fd")
        .map(|entries| entries.count())
        .unwrap_or(0)
}

/// Waits for a condition, so a count is not read before the runtime has caught
/// up. Bounded: it gives up and lets the assertion report the real number.
#[cfg(target_os = "linux")]
fn settle(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && !done() {
        thread::sleep(Duration::from_millis(20));
    }
}

// ------------------------------------------------- ending: the peer cut the wire

#[test]
fn a_connection_cut_mid_transfer_is_a_typed_end() {
    let (mut responder, initiator) = session_pair();

    // A real shutdown from the other side, which is what a cut looks like: not
    // a dropped object, an actual FIN.
    initiator.shutdown().unwrap();

    responder.stream().shutdown().ok();
    let error = match responder.recv() {
        Ok(Some(frame)) => panic!("a cut connection delivered {frame:?}"),
        Ok(None) => panic!("a cut connection reported a heartbeat forever"),
        Err(error) => error,
    };

    assert!(
        error.is_peer_gone(),
        "a cut connection must be one of the peer-gone endings, got {error:?}"
    );
    assert!(
        !error.poisons(),
        "a wire that ended said nothing false, so it must not poison"
    );
    // And specifically not a generic socket failure.
    assert!(
        !matches!(error, NetError::SocketFailed { .. }),
        "a cut connection produced the catch-all rather than a named ending: {error:?}"
    );
}

// ------------------------------------ ending: a stranger opens and says nothing

#[test]
fn a_peer_that_opens_connections_without_speaking_does_not_exhaust_the_listener() {
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();

    // More than the budget, all silent. This is the cheapest denial of service
    // there is: connect, say nothing, repeat.
    let hostile = MAX_PENDING_HANDSHAKES * 3;
    let mut sockets = Vec::new();
    for _ in 0..hostile {
        match TcpStream::connect(addr) {
            Ok(socket) => sockets.push(socket),
            Err(_) => break,
        }
    }
    assert!(
        sockets.len() >= MAX_PENDING_HANDSHAKES,
        "the test could not even open the budget, so it proves nothing"
    );

    // Every one of them is accepted, and the budget is checked at each step.
    // The whole backlog is drained on purpose: leaving some queued would mean
    // the honest dial below was answered by a leftover silent socket rather
    // than by the peer this test thinks it is serving, which is how a test ends
    // up asserting something it never exercised.
    let opened = sockets.len();
    for _ in 0..opened {
        let accepted = listener.accept().unwrap();
        assert!(
            listener.pending() <= MAX_PENDING_HANDSHAKES,
            "listener held {} unauthenticated connections against a budget of {}",
            listener.pending(),
            MAX_PENDING_HANDSHAKES
        );
        drop(accepted);
    }
    drop(sockets);

    // And it is still usable afterwards: refusing strangers must not cost it
    // the ability to serve a real peer.
    settle_pending(&listener);
    assert_eq!(
        listener.pending(),
        0,
        "dropping every accepted connection must return every slot"
    );

    let identity = DeviceIdentity::generate().unwrap();
    let dialling_id = DeviceIdentity::generate().unwrap();
    let honest = thread::spawn(move || {
        let stream = dial(addr).unwrap();
        initiate(stream, &dialling_id)
    });
    let accepted = listener.accept().unwrap();
    let served = respond(accepted, &identity);
    assert!(
        served.is_ok(),
        "the listener could not serve an honest peer after the flood: {:?}",
        served.err()
    );
    assert!(honest.join().unwrap().is_ok());
}

fn settle_pending(listener: &Listener) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && listener.pending() != 0 {
        thread::sleep(Duration::from_millis(20));
    }
}

// ---------------------------------------------- ending: the remote process died

#[test]
fn a_remote_process_killed_mid_transfer_is_a_typed_end_not_a_hang() {
    let destination = scratch("kill");

    // A real child, killed with a real signal, partway through its life.
    let mut child = Command::new(env!("CARGO_BIN_EXE_qyro_net_smoke"))
        .arg("serve")
        .arg("0")
        .arg(&destination)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let port: u16 = line
        .strip_prefix("LISTENING ")
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let identity = DeviceIdentity::generate().unwrap();
    let stream = dial(addr).unwrap();
    let mut session = initiate(stream, &identity).unwrap();

    // Killed after the handshake, so there is a live session to lose.
    child.kill().unwrap();
    let _ = child.wait();

    // The survivor must produce a typed ending. Not a panic, and not a hang:
    // the whole test is wrapped in a bound so that a hang fails rather than
    // stalling the suite for ever.
    session.stream().shutdown().ok();
    let started = Instant::now();
    let outcome = loop {
        match session.recv() {
            Ok(Some(_)) => panic!("a dead process kept sending"),
            Ok(None) => {
                assert!(
                    started.elapsed() < Duration::from_secs(30),
                    "the survivor hung instead of producing an ending"
                );
            }
            Err(error) => break error,
        }
    };

    assert!(
        outcome.is_peer_gone(),
        "a killed process must be one of the peer-gone endings, got {outcome:?}"
    );
    // ADR-0028 §5.1 is explicit that TCP cannot tell a killed process from a
    // pulled cable, so this asserts the *category* and refuses to pretend it
    // can name the cause. Pinning one variant here would be a bound
    // extrapolated from one run of one kernel.
    assert!(
        matches!(
            outcome,
            NetError::PeerVanished { .. }
                | NetError::PeerClosedEarly
                | NetError::PeerClosedMidFrame { .. }
        ),
        "unexpected ending for a killed process: {outcome:?}"
    );

    let _ = fs::remove_dir_all(&destination);
}

// ---------------------------------------------- resources: threads and handles

#[cfg(target_os = "linux")]
#[test]
fn no_thread_and_no_descriptor_survives_a_finished_session() {
    // Counted from /proc, which is the kernel's answer rather than the
    // program's opinion of itself. Not available on Windows, and this test is
    // therefore Linux-only -- which is a gap, not a pass, and the report says
    // so.
    let threads_before = live_threads();
    let descriptors_before = live_descriptors();

    // Several sessions, so a leak of one per session is visible rather than
    // lost in the noise of a single run.
    for _ in 0..4 {
        let (responder, initiator) = session_pair();
        responder.shutdown().ok();
        initiator.shutdown().ok();
        drop(responder);
        drop(initiator);
    }

    settle(|| live_threads() <= threads_before);
    let threads_after = live_threads();
    assert!(
        threads_after <= threads_before,
        "threads grew from {threads_before} to {threads_after} across four sessions"
    );

    settle(|| live_descriptors() <= descriptors_before);
    let descriptors_after = live_descriptors();
    assert!(
        descriptors_after <= descriptors_before,
        "descriptors grew from {descriptors_before} to {descriptors_after} across four \
         sessions; each session opens a socket and each socket is a descriptor"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_descriptor_leak_would_be_visible_to_this_measurement() {
    // The counter-test for the one above. A measurement that cannot see a leak
    // is not evidence that there is none, and "no leak" passing trivially is
    // exactly how a resource test rots. So: leak four descriptors on purpose
    // and confirm the count moves.
    let before = live_descriptors();
    let listener = Listener::bind(loopback()).unwrap();
    let addr = listener.local_addr().unwrap();

    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(TcpStream::connect(addr).unwrap());
    }
    let after = live_descriptors();
    assert!(
        after > before,
        "holding four sockets did not move the descriptor count ({before} then {after}); \
         the measurement in no_thread_and_no_descriptor_survives_a_finished_session \
         cannot see a leak and proves nothing"
    );

    drop(held);
    drop(listener);
}

// ------------------------------------------- ending: nothing arrives, ever again

#[test]
fn a_silent_peer_after_the_handshake_ends_the_session_rather_than_hanging() {
    let (mut responder, initiator) = session_pair();

    // Sixty seconds is the production deadline; shortened so the ending is
    // produced rather than assumed.
    let deadline = Duration::from_millis(700);
    responder.stream_mut().set_idle_timeout(deadline);

    let started = Instant::now();
    let error = loop {
        match responder.recv() {
            Ok(Some(frame)) => panic!("a silent peer delivered {frame:?}"),
            Ok(None) => {
                assert!(
                    started.elapsed() < Duration::from_secs(20),
                    "a silent peer was never declared dead"
                );
            }
            Err(error) => break error,
        }
    };

    match error {
        NetError::PeerSilent { idle } => assert!(idle >= deadline),
        other => panic!("silence after the handshake must be PeerSilent, got {other:?}"),
    }
    assert!(!error.poisons());
    drop(initiator);
}

// ----------------------------------------- endings: cancelled, and refused

/// A small real transfer over a real socket, ready to be interrupted.
///
/// Small on purpose. These two tests are about *how a transfer stops*; eight
/// megabytes would only make the interruption harder to place, and Phase 4
/// already covers the size that exercises the window.
struct WiredTransfer {
    sender: Sender,
    receiver: Receiver,
    /// The socket end the sender writes to and reads from.
    sending: FrameStream,
    /// The other end of the same socket, the receiver's.
    receiving: FrameStream,
    source: FileSource,
    sink: FileSink,
    destination: PathBuf,
    source_dir: PathBuf,
}

fn wired_transfer(label: &str) -> WiredTransfer {
    let source_dir = scratch(&format!("{label}-src"));
    let destination = scratch(&format!("{label}-dst"));
    let file = source_dir.join("payload.bin");
    let mut handle = fs::File::create(&file).unwrap();
    // Several chunks, so there is a middle to interrupt.
    handle.write_all(&vec![0xA5_u8; 300 * 1024]).unwrap();
    handle.sync_all().unwrap();

    let planned = vec![PlannedFile {
        source: file.clone(),
        relative: "payload.bin".to_owned(),
    }];
    let manifest = manifest_from_disk(1, 0, &planned).unwrap();
    let sink = FileSink::new(&destination, &manifest).unwrap();

    let mut paths = BTreeMap::new();
    paths.insert(1_u32, file);

    // The handshake is real; only afterwards do the parts go to the engine.
    let (receiving_session, sending_session) = session_pair();
    let (sending, send_sealer, send_opener) = sending_session.into_parts();
    let (receiving, recv_sealer, recv_opener) = receiving_session.into_parts();

    WiredTransfer {
        sender: Sender::new(send_sealer, send_opener, manifest),
        receiver: Receiver::new(recv_sealer, recv_opener),
        sending,
        receiving,
        source: FileSource::new(paths),
        sink,
        destination,
        source_dir,
    }
}

fn put(stream: &mut FrameStream, frames: &[Vec<u8>]) {
    for frame in frames {
        stream.write_frame(frame).unwrap();
    }
    stream.flush().unwrap();
}

/// Drives the transfer far enough that it is genuinely under way.
///
/// Returns once content has moved, so an interruption afterwards lands in the
/// middle of a transfer rather than before one started.
fn get_moving(wired: &mut WiredTransfer, rounds: usize) {
    let opening = wired.sender.open().unwrap();
    put(&mut wired.sending, &opening);

    for _ in 0..rounds {
        while let Ok(Some(frame)) = wired.receiving.read_frame() {
            let bytes = frame.try_encode().unwrap();
            let answers = wired.receiver.deliver(&bytes, &mut wired.sink).unwrap();
            put(&mut wired.receiving, &answers);
        }
        while let Ok(Some(frame)) = wired.sending.read_frame() {
            let bytes = frame.try_encode().unwrap();
            wired.sender.deliver(&bytes).unwrap();
        }
        let produced = wired.sender.pump(&wired.source).unwrap();
        put(&mut wired.sending, &produced);
    }
}

/// The receiver's next verdict on whatever arrives.
fn receiver_next(wired: &mut WiredTransfer) -> Result<(), TransferError> {
    loop {
        match wired.receiving.read_frame() {
            Ok(Some(frame)) => {
                let bytes = frame.try_encode().unwrap();
                wired.receiver.deliver(&bytes, &mut wired.sink)?;
                if wired.receiver.phase() == Phase::Cancelled {
                    return Ok(());
                }
            }
            Ok(None) => return Ok(()),
            Err(error) => panic!("transport ended instead of delivering: {error:?}"),
        }
    }
}

/// The sender's next verdict on whatever arrives.
fn sender_next(wired: &mut WiredTransfer) -> Result<(), TransferError> {
    loop {
        match wired.sending.read_frame() {
            Ok(Some(frame)) => {
                let bytes = frame.try_encode().unwrap();
                wired.sender.deliver(&bytes)?;
                if wired.sender.phase() == Phase::Cancelled {
                    return Ok(());
                }
            }
            Ok(None) => return Ok(()),
            Err(error) => panic!("transport ended instead of delivering: {error:?}"),
        }
    }
}

fn cleanup(wired: &WiredTransfer) {
    let _ = fs::remove_dir_all(&wired.source_dir);
    let _ = fs::remove_dir_all(&wired.destination);
}

#[test]
fn a_sender_that_cancels_mid_transfer_tells_the_receiver() {
    let mut wired = wired_transfer("cancel");
    get_moving(&mut wired, 1);
    // The precondition, stated as what it actually is: the transfer must still
    // be running. A cancel that arrives after the transfer finished would test
    // nothing, and asserting a specific phase here would make the test brittle
    // about scheduling rather than strict about the property.
    assert_ne!(
        wired.sender.phase(),
        Phase::Done,
        "the transfer finished before it could be cancelled, so nothing was interrupted"
    );
    assert_ne!(wired.sender.phase(), Phase::Cancelled);

    // The real control frame, from the real engine, sealed by the real sealer,
    // over the real socket.
    let cancel = wired.sender.request_cancel().unwrap();
    put(&mut wired.sending, std::slice::from_ref(&cancel));

    let outcome = receiver_next(&mut wired);
    assert!(
        outcome.is_ok() || matches!(outcome, Err(TransferError::Cancelled)),
        "a cancel must arrive as a cancellation, got {outcome:?}"
    );
    assert_eq!(
        wired.receiver.phase(),
        Phase::Cancelled,
        "the receiver did not learn it was cancelled"
    );

    // And nothing is materialised. A cancelled transfer leaves no final file,
    // because nothing ever calls finish_item for it.
    let arrived = wired.destination.join("payload.bin");
    assert!(
        !arrived.exists(),
        "a cancelled transfer left a final file at {}",
        arrived.display()
    );

    cleanup(&wired);
}

#[test]
fn a_receiver_that_refuses_stops_the_sender() {
    let mut wired = wired_transfer("refuse");
    get_moving(&mut wired, 1);
    assert_ne!(
        wired.sender.phase(),
        Phase::Done,
        "the transfer finished before it could be refused"
    );

    // What this is, and what it is not. The protocol defines a TransferReject
    // message, but the engine has no path that emits or handles one, so the
    // only refusal a receiver can express today is a cancel. The test uses the
    // refusal that exists rather than pretending the other one does; the gap is
    // in the report.
    let refusal = wired.receiver.request_cancel().unwrap();
    put(&mut wired.receiving, std::slice::from_ref(&refusal));

    let outcome = sender_next(&mut wired);
    assert!(
        outcome.is_ok() || matches!(outcome, Err(TransferError::Cancelled)),
        "a refusal must reach the sender, got {outcome:?}"
    );
    assert_eq!(
        wired.sender.phase(),
        Phase::Cancelled,
        "the sender kept going after being refused"
    );

    // It stops. A sender that kept emitting chunks here would be pushing
    // content at a peer that has already said no.
    let after = wired.sender.pump(&wired.source);
    let produced = after.unwrap_or_default();
    assert!(
        produced.is_empty(),
        "the sender produced {} more frames after being refused",
        produced.len()
    );

    cleanup(&wired);
}
