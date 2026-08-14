//! What `qyro_session` actually does, exercised rather than read.
//!
//! Phase 01 shipped this crate with six tests, all of them in `guards.rs`, and
//! all six read the production files as *text*: no panicking construct, every
//! file listed, every error variant constructed somewhere. Structural guards are
//! worth having and they caught a real defect — the `AlreadyFailed` variant that
//! nothing could produce — but not one of them opens a socket, and 444 lines of
//! `session.rs` had no test that made the code run (QYR-0309).
//!
//! So this file is the behaviour half. It drives a real sender and a real
//! receiver, in two threads, over a real loopback socket, through the crate's
//! **public** API and nothing else — which is also the surface ADR-0032 §2
//! bounds, so exercising it here is exercising the boundary itself.
//!
//! # Why a reserved port and not port 0
//!
//! The obvious shape is to bind the receiver on port 0 and ask it which port it
//! got. That is impossible today, and the impossibility is the finding: the
//! `Listener` that knows the answer is a local in `open_receiver`, dropped
//! before the constructor returns, and `Session::local_addr` hands back
//! `peer_addr` — the *far* end (QYR-0314). So the port is reserved the
//! conventional way: bind an ephemeral socket, read the number, drop it. The
//! sender then retries the dial, because the receiver may not have bound yet,
//! and a bounded retry is honest where a sleep is a guess.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use qyro_session::{Progress, Session, SessionError, SessionState};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A directory that removes itself, so a failed assertion cannot leave a file
/// that makes the next run pass.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "qyro-session-{tag}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Writes `len` deterministic bytes without holding them all in memory.
///
/// Deterministic on purpose: the comparison at the end has to be able to say
/// *which* byte differs, and a random file makes a failure unreproducible.
fn write_pattern(path: &Path, len: u64) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(path).unwrap();
    let mut written = 0_u64;
    let mut block = [0_u8; 4096];
    while written < len {
        let take = usize::try_from((len - written).min(block.len() as u64)).unwrap();
        for (index, slot) in block.iter_mut().enumerate().take(take) {
            // A byte that depends on its own offset: a transfer that shifted or
            // duplicated a chunk changes the value, not just the length.
            *slot = ((written + index as u64) % 251) as u8;
        }
        file.write_all(&block[..take]).unwrap();
        written += take as u64;
    }
    file.flush().unwrap();
}

fn read_all(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

/// A port nothing is listening on, released immediately.
///
/// The window between the drop and the receiver's bind is a race in principle.
/// It is the conventional technique and it is bounded; the alternative — a
/// hard-coded port — collides with whatever else the machine is running, which
/// is a worse race with a worse failure message.
fn a_free_port() -> u16 {
    let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = probe.local_addr().unwrap().port();
    assert_ne!(port, 0, "port 0 is the request, never the answer");
    port
}

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// Dials until the receiver has bound, or gives up with the real error.
///
/// Bounded rather than slept: a sleep long enough to be safe is long enough to
/// be slow, and a sleep short enough to be fast is a flake waiting for a loaded
/// machine.
fn open_sender_when_ready(
    address: SocketAddr,
    root: &Path,
    files: &[PathBuf],
) -> Result<Session, SessionError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match Session::open_sender(address, root, files, None) {
            Err(SessionError::PeerUnreachable) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            other => return other,
        }
    }
}

/// Steps a session to its ending, and reports the last progress seen.
///
/// Returns the terminal state and every distinct `done` value observed, so a
/// caller can assert on the shape of the progression and not only on its end.
fn drive(session: &mut Session) -> (Result<SessionState, SessionError>, Vec<Progress>) {
    let mut seen = vec![session.progress()];
    loop {
        match session.step() {
            Ok(SessionState::InProgress) => {
                let now = session.progress();
                if seen.last() != Some(&now) {
                    seen.push(now);
                }
            }
            other => {
                let now = session.progress();
                if seen.last() != Some(&now) {
                    seen.push(now);
                }
                return (other, seen);
            }
        }
    }
}

/// One file, one root, one transfer, both ends driven to their ending.
struct Moved {
    sent: Result<SessionState, SessionError>,
    received: Result<SessionState, SessionError>,
    materialised: Result<u32, SessionError>,
    sender_progress: Vec<Progress>,
    receiver_progress: Vec<Progress>,
}

fn move_files(root: &Path, files: &[PathBuf], destination: &Path) -> Moved {
    let address = loopback(a_free_port());
    let destination = destination.to_path_buf();

    let receiving = thread::spawn(move || {
        let mut session = match Session::open_receiver(address, &destination, None) {
            Ok(session) => session,
            Err(error) => return (Err(error), Err(error), Vec::new()),
        };
        let (state, progress) = drive(&mut session);
        let materialised = session.finish();
        (state, materialised, progress)
    });

    let mut sender = open_sender_when_ready(address, root, files);
    let (sent, sender_progress) = match sender.as_mut() {
        Ok(session) => drive(session),
        Err(error) => (Err(*error), Vec::new()),
    };

    let (received, materialised, receiver_progress) = receiving.join().unwrap();
    Moved {
        sent,
        received,
        materialised,
        sender_progress,
        receiver_progress,
    }
}

/// Two chunk windows and a bit.
///
/// The engine moves 64 KiB chunks behind a window of 16, so a file has to be
/// larger than 1 MiB before the window, the go-back-N and the flow control are
/// exercised at all. A file that fits in one window would pass a transfer that
/// never refills.
const CROSSES_THE_WINDOW: u64 = 2 * 1024 * 1024 + 7;

// ------------------------------------------------------- arguments, before the wire

#[test]
fn an_empty_file_list_is_refused_before_anything_is_dialled() {
    let root = Scratch::new("empty");

    // Nothing is listening on this address. A `BadArgument` therefore proves the
    // refusal happened before the dial: had the check been missing or moved
    // after it, this would be `PeerUnreachable` instead, and the two are
    // distinguishable exactly because nothing is listening.
    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[], None);

    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

#[test]
fn a_file_outside_the_root_is_refused_rather_than_renamed_to_its_last_component() {
    let root = Scratch::new("root");
    let elsewhere = Scratch::new("elsewhere");
    let stray = elsewhere.path("photo.jpg");
    write_pattern(&stray, 32);

    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[stray], None);

    // The quiet failure this guards against is naming the file `photo.jpg` and
    // sending it anyway: the receiver would get a name the sender never chose.
    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

#[test]
fn a_parent_directory_in_the_remainder_is_refused() {
    let root = Scratch::new("parent");
    let nested = root.path("inner");
    fs::create_dir_all(&nested).unwrap();
    write_pattern(&root.path("secret.bin"), 32);

    // `inner/../secret.bin` strips against the root successfully and still is
    // not a plain descendant. qyro_manifest refuses it too; refusing here keeps
    // the error the caller's.
    let escaping = nested.join("..").join("secret.bin");
    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[escaping], None);

    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

// ------------------------------------------------------------ the transfer itself

#[test]
fn a_file_crosses_two_sessions_on_the_loopback_and_arrives_byte_for_byte() {
    let source = Scratch::new("send");
    let destination = Scratch::new("recv");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );
    assert_eq!(moved.received, Ok(SessionState::Completed));
    assert_eq!(moved.materialised, Ok(1));

    let arrived = destination.path("payload.bin");
    assert!(arrived.exists(), "the receiver materialised nothing");
    let expected = read_all(&original);
    let actual = read_all(&arrived);
    assert_eq!(
        actual.len(),
        expected.len(),
        "the arrival is a different length from the original"
    );
    assert_eq!(
        actual, expected,
        "the arrival differs from the original byte for byte"
    );

    // And no partial survives its own success.
    assert!(
        !destination.path("payload.bin.qyro-part").exists(),
        "a .qyro-part outlived a completed transfer"
    );
}

#[test]
fn a_corrupted_arrival_would_be_visible_to_this_comparison() {
    // The counter-test for the one above (R2 §1.7). A byte-for-byte comparison
    // that cannot see a changed byte is not evidence that none changed, and a
    // comparison of a file with itself is exactly how this repository has
    // produced five green tests that checked nothing.
    let source = Scratch::new("corrupt-src");
    let destination = Scratch::new("corrupt-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );

    let arrived = destination.path("payload.bin");
    let mut tampered = read_all(&arrived);
    let midpoint = tampered.len() / 2;
    tampered[midpoint] ^= 0x01;

    let expected = read_all(&original);
    assert_eq!(
        tampered.len(),
        expected.len(),
        "flipping a bit must not change the length, or this test proves the wrong thing"
    );
    assert_ne!(
        tampered, expected,
        "a single flipped bit was invisible to the comparison the test above relies on"
    );
}

#[test]
fn two_files_under_a_common_root_arrive_under_their_own_relative_names() {
    // The doc-comment on `open_sender` claims this explicitly: naming by file
    // name alone would send `a.txt` twice and make the receiver arbitrate a
    // collision the sender created. Nothing exercised the claim.
    let source = Scratch::new("names-src");
    let destination = Scratch::new("names-dst");
    let first = source.path("docs/a.txt");
    let second = source.path("notes/a.txt");
    write_pattern(&first, 4096);
    write_pattern(&second, 8192);

    let moved = move_files(
        &source.dir,
        &[first.clone(), second.clone()],
        &destination.dir,
    );

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );
    assert_eq!(moved.materialised, Ok(2));

    let arrived_first = destination.path("docs/a.txt");
    let arrived_second = destination.path("notes/a.txt");
    assert!(arrived_first.exists(), "docs/a.txt did not arrive");
    assert!(arrived_second.exists(), "notes/a.txt did not arrive");

    // Two different sizes on purpose. Had the two names collapsed into one, the
    // survivor would carry one of the two bodies and the other would be absent
    // — a size check distinguishes that from a genuine pair, where an existence
    // check alone would not if both wrote the same bytes.
    assert_eq!(read_all(&arrived_first), read_all(&first));
    assert_eq!(read_all(&arrived_second), read_all(&second));
    assert_ne!(
        read_all(&arrived_first).len(),
        read_all(&arrived_second).len(),
        "the two arrivals are indistinguishable, so this test cannot see a collision"
    );
}

#[test]
fn progress_reaches_the_total_and_never_goes_backwards() {
    let source = Scratch::new("progress-src");
    let destination = Scratch::new("progress-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );

    let seen = moved.sender_progress;
    assert!(
        seen.len() > 2,
        "only {} progress samples for a {CROSSES_THE_WINDOW}-byte transfer; a counter \
         that reported its final value once would satisfy a weaker assertion",
        seen.len()
    );
    for pair in seen.windows(2) {
        assert!(
            pair[1].done >= pair[0].done,
            "progress went backwards: {} then {}",
            pair[0].done,
            pair[1].done
        );
    }
    let last = seen.last().unwrap();
    assert_eq!(
        last.total, CROSSES_THE_WINDOW,
        "the total the session reports is not the size of what was asked for"
    );
    assert_eq!(
        last.done, last.total,
        "the transfer completed without progress reaching the total"
    );
    // Trap 4 of the phase document: a transfer test that never checks there was
    // a transfer can be measuring the empty set.
    assert!(last.total > 0, "the total is zero, so nothing was moved");

    // The receiving end learns its total from the manifest rather than from the
    // caller, and that is worth defending on its own: it is the only number a
    // receiver has before any content arrives.
    let received = moved.receiver_progress;
    let receiver_total = received
        .iter()
        .map(|sample| sample.total)
        .max()
        .expect("the receiver was driven at least once");
    assert_eq!(
        receiver_total, CROSSES_THE_WINDOW,
        "the receiver never learned the size the manifest declares"
    );
    assert_eq!(
        received.first().map(|sample| sample.total),
        Some(0),
        "the receiver started out already knowing the total, so this test cannot \
         tell learning it from being handed it"
    );

    // Deliberately no assertion on the receiver's `done`: it is never assigned,
    // and `Progress::item` is never assigned by either role (QYR-0317,
    // QYR-0318). Asserting the current zero would freeze a defect into a
    // contract; asserting the intended behaviour would fail today. The findings
    // carry it instead.
}

// ------------------------------------------------------------- the progress bridge

/// Sends `size` bytes with an observer attached, and counts both.
///
/// Returns every emission and **how many times `step` was called**. The two
/// numbers together are what makes the budget measurable: a counter that only
/// knew emissions could not tell a budget from an implementation that emits
/// once and stops.
fn move_observed(size: u64) -> (Vec<Progress>, usize, Result<SessionState, SessionError>) {
    let source = Scratch::new("budget-src");
    let destination = Scratch::new("budget-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, size);

    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();
    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let (state, _) = drive(&mut session);
            let _ = session.finish();
            state
        })
    });

    let seen = Arc::new(Mutex::new(Vec::new()));
    // Rebuilt on every retry rather than probed with a throwaway connection: the
    // receiver accepts exactly once, so a probe that succeeded would consume the
    // session under test.
    let recorder_for = |shared: &Arc<Mutex<Vec<Progress>>>| -> Box<dyn FnMut(Progress) + Send> {
        let recorder = Arc::clone(shared);
        Box::new(move |progress| {
            if let Ok(mut log) = recorder.lock() {
                log.push(progress);
            }
        })
    };

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sender = loop {
        let attempt = Session::open_sender(
            address,
            &source.dir,
            &[original.clone()],
            Some(recorder_for(&seen)),
        );
        match attempt {
            Err(SessionError::PeerUnreachable) if Instant::now() < deadline => {
                // A refused dial emitted nothing, so the log is still empty and
                // the retry starts clean.
                thread::sleep(Duration::from_millis(5));
            }
            other => break other,
        }
    };
    let outcome = match sender.as_mut() {
        Ok(session) => {
            let mut steps = 0_usize;
            let state = loop {
                steps += 1;
                match session.step() {
                    Ok(SessionState::InProgress) => {}
                    other => break other,
                }
            };
            (state, steps)
        }
        Err(error) => (Err(*error), 0),
    };
    let _ = receiving.join();
    let emissions = seen.lock().map(|log| log.clone()).unwrap_or_default();
    (emissions, outcome.1, outcome.0)
}

#[test]
fn a_session_without_an_observer_still_completes() {
    // ADR-0033 §2: «no observer» must never be a second code path. Every other
    // test in this file passes None, so this one states the property by name
    // rather than leaving it implied.
    let source = Scratch::new("noobs-src");
    let destination = Scratch::new("noobs-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "a session with no observer did not complete; the receiver ended {:?}",
        moved.received
    );
    assert_eq!(
        read_all(&destination.path("payload.bin")),
        read_all(&original)
    );
}

#[test]
fn the_callback_budget_is_respected_for_a_known_file_size() {
    let (small, _, small_state) = move_observed(512 * 1024);
    let (large, _, large_state) = move_observed(4 * 1024 * 1024);
    assert_eq!(small_state, Ok(SessionState::Completed));
    assert_eq!(large_state, Ok(SessionState::Completed));

    // The upper bound of ADR-0033 §4: 100 stepped emissions, plus the opening
    // one, plus the terminal one.
    const CEILING: usize = 102;
    assert!(
        large.len() <= CEILING,
        "{} emissions for 4 MiB, over the budget of {CEILING}",
        large.len()
    );

    // Two sizes and a strict inequality, which is the shape that tells a
    // measured counter from a constant (R1 §5.6). Below the 25 MiB elbow the
    // 256 KiB floor decides, so eight times the bytes must emit strictly more.
    // An implementation that always emitted 102 -- or always 2 -- satisfies the
    // ceiling above and fails here.
    assert!(
        large.len() > small.len(),
        "512 KiB emitted {} and 4 MiB emitted {}; a budget that does not grow \
         with the file below the floor is not measuring the file",
        small.len(),
        large.len()
    );

    // And it is a real progression, not a repeated number.
    let last = large
        .last()
        .expect("a completed transfer emitted something");
    assert_eq!(last.done, last.total, "the last emission is not the total");
    assert_eq!(
        large.first().map(|first| first.done),
        Some(0),
        "the opening emission did not start at zero"
    );
    for pair in large.windows(2) {
        assert!(
            pair[1].done >= pair[0].done,
            "an emission went backwards: {} then {}",
            pair[0].done,
            pair[1].done
        );
    }
}

#[test]
fn an_emission_per_chunk_would_be_visible_to_this_measurement() {
    // R2 §1.7, and the reason the helper counts `step` calls as well as
    // emissions. The budget exists to stop the emission count tracking the
    // chunk count; the way to know this measurement could see that failure is
    // to compare it against a number that *does* track the chunks.
    //
    // `step` is the loop that moves chunks, so an implementation that emitted
    // once per step would make the two counts equal. They must not be.
    let (emissions, steps, state) = move_observed(4 * 1024 * 1024);
    assert_eq!(state, Ok(SessionState::Completed));

    assert!(
        steps > 0,
        "no step ran, so this measurement is comparing two zeros"
    );
    assert!(
        emissions.len() < steps,
        "{} emissions for {steps} steps. Equal counts is exactly what emitting \
         once per chunk looks like, so this measurement would report it",
        emissions.len()
    );
    // Not merely fewer -- decisively fewer. One emission short of per-step would
    // satisfy a bare `<` while being the failure this guards against.
    assert!(
        emissions.len() * 2 < steps,
        "{} emissions for {steps} steps is within a factor of two of one per \
         step, which is not a budget",
        emissions.len()
    );
}

// --------------------------------------------------------------- endings and state

#[test]
fn a_cancelled_session_reports_cancelled_and_keeps_reporting_it() {
    let source = Scratch::new("cancel");
    let original = source.path("payload.bin");
    write_pattern(&original, 4096);
    let address = loopback(a_free_port());
    let destination = Scratch::new("cancel-dst");
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let _ = drive(&mut session);
        })
    });

    let mut sender = open_sender_when_ready(address, &source.dir, &[original]).unwrap();
    assert!(
        !sender.is_cancelled(),
        "a fresh session is already cancelled"
    );

    sender.cancel();
    assert!(sender.is_cancelled(), "cancel did not raise the flag");

    // ADR-0032 §5 freezes stickiness as *returning the same code*. A second Ok
    // would let a caller believe the session recovered.
    assert_eq!(sender.step(), Err(SessionError::Cancelled));
    assert_eq!(sender.step(), Err(SessionError::Cancelled));
    assert_eq!(sender.step(), Err(SessionError::Cancelled));

    drop(sender);
    let _ = receiving.join();
}

#[test]
fn a_receiver_that_never_gets_a_peer_reports_the_peer_and_not_success() {
    let destination = Scratch::new("no-peer");
    let address = loopback(a_free_port());

    let receiving = thread::spawn(move || Session::open_receiver(address, &destination.dir, None));

    // Connect and hang up without speaking. The receiver must not treat a
    // socket that opened and closed as an authenticated peer.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(socket) = std::net::TcpStream::connect(address) {
            drop(socket);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let outcome = receiving.join().unwrap();
    assert!(
        outcome.is_err(),
        "a peer that said nothing produced an open session"
    );
    let error = outcome.err().unwrap();
    assert!(
        error == SessionError::PeerUnreachable || error == SessionError::NotAuthenticated,
        "a silent peer produced {error:?}, which is neither of the two endings that mean \
         the handshake did not happen"
    );
}

#[test]
fn finishing_a_sender_materialises_nothing_and_says_so() {
    // `finish` is the receiver's operation. A sender answering anything but zero
    // would mean the count came from somewhere other than materialised items.
    let source = Scratch::new("finish-src");
    let destination = Scratch::new("finish-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, 4096);
    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let (state, _) = drive(&mut session);
            let _ = session.finish();
            state
        })
    });

    let mut sender = open_sender_when_ready(address, &source.dir, &[original]).unwrap();
    let (sent, _) = drive(&mut sender);
    assert_eq!(sent, Ok(SessionState::Completed));

    assert_eq!(
        sender.finish(),
        Ok(0),
        "a sending session reported materialised items"
    );

    let _ = receiving.join();
}
