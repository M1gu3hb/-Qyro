//! Resume metadata, byte for byte as ADR-0027 §3 freezes it.
//!
//! Same standard as the identity blob: magic first, version refused by name,
//! and a future version rejected without interpreting anything. A format that
//! guesses what a version it does not know meant is a format with two readings.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::error::FsError;

/// Eight bytes that say what this file is before anything else is believed.
pub(crate) const MAGIC: [u8; 8] = *b"QYRO-RSM";

/// The only version this build writes or accepts.
pub(crate) const VERSION: u8 = 1;

/// Bytes before the per-item entries begin.
pub(crate) const HEADER_LEN: usize = 20;

/// Bytes per entry: `item_id` then `bytes_committed`.
pub(crate) const ENTRY_LEN: usize = 12;

/// How far one item got, as of the last commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ItemProgress {
    /// The manifest entry's id.
    pub item_id: u32,
    /// Bytes confirmed on disk. Anything past this was never committed.
    pub bytes_committed: u64,
}

/// What a transfer needs to carry on after the process that started it died.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeState {
    /// Which transfer this describes.
    pub transfer_id: u64,
    /// One entry per item, in manifest order.
    pub items: Vec<ItemProgress>,
}

impl ResumeState {
    /// Serialises header and entries.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let count = u16::try_from(self.items.len()).unwrap_or(u16::MAX);
        let mut out = Vec::with_capacity(HEADER_LEN + self.items.len() * ENTRY_LEN);
        out.extend_from_slice(&MAGIC);
        out.push(VERSION);
        out.push(0);
        out.extend_from_slice(&count.to_be_bytes());
        out.extend_from_slice(&self.transfer_id.to_be_bytes());
        for item in &self.items {
            out.extend_from_slice(&item.item_id.to_be_bytes());
            out.extend_from_slice(&item.bytes_committed.to_be_bytes());
        }
        out
    }

    /// Reads metadata, refusing anything this build does not understand.
    ///
    /// The order is the order, and each step decides a different error, exactly
    /// like the identity blob's read order.
    ///
    /// # Errors
    ///
    /// [`FsError::ResumeTruncated`], [`FsError::NotResumeMetadata`],
    /// [`FsError::UnsupportedResumeVersion`] or [`FsError::ResumeReservedNotZero`].
    pub fn decode(bytes: &[u8]) -> Result<Self, FsError> {
        let short = || FsError::ResumeTruncated { found: bytes.len() };

        // 1. Enough bytes to hold a header.
        let header = bytes.get(..HEADER_LEN).ok_or_else(short)?;

        // 2. Magic, before anything here is believed.
        if header.get(..8) != Some(&MAGIC[..]) {
            return Err(FsError::NotResumeMetadata);
        }

        // 3. Version, refused by name rather than guessed at.
        let version = *header.get(8).ok_or_else(short)?;
        if version != VERSION {
            return Err(FsError::UnsupportedResumeVersion { found: version });
        }

        // 4. Reserved must be zero. A field that is ignored is a field two
        //    versions read differently (ADR-0018).
        if *header.get(9).ok_or_else(short)? != 0 {
            return Err(FsError::ResumeReservedNotZero);
        }

        let count = usize::from(u16::from_be_bytes([
            *header.get(10).ok_or_else(short)?,
            *header.get(11).ok_or_else(short)?,
        ]));
        let mut id_bytes = [0u8; 8];
        id_bytes.copy_from_slice(header.get(12..20).ok_or_else(short)?);
        let transfer_id = u64::from_be_bytes(id_bytes);

        // 5. The declared count against the bytes actually present.
        let body = bytes.get(HEADER_LEN..).ok_or_else(short)?;
        let needed = count.checked_mul(ENTRY_LEN).ok_or_else(short)?;
        if body.len() != needed {
            return Err(short());
        }

        let mut items = Vec::with_capacity(count);
        for index in 0..count {
            let start = index.checked_mul(ENTRY_LEN).ok_or_else(short)?;
            let entry = body
                .get(start..start.checked_add(ENTRY_LEN).ok_or_else(short)?)
                .ok_or_else(short)?;
            let mut id = [0u8; 4];
            id.copy_from_slice(entry.get(..4).ok_or_else(short)?);
            let mut committed = [0u8; 8];
            committed.copy_from_slice(entry.get(4..12).ok_or_else(short)?);
            items.push(ItemProgress {
                item_id: u32::from_be_bytes(id),
                bytes_committed: u64::from_be_bytes(committed),
            });
        }

        Ok(Self { transfer_id, items })
    }

    /// How far `item_id` got, if this state mentions it.
    #[must_use]
    pub fn progress_of(&self, item_id: u32) -> Option<u64> {
        self.items
            .iter()
            .find(|item| item.item_id == item_id)
            .map(|item| item.bytes_committed)
    }
}
