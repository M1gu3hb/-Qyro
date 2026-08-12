//! Public contracts for the local append-only transfer history.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "filesystem contract fixtures must fail loudly and alter exact bytes"
)]

use qyro_fs::{
    HistoryDirection, HistoryError, HistoryPeer, HistoryRecord, HistoryRepair, HistoryStatus,
    TransferHistory,
};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

struct TestFile {
    path: PathBuf,
}

impl TestFile {
    fn new(label: &str) -> Self {
        let nonce = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "qyro-history-{label}-{}-{nonce}.bin",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn peer(byte: u8) -> HistoryPeer {
    HistoryPeer::from_bytes([byte; 32])
}

fn record(
    ended_at: i64,
    transfer_id: u64,
    peer: HistoryPeer,
    status: HistoryStatus,
) -> HistoryRecord {
    HistoryRecord::new(
        ended_at,
        transfer_id,
        peer,
        HistoryDirection::Received,
        status,
        3,
        1_024,
    )
    .unwrap()
}

#[test]
fn appended_records_survive_reopening_in_time_order() {
    let file = TestFile::new("round-trip");
    let mut history = TransferHistory::open(&file.path).unwrap();
    history
        .append(
            HistoryRecord::new(
                100,
                7,
                peer(0x11),
                HistoryDirection::Sent,
                HistoryStatus::Completed,
                3,
                2_048,
            )
            .unwrap(),
        )
        .unwrap();
    history
        .append(record(101, 8, peer(0x22), HistoryStatus::Failed))
        .unwrap();
    drop(history);

    let reopened = TransferHistory::open(&file.path).unwrap();

    assert_eq!(reopened.repair(), HistoryRepair::Clean);
    assert_eq!(reopened.records().len(), 2);
    assert_eq!(reopened.records()[0].transfer_id(), 7);
    assert_eq!(reopened.records()[0].ended_at(), 100);
    assert_eq!(reopened.records()[0].peer(), peer(0x11));
    assert_eq!(reopened.records()[0].direction(), HistoryDirection::Sent);
    assert_eq!(reopened.records()[0].status(), HistoryStatus::Completed);
    assert_eq!(reopened.records()[1].transfer_id(), 8);
    assert_eq!(reopened.records()[1].bytes_transferred(), 1_024);
    assert_eq!(reopened.records()[1].item_count(), 3);
    assert_eq!(
        reopened.records()[1].direction(),
        HistoryDirection::Received
    );
    assert_eq!(reopened.records()[1].status(), HistoryStatus::Failed);
    assert!(format!("{reopened:?}").contains("records: 2"));
}

#[test]
fn a_crc_corruption_discards_the_first_bad_record_and_everything_after_it() {
    const HEADER_LEN: usize = 12;
    const ENCODED_RECORD_LEN: usize = 72;
    const RECORD_PREFIX_LEN: usize = 4;
    let file = TestFile::new("crc-tail");
    let mut history = TransferHistory::open(&file.path).unwrap();
    for (time, id) in [(100, 1), (101, 2), (102, 3)] {
        history
            .append(record(
                time,
                id,
                peer(u8::try_from(id).unwrap()),
                HistoryStatus::Completed,
            ))
            .unwrap();
    }
    drop(history);

    let mut bytes = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&file.path)
        .unwrap();
    let corrupt_offset =
        u64::try_from(HEADER_LEN + ENCODED_RECORD_LEN + RECORD_PREFIX_LEN + 8).unwrap();
    bytes.seek(SeekFrom::Start(corrupt_offset)).unwrap();
    bytes.write_all(&[0xff]).unwrap();
    bytes.sync_all().unwrap();
    drop(bytes);

    let repaired = TransferHistory::open(&file.path).unwrap();

    assert_eq!(
        repaired.repair(),
        HistoryRepair::TailDiscarded {
            bytes: u64::try_from(2 * ENCODED_RECORD_LEN).unwrap()
        }
    );
    assert_eq!(repaired.records().len(), 1);
    assert_eq!(repaired.records()[0].transfer_id(), 1);
    assert_eq!(
        std::fs::metadata(&file.path).unwrap().len(),
        u64::try_from(HEADER_LEN + ENCODED_RECORD_LEN).unwrap()
    );
}

#[test]
fn a_history_from_a_future_version_is_refused_by_version() {
    let file = TestFile::new("future");
    drop(TransferHistory::open(&file.path).unwrap());
    let mut bytes = OpenOptions::new().write(true).open(&file.path).unwrap();
    bytes.seek(SeekFrom::Start(8)).unwrap();
    bytes.write_all(&[2]).unwrap();
    bytes.sync_all().unwrap();

    assert_eq!(
        TransferHistory::open(&file.path).unwrap_err(),
        HistoryError::UnsupportedHistoryVersion { found: 2 }
    );
}

#[test]
fn a_half_written_record_is_detected_truncated_and_previous_records_survive() {
    const HEADER_LEN: usize = 12;
    let file = TestFile::new("torn");
    let source = TestFile::new("record-source");

    let mut history = TransferHistory::open(&file.path).unwrap();
    history
        .append(record(100, 1, peer(0x11), HistoryStatus::Completed))
        .unwrap();
    history
        .append(record(101, 2, peer(0x22), HistoryStatus::Cancelled))
        .unwrap();
    drop(history);
    let valid_len = std::fs::metadata(&file.path).unwrap().len();

    let mut source_history = TransferHistory::open(&source.path).unwrap();
    source_history
        .append(record(102, 3, peer(0x33), HistoryStatus::Failed))
        .unwrap();
    drop(source_history);
    let encoded = std::fs::read(&source.path).unwrap();
    let encoded_record = &encoded[HEADER_LEN..];
    let half = encoded_record.len() / 2;
    let mut destination = OpenOptions::new().append(true).open(&file.path).unwrap();
    destination.write_all(&encoded_record[..half]).unwrap();
    destination.sync_all().unwrap();
    drop(destination);

    let repaired = TransferHistory::open(&file.path).unwrap();

    assert_eq!(
        repaired.repair(),
        HistoryRepair::TailDiscarded {
            bytes: u64::try_from(half).unwrap()
        }
    );
    assert_eq!(repaired.records().len(), 2);
    assert_eq!(repaired.records()[0].transfer_id(), 1);
    assert_eq!(repaired.records()[1].transfer_id(), 2);
    assert_eq!(std::fs::metadata(&file.path).unwrap().len(), valid_len);
}

#[test]
fn latest_peer_and_status_queries_filter_the_loaded_vector() {
    let file = TestFile::new("queries");
    let mut history = TransferHistory::open(&file.path).unwrap();
    let alice = peer(0xa1);
    let bob = peer(0xb2);
    for entry in [
        record(10, 1, alice, HistoryStatus::Completed),
        record(11, 2, bob, HistoryStatus::Failed),
        record(12, 3, alice, HistoryStatus::Failed),
        record(13, 4, alice, HistoryStatus::Cancelled),
    ] {
        history.append(entry).unwrap();
    }

    assert_eq!(
        history
            .latest(2)
            .map(HistoryRecord::transfer_id)
            .collect::<Vec<_>>(),
        [4, 3]
    );
    assert_eq!(
        history
            .for_peer(&alice)
            .map(HistoryRecord::transfer_id)
            .collect::<Vec<_>>(),
        [1, 3, 4]
    );
    assert_eq!(
        history
            .with_status(HistoryStatus::Failed)
            .map(HistoryRecord::transfer_id)
            .collect::<Vec<_>>(),
        [2, 3]
    );
}

#[test]
fn an_older_record_cannot_break_the_time_order() {
    let file = TestFile::new("order");
    let mut history = TransferHistory::open(&file.path).unwrap();
    history
        .append(record(100, 1, peer(0x11), HistoryStatus::Completed))
        .unwrap();

    assert_eq!(
        history.append(record(99, 2, peer(0x22), HistoryStatus::Completed)),
        Err(HistoryError::OutOfOrder {
            previous: 100,
            found: 99
        })
    );
    assert_eq!(history.records().len(), 1);

    history
        .append(record(100, 3, peer(0x33), HistoryStatus::Cancelled))
        .unwrap();
    assert_eq!(history.records().len(), 2);
}

#[test]
fn a_negative_history_timestamp_is_refused_by_name() {
    assert_eq!(
        HistoryRecord::new(
            -1,
            7,
            peer(0x11),
            HistoryDirection::Sent,
            HistoryStatus::Failed,
            0,
            0,
        ),
        Err(HistoryError::InvalidTimestamp { found: -1 })
    );
}

#[test]
fn history_errors_keep_their_named_diagnostic_and_unknown_io_code() {
    assert_eq!(
        HistoryError::UnsupportedHistoryVersion { found: 9 }.to_string(),
        "transfer history declares unsupported version 9"
    );
    assert_eq!(
        HistoryError::from(std::io::Error::other("no raw code")),
        HistoryError::Io { code: -1 }
    );
}
