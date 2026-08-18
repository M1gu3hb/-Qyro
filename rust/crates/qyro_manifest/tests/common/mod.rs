//! Hand-built manifest bytes.
//!
//! The construction API enforces the same invariants the decoder does, so a
//! manifest built through `TransferManifest::new` can never reach the decoder's
//! checks: it is rejected before it can be encoded. A test that wants to prove
//! the decoder rejects something has to hand it bytes nobody's constructor
//! would produce.
//!
//! Everything here writes exactly what it is told, including values the encoder
//! would refuse to emit. That is the point.

#![allow(dead_code, reason = "each test binary uses a different part of this")]

use qyro_manifest::{MANIFEST_MAGIC, MANIFEST_VERSION};

/// One item, as bytes, with no validation of any kind.
pub struct RawItem {
    pub item_id: u32,
    pub kind: u8,
    /// Written verbatim after the length prefix below.
    pub path: Vec<u8>,
    /// The `u32` length prefix actually written for `path`.
    ///
    /// Separate from `path.len()` so a test can declare one length and supply
    /// another.
    pub declared_path_len: Option<u32>,
    pub size: u64,
    /// `Some((declared_len, bytes))` writes the presence byte, that length
    /// prefix, and those bytes.
    pub mime: Option<(u32, Vec<u8>)>,
    pub modified: Option<i64>,
    pub hash_algorithm: u8,
    pub digest: Vec<u8>,
    pub compression: u8,
}

impl RawItem {
    /// A well-formed file item: SHA-256 digest, no MIME, no timestamp.
    pub fn file(item_id: u32, path: &str, size: u64) -> Self {
        Self {
            item_id,
            kind: 1,
            path: path.as_bytes().to_vec(),
            declared_path_len: None,
            size,
            mime: None,
            modified: None,
            hash_algorithm: 1,
            digest: vec![u8::try_from(item_id % 251).unwrap_or(0); 32],
            compression: 0,
        }
    }

    /// A well-formed directory item.
    pub fn directory(item_id: u32, path: &str) -> Self {
        Self {
            kind: 2,
            hash_algorithm: 0,
            digest: Vec::new(),
            ..Self::file(item_id, path, 0)
        }
    }

    /// Declares a MIME length without supplying that many bytes.
    pub fn with_declared_mime(mut self, declared_len: u32, bytes: &[u8]) -> Self {
        self.mime = Some((declared_len, bytes.to_vec()));
        self
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.item_id.to_be_bytes());
        out.push(self.kind);
        let declared = self
            .declared_path_len
            .unwrap_or_else(|| u32::try_from(self.path.len()).unwrap_or(u32::MAX));
        out.extend_from_slice(&declared.to_be_bytes());
        out.extend_from_slice(&self.path);
        out.extend_from_slice(&self.size.to_be_bytes());
        match &self.mime {
            Some((declared_len, bytes)) => {
                out.push(1);
                out.extend_from_slice(&declared_len.to_be_bytes());
                out.extend_from_slice(bytes);
            }
            None => out.push(0),
        }
        match self.modified {
            Some(seconds) => {
                out.push(1);
                out.extend_from_slice(&seconds.to_be_bytes());
            }
            None => out.push(0),
        }
        out.push(self.hash_algorithm);
        out.extend_from_slice(&self.digest);
        out.push(self.compression);
    }
}

/// A manifest assembled byte by byte.
pub struct RawManifest {
    pub magic: [u8; 4],
    pub version: u16,
    pub transfer_id: u64,
    pub created_unix_seconds: i64,
    /// Written verbatim into the header, whatever the items sum to.
    pub total_bytes: u64,
    /// Written verbatim into the header, whatever `items` holds.
    pub declared_item_count: Option<u32>,
    pub items: Vec<RawItem>,
}

impl RawManifest {
    /// A header whose declared values follow the items, so a test only has to
    /// change the one thing it is about.
    pub fn new(items: Vec<RawItem>) -> Self {
        let total_bytes = items.iter().fold(0u64, |sum, item| {
            sum.checked_add(item.size).unwrap_or(u64::MAX)
        });
        Self {
            magic: MANIFEST_MAGIC,
            version: MANIFEST_VERSION,
            transfer_id: 42,
            created_unix_seconds: 1_760_000_000,
            total_bytes,
            declared_item_count: None,
            items,
        }
    }

    /// Overrides the declared total without touching any item.
    #[must_use]
    pub const fn with_total_bytes(mut self, total_bytes: u64) -> Self {
        self.total_bytes = total_bytes;
        self
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.magic);
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&self.transfer_id.to_be_bytes());
        out.extend_from_slice(&self.created_unix_seconds.to_be_bytes());
        out.extend_from_slice(&self.total_bytes.to_be_bytes());
        let count = self
            .declared_item_count
            .unwrap_or_else(|| u32::try_from(self.items.len()).unwrap_or(u32::MAX));
        out.extend_from_slice(&count.to_be_bytes());
        for item in &self.items {
            item.write(&mut out);
        }
        out
    }
}
