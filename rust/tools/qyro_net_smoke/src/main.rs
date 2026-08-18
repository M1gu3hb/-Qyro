//! Two real processes, one socket, one file.
//!
//! Sprint 6A Phase 4. Two threads in one process do not prove what two
//! processes prove: they share an allocator, global state and the test runner.
//! This binary exists so that the thing under test is an operating-system
//! process boundary.
//!
//! ```text
//! qyro_net_smoke serve <port> <destination-directory>
//! qyro_net_smoke send  <ip:port> <file>...
//! ```
//!
//! **A harness. Never shipped.** Nothing in the product depends on it; it
//! depends on the product. Same standing as `qyro_crypto_smoke` and
//! `qyro_store_smoke` under ADR-0023.
//!
//! # Two things it does deliberately
//!
//! `serve` prints `LISTENING <port>` and flushes **before** it accepts. A test
//! that waits for that line is synchronised on the thing that actually matters;
//! a test that sleeps instead is guessing, and the usual cure for guessing
//! wrong is a longer sleep.
//!
//! Both modes print one line of JSON at the end. The numbers in it are measured
//! during the run — bytes summed from the frames actually held — because the
//! memory claim of Phase 4 has to be checked across a process boundary, where a
//! `cfg(test)` counter does not exist.

#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing)]

use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;

#[cfg(test)]
mod guards;

use qyro_crypto::DeviceIdentity;
use qyro_fs::{FileSink, FileSource, PlannedFile, manifest_from_disk};
use qyro_net::{FrameStream, Listener, dial, initiate, respond};
use qyro_transfer::{ContentSource, ItemVerdict, Phase, Receiver, Sender};

/// Where a transfer's bytes go while it runs, and how many are held at once.
///
/// The number this reports is the sum of the lengths of the frames the process
/// is holding, taken as it holds them. It is not a constant and not a limit: if
/// the engine ever buffered a whole file, this would say so.
#[derive(Default)]
struct HeldBytes {
    current: usize,
    peak: usize,
}

impl HeldBytes {
    fn took(&mut self, frames: &[Vec<u8>]) {
        let added: usize = frames.iter().map(Vec::len).sum();
        self.current = self.current.saturating_add(added);
        if self.current > self.peak {
            self.peak = self.current;
        }
    }

    fn released(&mut self, bytes: usize) {
        self.current = self.current.saturating_sub(bytes);
    }
}

/// A source that flips one bit of the content, after the manifest was built.
///
/// This is how "a byte is corrupted in flight" is provoked between two
/// processes. The manifest already carries the digest of the honest file, so
/// what reaches the far end no longer hashes to what was promised — which is
/// the situation a corrupted wire produces, without needing a proxy in the
/// middle.
struct CorruptingSource {
    inner: FileSource,
    at: u64,
}

impl ContentSource for CorruptingSource {
    fn read_at(&self, item_id: u32, offset: u64, out: &mut [u8]) -> usize {
        let read = self.inner.read_at(item_id, offset, out);
        if self.at >= offset {
            let local = self.at.saturating_sub(offset);
            if let Ok(index) = usize::try_from(local)
                && index < read
                && let Some(byte) = out.get_mut(index)
            {
                *byte ^= 0x01;
            }
        }
        read
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = args.first().map(String::as_str);
    let result = match mode {
        Some("serve") => serve(args.get(1..).unwrap_or_default()),
        Some("send") => send(args.get(1..).unwrap_or_default()),
        _ => Err("usage: qyro_net_smoke serve <port> <dir> | send <ip:port> <file>...".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("qyro_net_smoke: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Receives: listens, responds to the handshake, writes files, verifies.
fn serve(args: &[String]) -> Result<(), String> {
    let port: u16 = args
        .first()
        .ok_or("serve needs a port")?
        .parse()
        .map_err(|_| "port must be a number".to_owned())?;
    let destination = PathBuf::from(args.get(1).ok_or("serve needs a destination directory")?);

    let listener = Listener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
        .map_err(|error| format!("bind: {error}"))?;
    let bound = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;

    // Before accept, and flushed. This is the synchronisation point.
    println!("LISTENING {}", bound.port());
    std::io::stdout().flush().map_err(|e| e.to_string())?;

    let identity = DeviceIdentity::generate().map_err(|error| format!("identity: {error}"))?;
    let accepted = listener
        .accept()
        .map_err(|error| format!("accept: {error}"))?;
    let session = respond(accepted, &identity).map_err(|error| format!("handshake: {error}"))?;
    let (stream, sealer, opener) = session.into_parts();

    let (mut stream, writer, outbound) = spawn_writer(stream)?;
    let mut receiver = Receiver::new(sealer, opener);
    let mut sink: Option<FileSink> = None;
    let mut held = HeldBytes::default();

    loop {
        match stream.read_frame() {
            Ok(Some(frame)) => {
                let bytes = frame
                    .try_encode()
                    .map_err(|error| format!("re-encode: {error}"))?;

                // A sink cannot exist before the manifest that describes it,
                // and the manifest arrives on the wire. Until then the engine
                // is handed a sink that refuses to be written to -- not one
                // that silently swallows, because a silent sink would turn
                // "we wrote content nobody planned for" into a passing run.
                let mut refusing = RefusingSink::default();
                let answers = match sink.as_mut() {
                    Some(target) => receiver.deliver(&bytes, target),
                    None => receiver.deliver(&bytes, &mut refusing),
                }
                .map_err(|error| format!("deliver: {error}"))?;
                if refusing.written {
                    return Err("content arrived before the manifest that describes it".to_owned());
                }

                // The manifest has landed by the time the frame carrying it
                // returns, so the sink can be built before any chunk can
                // possibly arrive.
                if sink.is_none()
                    && let Some(manifest) = receiver.manifest()
                {
                    sink = Some(
                        FileSink::new(&destination, manifest)
                            .map_err(|error| format!("sink: {error}"))?,
                    );
                }
                held.took(&answers);
                for answer in answers {
                    let size = answer.len();
                    outbound
                        .send(answer)
                        .map_err(|_| "writer gone".to_owned())?;
                    held.released(size);
                }
            }
            Ok(None) => {}
            Err(error) => {
                if receiver.phase() == Phase::Done {
                    break;
                }
                drop(outbound);
                let _ = writer.join();
                return Err(format!("read: {error}"));
            }
        }
        if receiver.phase() == Phase::Done {
            break;
        }
    }

    drop(outbound);
    let _ = writer.join();

    let verdicts = receiver.verdicts();
    let mut materialised = Vec::new();
    if let Some(target) = sink.as_mut() {
        // `finish_item` is called for **every** item, including the ones the
        // engine already judged bad. There are two digest checks in this
        // system: the engine hashes the stream as it arrives, and `FileSink`
        // hashes the part file before renaming it. Only the second one owns the
        // part file, and it is the one that deletes it on a mismatch. Skipping
        // the call for an item the engine had already refused therefore leaves
        // the rejected bytes on disk under a name that says "transfer in
        // progress" -- which is exactly what
        // `the_receiver_refuses_a_file_whose_digest_does_not_match` caught.
        for (item_id, verdict) in &verdicts {
            match target.finish_item(*item_id) {
                Ok(path) => {
                    if *verdict == ItemVerdict::Ok {
                        materialised.push(path.display().to_string());
                    } else {
                        // The filesystem accepted what the engine refused. That
                        // is a contradiction, not a pass: two checks over the
                        // same bytes disagreeing means one of them is wrong.
                        return Err(format!(
                            "item {item_id}: engine said {verdict:?} but the file verified"
                        ));
                    }
                }
                Err(error) => {
                    if *verdict == ItemVerdict::Ok {
                        return Err(format!("finish item {item_id}: {error}"));
                    }
                    // Refused by both, part file removed by the sink. Nothing
                    // to materialise and nothing left behind.
                }
            }
        }
    }

    let ok = verdicts
        .iter()
        .all(|(_, verdict)| *verdict == ItemVerdict::Ok);
    println!(
        "{{\"role\":\"serve\",\"items\":{},\"all_ok\":{},\"peak_held_bytes\":{},\"materialised\":{}}}",
        verdicts.len(),
        ok,
        held.peak,
        materialised.len()
    );
    Ok(())
}

/// Sends: builds a manifest, dials, initiates, pumps until done.
fn send(args: &[String]) -> Result<(), String> {
    let addr: SocketAddr = args
        .first()
        .ok_or("send needs ip:port")?
        .parse()
        .map_err(|_| "address must be ip:port".to_owned())?;

    // An optional trailing `--corrupt-at <offset>` provokes the digest refusal.
    let mut files: Vec<String> = Vec::new();
    let mut corrupt_at: Option<u64> = None;
    let mut rest = args.get(1..).unwrap_or_default().iter();
    while let Some(argument) = rest.next() {
        if argument == "--corrupt-at" {
            corrupt_at = rest
                .next()
                .and_then(|value| value.parse().ok())
                .map(Some)
                .ok_or("--corrupt-at needs an offset")?;
        } else {
            files.push(argument.clone());
        }
    }
    if files.is_empty() {
        return Err("send needs at least one file".to_owned());
    }

    let planned: Vec<PlannedFile> = files
        .iter()
        .map(|path| {
            let source = PathBuf::from(path);
            let relative = source
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "file".to_owned());
            PlannedFile { source, relative }
        })
        .collect();

    let manifest =
        manifest_from_disk(1, 0, &planned).map_err(|error| format!("manifest: {error}"))?;

    let mut paths = BTreeMap::new();
    for (index, file) in planned.iter().enumerate() {
        let item_id = u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX);
        paths.insert(item_id, file.source.clone());
    }
    let file_source = FileSource::new(paths);
    let source: Box<dyn ContentSource> = match corrupt_at {
        Some(at) => Box::new(CorruptingSource {
            inner: file_source,
            at,
        }),
        None => Box::new(file_source),
    };

    let identity = DeviceIdentity::generate().map_err(|error| format!("identity: {error}"))?;
    let stream = dial(addr).map_err(|error| format!("dial: {error}"))?;
    let session = initiate(stream, &identity).map_err(|error| format!("handshake: {error}"))?;
    let (stream, sealer, opener) = session.into_parts();

    let (mut stream, writer, outbound) = spawn_writer(stream)?;
    let mut sender = Sender::new(sealer, opener, manifest);
    let mut held = HeldBytes::default();

    let opening = sender.open().map_err(|error| format!("open: {error}"))?;
    held.took(&opening);
    for frame in opening {
        let size = frame.len();
        outbound.send(frame).map_err(|_| "writer gone".to_owned())?;
        held.released(size);
    }

    loop {
        match stream.read_frame() {
            Ok(Some(frame)) => {
                let bytes = frame
                    .try_encode()
                    .map_err(|error| format!("re-encode: {error}"))?;
                sender
                    .deliver(&bytes)
                    .map_err(|error| format!("deliver: {error}"))?;
            }
            Ok(None) => {}
            Err(error) => {
                if sender.phase() == Phase::Done {
                    break;
                }
                drop(outbound);
                let _ = writer.join();
                return Err(format!("read: {error}"));
            }
        }

        let produced = sender
            .pump(source.as_ref())
            .map_err(|error| format!("pump: {error}"))?;
        held.took(&produced);
        for frame in produced {
            let size = frame.len();
            outbound.send(frame).map_err(|_| "writer gone".to_owned())?;
            held.released(size);
        }

        if sender.phase() == Phase::Done {
            break;
        }
    }

    drop(outbound);
    let _ = writer.join();

    let verdicts = sender.integrity().map(<[_]>::to_vec).unwrap_or_default();
    let ok = !verdicts.is_empty() && verdicts.iter().all(|(_, v)| *v == ItemVerdict::Ok);
    println!(
        "{{\"role\":\"send\",\"items\":{},\"all_ok\":{},\"peak_held_bytes\":{},\"bytes_sent\":{}}}",
        verdicts.len(),
        ok,
        held.peak,
        sender.bytes_sent()
    );
    Ok(())
}

/// The reading half, the writer thread, and the queue that feeds it.
type Wired = (FrameStream, thread::JoinHandle<()>, mpsc::Sender<Vec<u8>>);

/// Splits the socket and puts writing on its own thread.
///
/// ADR-0028 §6: a single thread deadlocks the moment the peer stops reading,
/// because the write blocks and takes the reading of the acknowledgements that
/// would unblock it down with it. With a window of sixteen 64 KiB chunks there
/// is a megabyte in flight per direction, comfortably more than a socket buffer,
/// so this is not a theoretical concern.
fn spawn_writer(stream: FrameStream) -> Result<Wired, String> {
    let writing_half = stream
        .try_clone_socket()
        .map_err(|error| format!("clone socket: {error}"))?;
    let mut writing =
        FrameStream::new(writing_half).map_err(|error| format!("wrap write half: {error}"))?;
    // The writing half never reads, so it is never subject to the
    // pre-authentication allowance; marking it keeps its buffer sized for real
    // frames rather than for a stranger's.
    writing.mark_authenticated();

    let (outbound, inbox) = mpsc::channel::<Vec<u8>>();
    let writer = thread::spawn(move || {
        // Ends when the channel closes, which happens when the sender side is
        // dropped. That is what makes "no thread survives a finished transfer"
        // true by construction rather than by hope.
        while let Ok(frame) = inbox.recv() {
            if writing.write_frame(&frame).is_err() {
                break;
            }
        }
        let _ = writing.flush();
    });
    Ok((stream, writer, outbound))
}

/// A sink that records having been written to, and writes nothing.
///
/// Used only for the frames that precede the manifest. If the engine ever wrote
/// through it, content would be arriving for a plan nobody has -- so it
/// remembers, and the caller turns that into a failure rather than losing bytes
/// quietly.
#[derive(Default)]
struct RefusingSink {
    written: bool,
}

impl qyro_transfer::ContentSink for RefusingSink {
    fn write_at(&mut self, _item_id: u32, _offset: u64, _bytes: &[u8]) {
        self.written = true;
    }
}
