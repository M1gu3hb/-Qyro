//! The transfer engine, driven end to end over sealed frames.
//!
//! Every test here uses a **real** sealer and opener derived from a **real**
//! four-message handshake. There is no crypto double anywhere in this file: the
//! whole point of 5A is that the pieces work together, and a double would test
//! that they work with something else.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use qyro_crypto::DeviceIdentity;
use qyro_crypto::aead::{FrameOpener, FrameSealer};
use qyro_crypto::handshake::{InitiatorStart, ResponderStart};
use qyro_manifest::{HashAlgorithm, HashMetadata, ManifestItem, RelativePath, TransferManifest};
use sha2::{Digest, Sha256};

use crate::error::{ItemVerdict, TransferError};
use crate::session::{CHUNK_SIZE, ContentSink, ContentSource, Phase, Receiver, Sender};

// ------------------------------------------------------------ the real session

struct Ends {
    sender_sealer: FrameSealer,
    sender_opener: FrameOpener,
    receiver_sealer: FrameSealer,
    receiver_opener: FrameOpener,
}

/// A real handshake, and the two directions it establishes.
fn established() -> Ends {
    let alice = DeviceIdentity::generate().expect("identity");
    let bob = DeviceIdentity::generate().expect("identity");

    // The public entry points, which take their entropy from the system. A test
    // in this crate cannot ask for deterministic entropy — `qyro_crypto` keeps
    // those constructors crate-private on purpose (QYR-0069), and nothing here
    // needs determinism: what is under test is the engine, not the handshake.
    let (hello, awaiting) = InitiatorStart::new(&alice).send_hello().expect("opens");
    let (responder_hello, awaiting_finish) = ResponderStart::new(&bob)
        .receive_initiator_hello_from_system(&hello)
        .expect("accepts");
    let (finish, awaiting_responder_finish) = awaiting
        .receive_responder_hello(&responder_hello)
        .expect("verifies");
    let pending = awaiting_finish
        .receive_initiator_finish(&finish)
        .expect("verifies");
    let responder_finish = *pending.encoded_finish();
    let responder = pending.confirm_sent();
    let initiator = awaiting_responder_finish
        .receive_responder_finish(&responder_finish)
        .expect("verifies");

    let (sender_sealer, sender_opener) = initiator.into_frame_crypto().expect("derives");
    let (receiver_sealer, receiver_opener) = responder.into_frame_crypto().expect("derives");
    Ends {
        sender_sealer,
        sender_opener,
        receiver_sealer,
        receiver_opener,
    }
}

// ------------------------------------------------------------- content in memory

/// Content generated from a seed rather than stored.
///
/// Deliberately not a `Vec` of the whole file: the memory test needs a source
/// that *cannot* be holding the payload, so that what it measures is the engine
/// and not the fixture.
struct Generated {
    sizes: Vec<(u32, u64)>,
}

impl Generated {
    fn byte_at(item_id: u32, offset: u64) -> u8 {
        (offset.wrapping_mul(31).wrapping_add(u64::from(item_id)) % 251) as u8
    }

    fn size_of(&self, item_id: u32) -> u64 {
        self.sizes
            .iter()
            .find(|(id, _)| *id == item_id)
            .map_or(0, |(_, size)| *size)
    }

    fn digest_of(&self, item_id: u32) -> Vec<u8> {
        let mut hasher = Sha256::new();
        let size = self.size_of(item_id);
        let mut offset = 0u64;
        let mut buffer = [0u8; 4096];
        while offset < size {
            let want = ((size - offset).min(4096)) as usize;
            for (index, slot) in buffer[..want].iter_mut().enumerate() {
                *slot = Self::byte_at(item_id, offset + index as u64);
            }
            hasher.update(&buffer[..want]);
            offset += want as u64;
        }
        hasher.finalize().to_vec()
    }
}

impl ContentSource for Generated {
    fn read_at(&self, item_id: u32, offset: u64, out: &mut [u8]) -> usize {
        let size = self.size_of(item_id);
        let available = size.saturating_sub(offset);
        let want = (available.min(out.len() as u64)) as usize;
        for (index, slot) in out[..want].iter_mut().enumerate() {
            *slot = Self::byte_at(item_id, offset + index as u64);
        }
        want
    }
}

/// A sink that only hashes and counts, so the receiver's own memory is what the
/// memory test measures.
#[derive(Default)]
struct CountingSink {
    written: u64,
    peak_single_write: usize,
}

impl ContentSink for CountingSink {
    fn write_at(&mut self, _item_id: u32, _offset: u64, bytes: &[u8]) {
        self.written += bytes.len() as u64;
        self.peak_single_write = self.peak_single_write.max(bytes.len());
    }
}

/// A sink that keeps the bytes, for tests that need to look at them.
#[derive(Default)]
struct BufferSink {
    items: Vec<(u32, Vec<u8>)>,
}

impl ContentSink for BufferSink {
    fn write_at(&mut self, item_id: u32, offset: u64, bytes: &[u8]) {
        let slot = match self.items.iter_mut().find(|(id, _)| *id == item_id) {
            Some(entry) => &mut entry.1,
            None => {
                self.items.push((item_id, Vec::new()));
                &mut self.items.last_mut().expect("just pushed").1
            }
        };
        let start = offset as usize;
        if slot.len() < start + bytes.len() {
            slot.resize(start + bytes.len(), 0);
        }
        slot[start..start + bytes.len()].copy_from_slice(bytes);
    }
}

// ----------------------------------------------------------------- the fixture

fn manifest_for(source: &Generated) -> TransferManifest {
    let items: Vec<ManifestItem> = source
        .sizes
        .iter()
        .enumerate()
        .map(|(index, (item_id, size))| {
            let path = RelativePath::parse(&format!("file{index}.bin")).expect("path");
            let hash = HashMetadata::new(HashAlgorithm::Sha256, source.digest_of(*item_id))
                .expect("digest");
            ManifestItem::file(*item_id, path, *size, hash).expect("item")
        })
        .collect();
    TransferManifest::new(7, 0, items).expect("manifest")
}

/// Two items, several chunks each, one of them not a whole multiple of a chunk.
fn small_transfer() -> Generated {
    Generated {
        sizes: vec![
            (1, (CHUNK_SIZE * 2 + 1234) as u64),
            (2, (CHUNK_SIZE + 7) as u64),
        ],
    }
}

/// Drives both ends until neither has anything left to say.
///
/// Returns the number of round trips, so a test can tell "it finished" from "it
/// stopped moving".
fn run_to_quiet(
    sender: &mut Sender,
    receiver: &mut Receiver,
    source: &Generated,
    sink: &mut dyn ContentSink,
    max_rounds: usize,
) -> Result<usize, TransferError> {
    let mut to_receiver: Vec<Vec<u8>> = sender.open()?;
    let mut rounds = 0;

    while rounds < max_rounds {
        rounds += 1;
        let mut to_sender: Vec<Vec<u8>> = Vec::new();
        for frame in to_receiver.drain(..) {
            to_sender.extend(receiver.deliver(&frame, sink)?);
        }
        for frame in &to_sender {
            sender.deliver(frame)?;
        }
        to_receiver = sender.pump(source)?;
        if to_receiver.is_empty() && to_sender.is_empty() {
            break;
        }
    }
    Ok(rounds)
}

// -------------------------------------------------------------- the happy path

#[test]
fn a_multi_file_transfer_completes_over_sealed_frames() {
    let ends = established();
    let source = small_transfer();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = BufferSink::default();

    run_to_quiet(&mut sender, &mut receiver, &source, &mut sink, 200).expect("transfer");

    assert_eq!(sender.phase(), Phase::Done, "sender did not finish");
    assert_eq!(receiver.phase(), Phase::Done, "receiver did not finish");

    let verdicts = sender.integrity().expect("verdicts arrived");
    assert_eq!(verdicts.len(), 2);
    for (item_id, verdict) in verdicts {
        assert_eq!(*verdict, ItemVerdict::Ok, "item {item_id} was not ok");
    }

    // And the bytes really arrived, which "Ok" alone would not prove if the
    // receiver's own digest were the thing being compared to itself.
    for (item_id, size) in &source.sizes {
        let (_, got) = sink
            .items
            .iter()
            .find(|(id, _)| id == item_id)
            .expect("item arrived");
        assert_eq!(got.len() as u64, *size);
        for offset in [0u64, 1, 65_535, 65_536, size - 1] {
            assert_eq!(
                got[offset as usize],
                Generated::byte_at(*item_id, offset),
                "item {item_id} byte {offset} is wrong"
            );
        }
    }
}

// ------------------------------------------------------------------ corruption

#[test]
fn a_flipped_bit_in_a_chunk_is_a_typed_error() {
    let ends = established();
    let source = small_transfer();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = BufferSink::default();

    // Negotiate honestly, then corrupt exactly one content frame.
    let mut to_receiver = sender.open().expect("open");
    let mut to_sender = Vec::new();
    for frame in to_receiver.drain(..) {
        to_sender.extend(receiver.deliver(&frame, &mut sink).expect("negotiates"));
    }
    for frame in &to_sender {
        sender.deliver(frame).expect("accepts");
    }
    let chunks = sender.pump(&source).expect("chunks");
    let mut corrupted = chunks.last().expect("a chunk").clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x01;

    let outcome = receiver.deliver(&corrupted, &mut sink);
    assert_eq!(
        outcome.unwrap_err(),
        TransferError::NotAuthenticated,
        "a flipped bit must not become a file"
    );
    assert_eq!(
        receiver.phase(),
        Phase::Poisoned,
        "a frame that did not authenticate left the session usable"
    );
    assert!(
        sink.items.iter().all(|(_, bytes)| bytes.is_empty()),
        "content from an unauthenticated frame reached the sink"
    );
}

#[test]
fn the_receiver_verifies_every_digest_against_the_manifest() {
    let ends = established();
    let source = small_transfer();

    // A manifest whose first item claims a digest the content does not have.
    // The receiver must catch it at close, from its own hashing — not from
    // anything the sender says.
    let mut items: Vec<ManifestItem> = Vec::new();
    for (index, (item_id, size)) in source.sizes.iter().enumerate() {
        let path = RelativePath::parse(&format!("file{index}.bin")).expect("path");
        let mut digest = source.digest_of(*item_id);
        if index == 0 {
            digest[0] ^= 0xFF;
        }
        let hash = HashMetadata::new(HashAlgorithm::Sha256, digest).expect("digest");
        items.push(ManifestItem::file(*item_id, path, *size, hash).expect("item"));
    }
    let manifest = TransferManifest::new(7, 0, items).expect("manifest");

    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = CountingSink::default();
    run_to_quiet(&mut sender, &mut receiver, &source, &mut sink, 200).expect("runs");

    let verdicts = sender.integrity().expect("verdicts");
    assert_eq!(
        verdicts[0].1,
        ItemVerdict::DigestMismatch,
        "the mismatched item was accepted"
    );
    assert_eq!(
        verdicts[1].1,
        ItemVerdict::Ok,
        "the untouched item was not accepted, so this test passes for the wrong reason"
    );
}

// -------------------------------------------------------------- flow control

#[test]
fn the_sender_stops_when_the_receiver_stops_acking() {
    let ends = established();
    // One item big enough to need far more chunks than the window.
    let source = Generated {
        sizes: vec![(1, (CHUNK_SIZE * 64) as u64)],
    };
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = CountingSink::default();

    let mut to_receiver = sender.open().expect("open");
    let mut to_sender = Vec::new();
    for frame in to_receiver.drain(..) {
        to_sender.extend(receiver.deliver(&frame, &mut sink).expect("negotiates"));
    }
    for frame in &to_sender {
        sender.deliver(frame).expect("accepts");
    }

    // Now pump repeatedly and never deliver an ack.
    let first = sender.pump(&source).expect("first batch");
    let in_flight_after_first = sender.chunks_in_flight();
    let second = sender.pump(&source).expect("second batch");

    assert_eq!(
        second.len(),
        0,
        "the sender produced {} more chunks with nothing acknowledged",
        second.len()
    );
    assert_eq!(
        in_flight_after_first,
        crate::session::WINDOW_CHUNKS,
        "the sender stopped somewhere other than the window"
    );
    // An ItemStart rides along with the first chunk, so the batch is window + 1.
    assert_eq!(
        first.len() as u32,
        crate::session::WINDOW_CHUNKS + 1,
        "the first batch was not one window of chunks plus the ItemStart"
    );

    // And it starts again once an ack arrives — otherwise "it stopped" would
    // also be true of an engine that had simply broken.
    let ack_frames = receiver
        .deliver(&first[1], &mut sink)
        .expect("receiver takes the first chunk");
    for frame in &ack_frames {
        sender.deliver(frame).expect("ack accepted");
    }
    let third = sender.pump(&source).expect("third batch");
    assert!(
        !third.is_empty(),
        "the sender never resumed after an acknowledgement"
    );
}

// ------------------------------------------------------------ retransmission

#[test]
fn a_dropped_chunk_is_retransmitted_and_the_transfer_still_completes() {
    let ends = established();
    let source = small_transfer();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = BufferSink::default();

    let mut to_receiver = sender.open().expect("open");
    let mut dropped_one = false;
    let mut rounds = 0;

    while rounds < 400 {
        rounds += 1;
        let mut to_sender = Vec::new();
        for (index, frame) in to_receiver.drain(..).enumerate() {
            // Drop exactly one content frame, once.
            if !dropped_one && index == 2 {
                dropped_one = true;
                continue;
            }
            to_sender.extend(receiver.deliver(&frame, &mut sink).expect("delivers"));
        }
        for frame in &to_sender {
            sender.deliver(frame).expect("accepts");
        }
        to_receiver = sender.pump(&source).expect("pumps");

        if to_receiver.is_empty() && to_sender.is_empty() {
            if sender.phase() == Phase::Done {
                break;
            }
            // Stalled on the gap: go back to the first chunk the receiver has
            // not confirmed. Everything after it was discarded, so resending
            // only the one that was dropped would leave the rest missing.
            let resent = sender
                .retransmit(1, 1, &source)
                .expect("retransmits a new frame");
            to_receiver.push(resent);
        }
    }

    assert!(dropped_one, "the test never dropped anything");
    assert_eq!(sender.phase(), Phase::Done, "the transfer did not recover");
    for (item_id, verdict) in sender.integrity().expect("verdicts") {
        assert_eq!(
            *verdict,
            ItemVerdict::Ok,
            "item {item_id} did not survive the drop"
        );
    }
}

#[test]
fn a_replayed_chunk_is_refused() {
    // The replay window belongs to FrameOpener and already exists. This checks
    // the engine actually routes through it instead of around it.
    let ends = established();
    let source = small_transfer();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = BufferSink::default();

    let mut to_receiver = sender.open().expect("open");
    let mut to_sender = Vec::new();
    for frame in to_receiver.drain(..) {
        to_sender.extend(receiver.deliver(&frame, &mut sink).expect("negotiates"));
    }
    for frame in &to_sender {
        sender.deliver(frame).expect("accepts");
    }
    let chunks = sender.pump(&source).expect("chunks");
    let first = chunks.first().expect("a frame").clone();

    receiver
        .deliver(&first, &mut sink)
        .expect("the first delivery is fine");
    let replayed = receiver.deliver(&first, &mut sink);
    assert_eq!(
        replayed.unwrap_err(),
        TransferError::NotAuthenticated,
        "the same sealed bytes were accepted twice"
    );
}

// -------------------------------------------------------- pause, resume, cancel

#[test]
fn pause_and_resume_leave_both_ends_agreeing() {
    let ends = established();
    let source = small_transfer();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = CountingSink::default();

    let mut to_receiver = sender.open().expect("open");
    let mut to_sender = Vec::new();
    for frame in to_receiver.drain(..) {
        to_sender.extend(receiver.deliver(&frame, &mut sink).expect("negotiates"));
    }
    for frame in &to_sender {
        sender.deliver(frame).expect("accepts");
    }

    let pause = sender.request_pause().expect("pause");
    receiver.deliver(&pause, &mut sink).expect("receives pause");
    assert_eq!(sender.phase(), Phase::Paused);
    assert_eq!(receiver.phase(), Phase::Paused);

    assert!(
        sender.pump(&source).expect("paused pump").is_empty(),
        "a paused sender still produced content"
    );

    let resume = sender.request_resume().expect("resume");
    receiver
        .deliver(&resume, &mut sink)
        .expect("receives resume");
    assert_eq!(sender.phase(), Phase::Transferring);
    assert_eq!(receiver.phase(), Phase::Transferring);

    assert!(
        !sender.pump(&source).expect("resumed pump").is_empty(),
        "a resumed sender produced nothing"
    );
}

#[test]
fn cancel_from_either_end_leaves_both_agreeing() {
    for cancel_from_sender in [true, false] {
        let ends = established();
        let source = small_transfer();
        let manifest = manifest_for(&source);
        let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
        let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
        let mut sink = CountingSink::default();

        let mut to_receiver = sender.open().expect("open");
        let mut to_sender = Vec::new();
        for frame in to_receiver.drain(..) {
            to_sender.extend(receiver.deliver(&frame, &mut sink).expect("negotiates"));
        }
        for frame in &to_sender {
            sender.deliver(frame).expect("accepts");
        }

        if cancel_from_sender {
            let cancel = sender.request_cancel().expect("cancel");
            receiver.deliver(&cancel, &mut sink).expect("receives");
        } else {
            let cancel = receiver.request_cancel().expect("cancel");
            sender.deliver(&cancel).expect("receives");
        }

        assert_eq!(
            sender.phase(),
            Phase::Cancelled,
            "sender disagreed after a cancel from {}",
            if cancel_from_sender {
                "itself"
            } else {
                "the peer"
            }
        );
        assert_eq!(receiver.phase(), Phase::Cancelled, "receiver disagreed");
        assert_eq!(
            sender.pump(&source).unwrap_err(),
            TransferError::Cancelled,
            "a cancelled sender kept producing"
        );
    }
}

// ------------------------------------------------------------- illegal states

#[test]
fn a_message_in_the_wrong_state_is_refused_by_type() {
    use qyro_protocol::MessageType;

    // Four illegal transitions, each against a fresh pair so one refusal cannot
    // mask the next.
    let cases: [(MessageType, Vec<u8>); 4] = [
        (MessageType::DataChunk, vec![0u8; 16]),
        (MessageType::Complete, 0u64.to_be_bytes().to_vec()),
        (MessageType::ItemStart, vec![0u8; 12]),
        (MessageType::ChunkAck, vec![0u8; 8]),
    ];

    for (message_type, body) in cases {
        let ends = established();
        let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
        let mut sink = CountingSink::default();

        // Seal the illegal message with the sender's real sealer, so what is
        // being refused is the *state*, not the framing.
        let mut sealer = ends.sender_sealer;
        let frame = qyro_protocol::Frame::new(message_type, body).expect("frame");
        let sealed = sealer.seal(&frame).expect("seals");

        let outcome = receiver.deliver(&sealed.encode(), &mut sink);
        assert_eq!(
            outcome.unwrap_err(),
            TransferError::UnexpectedMessage { got: message_type },
            "{message_type:?} was not refused in the Negotiating phase"
        );
        assert_eq!(receiver.phase(), Phase::Poisoned);
    }
}

// -------------------------------------------------------------------- memory

#[test]
fn a_large_transfer_does_not_hold_the_whole_payload_in_memory() {
    // Eight megabytes across two items, none of it ever materialised: the source
    // generates bytes from a seed and the sink only counts them. If the engine
    // held the payload, the counter below would have to grow with it.
    let source = Generated {
        sizes: vec![(1, 4 * 1024 * 1024), (2, 4 * 1024 * 1024)],
    };
    let total: u64 = source.sizes.iter().map(|(_, size)| size).sum();

    let ends = established();
    let manifest = manifest_for(&source);
    let mut sender = Sender::new(ends.sender_sealer, ends.sender_opener, manifest);
    let mut receiver = Receiver::new(ends.receiver_sealer, ends.receiver_opener);
    let mut sink = CountingSink::default();

    run_to_quiet(&mut sender, &mut receiver, &source, &mut sink, 4000).expect("transfer");

    assert_eq!(
        sender.phase(),
        Phase::Done,
        "the large transfer did not finish"
    );
    assert_eq!(sink.written, total, "not every byte arrived");

    // The measurement. One chunk buffer, not one payload.
    assert_eq!(
        sender.peak_content_held, CHUNK_SIZE,
        "the sender held {} bytes at once against a chunk size of {CHUNK_SIZE}",
        sender.peak_content_held
    );
    assert_eq!(
        sink.peak_single_write, CHUNK_SIZE,
        "the receiver handed the sink {} bytes at once",
        sink.peak_single_write
    );
    // And the bound is far below the payload, which is the property that matters.
    assert!(
        (sender.peak_content_held as u64) * 64 < total,
        "the in-flight bound is not meaningfully smaller than the payload"
    );
}
