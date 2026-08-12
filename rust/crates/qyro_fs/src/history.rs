//! Append-only local transfer history, byte-for-byte and crash recoverable.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use core::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::{
    HistoryDirection, HistoryError, HistoryPeer, HistoryRecord, HistoryRepair, HistoryStatus,
};

const MAGIC: [u8; 8] = *b"QYRO-HST";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 12;
const RECORD_BODY_LEN: usize = 64;
const RECORD_PREFIX_LEN: usize = 4;
const RECORD_CRC_LEN: usize = 4;
const ENCODED_RECORD_LEN: usize = RECORD_PREFIX_LEN + RECORD_BODY_LEN + RECORD_CRC_LEN;

/// Maximum bytes accepted or produced by one history file.
pub const MAX_HISTORY_FILE_LEN: u64 = 16_777_216;

const HEADER: [u8; HEADER_LEN] = [
    b'Q', b'Y', b'R', b'O', b'-', b'H', b'S', b'T', VERSION, 0, 0, 0,
];

impl HistoryRecord {
    /// Validates and constructs a terminal transfer summary.
    ///
    /// # Errors
    ///
    /// Returns [`HistoryError::InvalidTimestamp`] for a negative Unix UTC time.
    pub const fn new(
        ended_at: i64,
        transfer_id: u64,
        peer: HistoryPeer,
        direction: HistoryDirection,
        status: HistoryStatus,
        item_count: u32,
        bytes_transferred: u64,
    ) -> Result<Self, HistoryError> {
        if ended_at < 0 {
            return Err(HistoryError::InvalidTimestamp { found: ended_at });
        }
        Ok(Self {
            ended_at,
            transfer_id,
            peer,
            direction,
            status,
            item_count,
            bytes_transferred,
        })
    }
}

/// An open append-only history and its in-memory chronological view.
pub struct TransferHistory {
    file: File,
    records: Vec<HistoryRecord>,
    repair: HistoryRepair,
    append_usable: bool,
}

impl fmt::Debug for TransferHistory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransferHistory")
            .field("records", &self.records.len())
            .field("repair", &self.repair)
            .field("append_usable", &self.append_usable)
            .finish_non_exhaustive()
    }
}

impl TransferHistory {
    /// Opens an existing history or creates a new header when the path is absent.
    ///
    /// A partial or corrupt record tail is removed before this returns. Header
    /// corruption and future versions are errors and are never repaired.
    ///
    /// # Errors
    ///
    /// Returns a typed [`HistoryError`] for invalid headers, oversized files or
    /// operating-system failures.
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        let mut file = open_or_create(path)?;
        let file_len = file.metadata()?.len();
        if exceeds_history_limit(file_len) {
            return Err(HistoryError::HistoryFileTooLarge {
                found: file_len,
                maximum: MAX_HISTORY_FILE_LEN,
            });
        }

        file.seek(SeekFrom::Start(0))?;
        let capacity =
            usize::try_from(file_len).map_err(|_| HistoryError::HistoryFileTooLarge {
                found: file_len,
                maximum: MAX_HISTORY_FILE_LEN,
            })?;
        let mut bytes = Vec::with_capacity(capacity);
        file.read_to_end(&mut bytes)?;
        validate_header(&bytes)?;
        let parsed = parse_records(&bytes);

        let present =
            u64::try_from(bytes.len()).map_err(|_| HistoryError::HistoryFileTooLarge {
                found: u64::MAX,
                maximum: MAX_HISTORY_FILE_LEN,
            })?;
        let valid_len =
            u64::try_from(parsed.valid_len).map_err(|_| HistoryError::HistoryFileTooLarge {
                found: u64::MAX,
                maximum: MAX_HISTORY_FILE_LEN,
            })?;
        let repair = if valid_len == present {
            HistoryRepair::Clean
        } else {
            file.set_len(valid_len)?;
            file.sync_data()?;
            HistoryRepair::TailDiscarded {
                bytes: present.saturating_sub(valid_len),
            }
        };

        Ok(Self {
            file,
            records: parsed.records,
            repair,
            append_usable: true,
        })
    }

    /// Appends and durably flushes one record.
    ///
    /// # Errors
    ///
    /// Refuses time going backwards, a full file or reuse after a failed write.
    /// Any I/O failure poisons appends until the file is reopened and repaired.
    pub fn append(&mut self, record: HistoryRecord) -> Result<(), HistoryError> {
        if !self.append_usable {
            return Err(HistoryError::NeedsReopen);
        }
        if let Some(previous) = self.records.last()
            && record.ended_at < previous.ended_at
        {
            return Err(HistoryError::OutOfOrder {
                previous: previous.ended_at,
                found: record.ended_at,
            });
        }

        let current = self.file.metadata()?.len();
        let encoded_len =
            u64::try_from(ENCODED_RECORD_LEN).map_err(|_| HistoryError::HistoryFileTooLarge {
                found: u64::MAX,
                maximum: MAX_HISTORY_FILE_LEN,
            })?;
        let prospective =
            current
                .checked_add(encoded_len)
                .ok_or(HistoryError::HistoryFileTooLarge {
                    found: u64::MAX,
                    maximum: MAX_HISTORY_FILE_LEN,
                })?;
        if exceeds_history_limit(prospective) {
            return Err(HistoryError::HistoryFileTooLarge {
                found: prospective,
                maximum: MAX_HISTORY_FILE_LEN,
            });
        }

        let encoded = encode_record(record);
        self.append_usable = false;
        self.file.seek(SeekFrom::End(0))?;
        self.file.write_all(&encoded)?;
        self.file.sync_data()?;
        self.records.push(record);
        self.append_usable = true;
        Ok(())
    }

    /// Chronological records loaded from disk plus successful appends.
    #[must_use]
    pub fn records(&self) -> &[HistoryRecord] {
        &self.records
    }

    /// Whether opening discarded a corrupt or partial tail.
    #[must_use]
    pub const fn repair(&self) -> HistoryRepair {
        self.repair
    }

    /// Newest `limit` records, newest first.
    pub fn latest(&self, limit: usize) -> impl Iterator<Item = &HistoryRecord> {
        self.records.iter().rev().take(limit)
    }

    /// Records for one complete peer fingerprint, in chronological order.
    pub fn for_peer(&self, peer: &HistoryPeer) -> impl Iterator<Item = &HistoryRecord> {
        self.records.iter().filter(|record| record.peer == *peer)
    }

    /// Records with one terminal status, in chronological order.
    pub fn with_status(&self, status: HistoryStatus) -> impl Iterator<Item = &HistoryRecord> {
        self.records
            .iter()
            .filter(move |record| record.status == status)
    }
}

fn open_or_create(path: &Path) -> Result<File, HistoryError> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    if file.metadata()?.len() == 0 {
        file.write_all(&HEADER)?;
        file.sync_data()?;
    }
    Ok(file)
}

const fn exceeds_history_limit(found: u64) -> bool {
    found > MAX_HISTORY_FILE_LEN
}

fn validate_header(bytes: &[u8]) -> Result<(), HistoryError> {
    if bytes.len() < HEADER_LEN {
        return Err(HistoryError::TruncatedHistoryHeader { found: bytes.len() });
    }
    let Some(magic) = bytes.get(..MAGIC.len()) else {
        return Err(HistoryError::TruncatedHistoryHeader { found: bytes.len() });
    };
    if magic != MAGIC {
        return Err(HistoryError::NotTransferHistory);
    }
    let Some(&version) = bytes.get(8) else {
        return Err(HistoryError::TruncatedHistoryHeader { found: bytes.len() });
    };
    if version != VERSION {
        return Err(HistoryError::UnsupportedHistoryVersion { found: version });
    }
    let Some(reserved) = bytes.get(9..HEADER_LEN) else {
        return Err(HistoryError::TruncatedHistoryHeader { found: bytes.len() });
    };
    if reserved != [0u8; 3] {
        return Err(HistoryError::HistoryReservedNotZero);
    }
    Ok(())
}

struct ParsedRecords {
    records: Vec<HistoryRecord>,
    valid_len: usize,
    #[cfg(test)]
    records_examined: usize,
}

fn parse_records(bytes: &[u8]) -> ParsedRecords {
    let mut records = Vec::new();
    let mut offset = HEADER_LEN;
    let mut previous_time = None;
    #[cfg(test)]
    let mut records_examined = 0usize;

    while offset < bytes.len() {
        let record_start = offset;
        #[cfg(test)]
        {
            records_examined = records_examined.saturating_add(1);
        }
        let Some(prefix_end) = offset.checked_add(RECORD_PREFIX_LEN) else {
            break;
        };
        let Some(length_bytes) = bytes.get(offset..prefix_end) else {
            break;
        };
        let mut encoded_length = [0u8; 4];
        encoded_length.copy_from_slice(length_bytes);
        let declared = u32::from_be_bytes(encoded_length);
        if declared != u32::try_from(RECORD_BODY_LEN).unwrap_or(u32::MAX) {
            break;
        }

        let body_start = prefix_end;
        let Some(body_end) = body_start.checked_add(RECORD_BODY_LEN) else {
            break;
        };
        let Some(crc_end) = body_end.checked_add(RECORD_CRC_LEN) else {
            break;
        };
        let Some(body) = bytes.get(body_start..body_end) else {
            break;
        };
        let Some(crc_bytes) = bytes.get(body_end..crc_end) else {
            break;
        };
        let mut encoded_crc = [0u8; 4];
        encoded_crc.copy_from_slice(crc_bytes);
        if crc32(body) != u32::from_be_bytes(encoded_crc) {
            break;
        }
        let Some(record) = decode_record(body) else {
            break;
        };
        if previous_time.is_some_and(|previous| record.ended_at < previous) {
            break;
        }

        records.push(record);
        previous_time = Some(record.ended_at);
        offset = crc_end;
        if offset <= record_start {
            break;
        }
    }

    ParsedRecords {
        records,
        valid_len: offset,
        #[cfg(test)]
        records_examined,
    }
}

fn encode_record(record: HistoryRecord) -> [u8; ENCODED_RECORD_LEN] {
    let mut body = Vec::with_capacity(RECORD_BODY_LEN);
    body.extend_from_slice(&record.ended_at.to_be_bytes());
    body.extend_from_slice(&record.transfer_id.to_be_bytes());
    body.extend_from_slice(record.peer.as_bytes());
    body.push(direction_to_wire(record.direction));
    body.push(status_to_wire(record.status));
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(&record.item_count.to_be_bytes());
    body.extend_from_slice(&record.bytes_transferred.to_be_bytes());

    let mut encoded = [0u8; ENCODED_RECORD_LEN];
    let (prefix, rest) = encoded.split_at_mut(RECORD_PREFIX_LEN);
    prefix.copy_from_slice(
        &u32::try_from(RECORD_BODY_LEN)
            .unwrap_or(u32::MAX)
            .to_be_bytes(),
    );
    let (destination_body, destination_crc) = rest.split_at_mut(RECORD_BODY_LEN);
    destination_body.copy_from_slice(&body);
    destination_crc.copy_from_slice(&crc32(&body).to_be_bytes());
    encoded
}

fn decode_record(body: &[u8]) -> Option<HistoryRecord> {
    if body.len() != RECORD_BODY_LEN {
        return None;
    }
    let ended_at = i64::from_be_bytes(body.get(0..8)?.try_into().ok()?);
    if ended_at < 0 {
        return None;
    }
    let transfer_id = u64::from_be_bytes(body.get(8..16)?.try_into().ok()?);
    let peer = HistoryPeer::from_bytes(body.get(16..48)?.try_into().ok()?);
    let direction = direction_from_wire(*body.get(48)?)?;
    let status = status_from_wire(*body.get(49)?)?;
    if body.get(50..52)? != [0u8; 2] {
        return None;
    }
    let item_count = u32::from_be_bytes(body.get(52..56)?.try_into().ok()?);
    let bytes_transferred = u64::from_be_bytes(body.get(56..64)?.try_into().ok()?);
    HistoryRecord::new(
        ended_at,
        transfer_id,
        peer,
        direction,
        status,
        item_count,
        bytes_transferred,
    )
    .ok()
}

const fn direction_to_wire(direction: HistoryDirection) -> u8 {
    match direction {
        HistoryDirection::Sent => 1,
        HistoryDirection::Received => 2,
    }
}

const fn direction_from_wire(value: u8) -> Option<HistoryDirection> {
    match value {
        1 => Some(HistoryDirection::Sent),
        2 => Some(HistoryDirection::Received),
        _ => None,
    }
}

const fn status_to_wire(status: HistoryStatus) -> u8 {
    match status {
        HistoryStatus::Completed => 1,
        HistoryStatus::Cancelled => 2,
        HistoryStatus::Failed => 3,
    }
}

const fn status_from_wire(value: u8) -> Option<HistoryStatus> {
    match value {
        1 => Some(HistoryStatus::Completed),
        2 => Some(HistoryStatus::Cancelled),
        3 => Some(HistoryStatus::Failed),
        _ => None,
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0xedb8_8320;
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low_bit_mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (POLYNOMIAL & low_bit_mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    const TEN_THOUSAND_PARSE_BUDGET: Duration = Duration::from_millis(500);

    fn within_parse_budget(elapsed: Duration, budget: Duration) -> bool {
        elapsed <= budget
    }

    fn encoded_history(record_count: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            HEADER_LEN.saturating_add(record_count.saturating_mul(ENCODED_RECORD_LEN)),
        );
        bytes.extend_from_slice(&HEADER);
        for index in 0..record_count {
            let transfer_id = u64::try_from(index).unwrap_or(u64::MAX);
            let record = HistoryRecord {
                ended_at: i64::try_from(index).unwrap_or(i64::MAX),
                transfer_id,
                peer: HistoryPeer::from_bytes([0x42; 32]),
                direction: HistoryDirection::Received,
                status: HistoryStatus::Completed,
                item_count: 3,
                bytes_transferred: 1_024,
            };
            bytes.extend_from_slice(&encode_record(record));
        }
        bytes
    }

    #[test]
    fn ten_thousand_records_have_the_measured_size_and_parse_inside_the_budget() {
        let bytes = encoded_history(10_000);
        let started = Instant::now();
        let parsed = parse_records(&bytes);
        let elapsed = started.elapsed();

        assert_eq!(bytes.len(), 720_012);
        assert_eq!(parsed.records.len(), 10_000);
        assert_eq!(parsed.records_examined, 10_000);
        assert!(
            within_parse_budget(elapsed, TEN_THOUSAND_PARSE_BUDGET),
            "10,000 records took {elapsed:?}, budget is {TEN_THOUSAND_PARSE_BUDGET:?}"
        );
        eprintln!(
            "10,000 history records: {} bytes parsed in {elapsed:?}",
            bytes.len()
        );
    }

    #[test]
    fn a_slow_parse_would_be_visible_to_the_startup_measurement() {
        let deliberately_slow = TEN_THOUSAND_PARSE_BUDGET + Duration::from_nanos(1);

        assert!(!within_parse_budget(
            deliberately_slow,
            TEN_THOUSAND_PARSE_BUDGET
        ));
    }

    #[test]
    fn the_parse_work_counter_grows_when_the_file_grows() {
        let small = parse_records(&encoded_history(10));
        let large = parse_records(&encoded_history(20));

        assert_eq!(small.records_examined, 10);
        assert_eq!(large.records_examined, 20);
        assert!(large.records_examined > small.records_examined);
    }

    #[test]
    fn crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn the_file_limit_accepts_the_exact_boundary_and_refuses_one_more() {
        assert!(!exceeds_history_limit(MAX_HISTORY_FILE_LEN));
        assert!(exceeds_history_limit(MAX_HISTORY_FILE_LEN + 1));
    }

    #[test]
    fn equal_times_stay_valid_and_a_decrease_discards_the_tail() {
        let fixture = |ended_at, transfer_id, peer_byte, status| HistoryRecord {
            ended_at,
            transfer_id,
            peer: HistoryPeer::from_bytes([peer_byte; 32]),
            direction: HistoryDirection::Received,
            status,
            item_count: 1,
            bytes_transferred: 1,
        };
        let first = fixture(100, 1, 1, HistoryStatus::Completed);
        let equal = fixture(100, 2, 2, HistoryStatus::Cancelled);
        let older = fixture(99, 3, 3, HistoryStatus::Failed);
        let mut bytes = HEADER.to_vec();
        bytes.extend_from_slice(&encode_record(first));
        bytes.extend_from_slice(&encode_record(equal));
        bytes.extend_from_slice(&encode_record(older));

        let parsed = parse_records(&bytes);

        assert_eq!(parsed.records, [first, equal]);
        assert_eq!(parsed.valid_len, HEADER_LEN + 2 * ENCODED_RECORD_LEN);
    }
}
