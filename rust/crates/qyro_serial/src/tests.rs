//! The serial channel over a pipe, with the wire made deliberately bad.
//!
//! **The class of evidence, written before the tests so nobody upgrades it:**
//! this is two halves of the protocol talking through an in-process queue. It is
//! **not** a physical UART and **not** a null-modem cable. Timing, framing
//! errors, a UART FIFO overrunning, and a cable with two wires crossed are all
//! outside it — and they are where a serial link actually fails. That is phase
//! 19, with hardware.
//!
//! What it *does* prove is everything the protocol is responsible for: that a
//! file crosses, that corruption is caught rather than delivered, that retries
//! are counted, and that a hopeless line ends in a named error rather than a
//! hang.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use crate::{MAX_ATTEMPTS, Receiver, Reply, SerialError, block_of, line_of, send_all, split};

fn payload_of(len: usize) -> Vec<u8> {
    // Not zeroes and not a repeating byte: a protocol that dropped a block
    // would still rebuild zeroes from zeroes and the test would pass.
    (0..len)
        .map(|index| ((index * 47 + 13) % 251) as u8)
        .collect()
}

/// A wire that corrupts one byte of every `corrupt_every`-th line it carries.
///
/// Corrupting the **Base64 payload** rather than the CRC field, because that is
/// what a bad cable does: the data is what is on the wire longest.
struct Wire {
    receiver: Receiver,
    carried: u32,
    corrupt_every: u32,
}

impl Wire {
    fn new(corrupt_every: u32) -> Self {
        Self {
            receiver: Receiver::new(),
            carried: 0,
            corrupt_every,
        }
    }

    fn carry(&mut self, line: &str) -> Result<Option<Reply>, SerialError> {
        self.carried += 1;
        let damaged = if self.corrupt_every > 0 && self.carried % self.corrupt_every == 0 {
            let mut bytes = line.as_bytes().to_vec();
            // The last byte is inside the Base64 payload for any real block.
            if let Some(last) = bytes.last_mut() {
                *last = if *last == b'A' { b'B' } else { b'A' };
            }
            String::from_utf8(bytes).unwrap_or_else(|_| line.to_owned())
        } else {
            line.to_owned()
        };
        Ok(self.receiver.accept(&damaged))
    }
}

#[test]
fn a_file_crosses_a_clean_line_with_no_retries() {
    let original = payload_of(4096);
    let mut wire = Wire::new(0);
    let tally = send_all(&original, |line| wire.carry(line)).expect("a clean line");

    assert_eq!(
        tally.blocks,
        u32::try_from(split(&original).len()).expect("small")
    );
    assert_eq!(tally.retries, 0, "a clean line cost retries");
    assert_eq!(
        wire.receiver.finish().as_deref(),
        Some(original.as_slice()),
        "the file came back different"
    );
}

#[test]
fn with_five_percent_of_lines_corrupted_it_still_completes_and_the_retries_are_counted() {
    // **The control the phase document asks for, and the number is asserted
    // rather than assumed.** One line in twenty is damaged; the transfer has to
    // finish, and it has to be able to say what that cost — because «4 blocks
    // were re-sent» is a sentence that tells somebody their cable is bad, and a
    // silent success is not.
    // 64 blocks, not 16: a wire that damages every twentieth line needs more
    // than twenty lines to damage anything, and the first draft of this test
    // asserted recovery over sixteen. It passed the file across and proved
    // nothing about corruption at all.
    let original = payload_of(32_768);
    let mut wire = Wire::new(20);
    let tally = send_all(&original, |line| wire.carry(line)).expect("5 % is survivable");

    assert_eq!(
        tally.blocks,
        u32::try_from(split(&original).len()).expect("small")
    );
    assert!(
        tally.retries > 0,
        "a wire that corrupted every twentieth line cost no retries, which \
         means the corruption never reached the checker"
    );
    assert!(
        tally.retries < tally.blocks,
        "{} retries for {} blocks is not recovery, it is thrashing",
        tally.retries,
        tally.blocks
    );
    assert_eq!(wire.receiver.finish().as_deref(), Some(original.as_slice()));
}

#[test]
fn with_a_hopeless_line_it_gives_up_by_name_and_does_not_hang() {
    // Every line corrupted. The requirement is **not** that it succeeds — it
    // cannot — but that it stops, quickly, with a sentence naming the block. A
    // hang is the failure nobody can diagnose.
    let original = payload_of(2048);
    let mut wire = Wire::new(1);
    let outcome = send_all(&original, |line| wire.carry(line));

    match outcome {
        Err(SerialError::GaveUp { index, attempts }) => {
            assert_eq!(
                index, 0,
                "it should give up on the first block it cannot land"
            );
            assert_eq!(attempts, MAX_ATTEMPTS);
        }
        other => panic!("a hopeless line did not end in a named failure: {other:?}"),
    }
}

#[test]
fn a_corrupt_block_is_rejected_and_never_delivered() {
    // The property everything else rests on. A checker that let a damaged block
    // through would produce a file that is *nearly* right, and this project has
    // written down what that costs (QYR-0359).
    let block = &split(&payload_of(crate::BLOCK_BYTES))[0];
    let good = line_of(block);
    let mut damaged = good.clone().into_bytes();
    let last = damaged.len() - 1;
    damaged[last] = if damaged[last] == b'A' { b'B' } else { b'A' };
    let damaged = String::from_utf8(damaged).expect("still ascii");

    assert!(block_of(&good).is_ok());
    assert!(
        matches!(block_of(&damaged), Err(SerialError::Corrupt { index: 0 })),
        "a damaged block was accepted"
    );
}

#[test]
fn somebody_elses_noise_gets_no_reply_at_all() {
    // A serial port emits rubbish when it opens and the wire may be shared.
    // Answering that with a NAK would put this protocol's traffic onto a line
    // it does not own, and a receiver that errored on it would never start.
    let mut receiver = Receiver::new();
    for noise in ["", "hello", "QS2 0 1 00000000 AAAA", "\u{0}\u{ff}garbage"] {
        assert_eq!(receiver.accept(noise), None, "it answered {noise:?}");
    }
    assert!(!receiver.is_complete());
}

#[test]
fn a_receiver_never_hands_back_a_partial_file() {
    let original = payload_of(2048);
    let blocks = split(&original);
    let mut receiver = Receiver::new();
    receiver.accept(&line_of(&blocks[0]));
    receiver.accept(&line_of(&blocks[2]));

    assert!(!receiver.is_complete());
    assert_eq!(
        receiver.finish(),
        None,
        "a partial transfer produced a file"
    );
    // Derived from the split, not written down: this said `2` and broke the day
    // BLOCK_BYTES moved from 512 to 510. A test that must be edited whenever a
    // constant changes is a test that will one day be edited to agree with a bug.
    assert_eq!(receiver.missing(), blocks.len() - 2);
}

#[test]
fn blocks_arriving_out_of_order_are_put_back_in_order() {
    // Not the common case on a serial line, but a retry means block 3 can land
    // after block 4. Reassembling by arrival order would produce a scrambled
    // file whose hash fails with nothing to point at.
    let original = payload_of(2048);
    let blocks = split(&original);
    let mut receiver = Receiver::new();
    // Every block, in an order no sender would choose: last first, then the
    // rest reversed. Derived from the split so it covers them all whatever
    // BLOCK_BYTES is.
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    order.reverse();
    order.swap(0, blocks.len() / 2);
    for index in order {
        receiver.accept(&line_of(&blocks[index]));
    }
    assert_eq!(receiver.finish().as_deref(), Some(original.as_slice()));
}

#[test]
fn a_reply_is_the_line_the_far_end_writes_and_back_again() {
    // The receiver on the far end is a PowerShell script writing `OK 3`. If
    // these two disagreed, every transfer would stall on the first block with
    // both ends convinced they were right.
    for reply in [
        Reply::Accepted(0),
        Reply::Rejected(7),
        Reply::Accepted(4096),
    ] {
        assert_eq!(Reply::of(&reply.line()), Some(reply));
    }
    assert_eq!(Reply::of("OK"), None);
    assert_eq!(Reply::of("MAYBE 3"), None);
    assert_eq!(Reply::of("OK three"), None);
}

#[test]
fn one_line_stays_inside_what_the_far_end_can_read() {
    // The PowerShell receiver reads with `ReadLine` into a 1 024-byte buffer by
    // default. A line longer than that is silently truncated on the far side and
    // arrives as a CRC failure that no amount of retrying will fix.
    let block = &split(&payload_of(crate::BLOCK_BYTES))[0];
    let line = line_of(block);
    assert!(
        line.len() < 1024,
        "a block line is {} bytes and the far end reads 1 024",
        line.len()
    );
    // And the control: the block is actually full, or this passes for the wrong
    // reason.
    assert_eq!(block.payload.len(), crate::BLOCK_BYTES);
}

/// The half of the pasted receiver that can be executed without a port.
///
/// **Because a generated script nobody has run is a script that does not work**,
/// and this project has already paid for that lesson. The port I/O cannot be
/// exercised here — this machine's only serial ports are Bluetooth endpoints,
/// not a linked pair — but the part most likely to be wrong can: that
/// `certutil -decode` really reassembles what [`crate::line_of`] produced.
///
/// The failure this closes is not hypothetical. A Base64 alphabet that differs
/// by one character, padding `certutil` refuses, or CRLF where it wants LF, all
/// produce a file that is *almost* right — and the transfer would report success
/// while the far machine held rubbish.
///
/// **Evidence class:** the real `certutil.exe`, on real bytes, on Windows. **Not**
/// over a serial port, **not** over a cable.
#[cfg(windows)]
#[test]
fn certutil_really_decodes_what_qyro_writes() {
    use std::io::Write as _;

    let original = payload_of(3000);
    let blocks = split(&original);

    // Exactly what the pasted script accumulates: field 4 of each QS1 line, one
    // per line, in the order they arrive.
    let mut accumulated = String::new();
    for block in &blocks {
        let line = line_of(block);
        let field = line.split(' ').nth(4).expect("a QS1 line has five fields");
        accumulated.push_str(field);
        accumulated.push_str("\r\n");
    }

    let dir = std::env::temp_dir().join(format!("qyro-serial-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    let encoded = dir.join("received.b64");
    let decoded = dir.join("received.bin");
    std::fs::File::create(&encoded)
        .and_then(|mut file| file.write_all(accumulated.as_bytes()))
        .expect("the encoded file is writable");

    let outcome = std::process::Command::new("certutil")
        .arg("-decode")
        .arg(&encoded)
        .arg(&decoded)
        .output()
        .expect("certutil has shipped with every Windows since XP");

    assert!(
        outcome.status.success(),
        "certutil refused what Qyro wrote -- which means the far machine would \
         hold nothing:\n{}\n{}",
        String::from_utf8_lossy(&outcome.stdout),
        String::from_utf8_lossy(&outcome.stderr)
    );

    let rebuilt = std::fs::read(&decoded).expect("certutil wrote the output");
    assert_eq!(
        rebuilt, original,
        "certutil decoded Qyro's Base64 into different bytes"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
