//! Replays the committed fuzz corpus through the frame opener.
//!
//! Same shape and same limits as the corpus smoke tests in `qyro_protocol` and
//! `qyro_manifest`: `cargo-fuzz` needs nightly, so a real campaign cannot run in
//! the ordinary CI job, and this replays the committed seeds instead. **It is a
//! smoke test, not fuzzing.** It proves nothing about inputs nobody has thought
//! of yet.
//!
//! In-crate rather than in `tests/`, for the reason that applies to every test
//! in this crate that needs a fixed session: an integration test is a separate
//! crate, and the deterministic constructors are `cfg(test)` and crate-private.
//!
//! The corpus it reads is the frame decoder's, which since this milestone
//! contains sealed frames — genuine ones taken from the committed vectors, and
//! twelve mutations of them. The decoder's own smoke test checks that none of
//! them break framing. This one checks the layer above: that none of them get
//! past the AEAD, and that the genuine ones do.

use std::fs;
use std::path::PathBuf;

use qyro_protocol::{DecodedFrame, FrameDecoder};
use serde_json::Value;

use super::vectors::{COMMITTED, field, hex, run, unhex};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus/frame_decoder")
        .canonicalize()
        .expect("the frame decoder corpus is committed")
}

/// Every plaintext the committed vectors record, as hex.
///
/// Nothing outside this set may ever come out of `open`, whatever the corpus
/// contains: an opener that produced a fifth plaintext would be authenticating
/// something the sealer never sealed.
fn recorded_plaintexts() -> Vec<String> {
    let document: Value = serde_json::from_str(COMMITTED).expect("valid JSON");
    [
        "initiator_to_responder_frames",
        "responder_to_initiator_frames",
    ]
    .iter()
    .flat_map(|list| {
        document[*list]
            .as_array()
            .expect("an array")
            .iter()
            .map(|frame| field(frame, &["plaintext"]))
            .collect::<Vec<_>>()
    })
    .collect()
}

#[test]
fn no_corpus_input_panics_or_opens_into_something_unrecorded() {
    let directory = corpus_dir();
    let allowed = recorded_plaintexts();

    // Sorted, so the run does not depend on the order the filesystem hands back,
    // and a fresh opener per file, so one seed cannot make the next look like a
    // replay of itself.
    let mut paths: Vec<PathBuf> = fs::read_dir(&directory)
        .expect("the corpus directory is readable")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();

    let mut seen = 0usize;
    let mut sealed_seeds = 0usize;
    let mut opened = 0usize;

    for path in &paths {
        let data = fs::read(path).expect("corpus file is readable");
        seen += 1;

        for chunk_size in [1usize, 7, 48, 4096] {
            let mut session = run();
            let mut decoder = FrameDecoder::new();
            let mut chunks = data.chunks(chunk_size);

            while let Some(chunk) = chunks.next() {
                if decoder.push(chunk).is_err() {
                    break;
                }
                loop {
                    match decoder.next_frame() {
                        Ok(Some(DecodedFrame::Encrypted(envelope))) => {
                            if chunk_size == 1 {
                                sealed_seeds += 1;
                            }
                            // Both openers, because a corpus seed does not say
                            // which direction it belongs to and neither may be
                            // made to panic by a byte string.
                            for opener in
                                [&mut session.responder_opener, &mut session.initiator_opener]
                            {
                                if let Ok(frame) = opener.open(&envelope) {
                                    if chunk_size == 1 {
                                        opened += 1;
                                    }
                                    let plaintext = hex(frame.payload());
                                    assert!(
                                        allowed.contains(&plaintext),
                                        "{} opened into a plaintext no sealer produced",
                                        path.display()
                                    );
                                    assert_eq!(
                                        unhex(&plaintext).len(),
                                        envelope.ciphertext().len(),
                                        "{}: plaintext and ciphertext are the same length",
                                        path.display()
                                    );
                                }
                            }
                        }
                        Ok(Some(_)) => {}
                        Ok(None) => break,
                        Err(_) => {
                            // A framing error poisons the decoder; stop this file.
                            chunks = data[data.len()..].chunks(chunk_size);
                            break;
                        }
                    }
                }
            }
        }
    }

    assert!(
        seen >= 30,
        "expected the committed corpus, found only {seen} inputs"
    );
    assert!(
        sealed_seeds >= 10,
        "the corpus must carry sealed frames now that sealing exists, found {sealed_seeds}"
    );
    assert!(
        opened >= 4,
        "the genuine seeds must still open, or this test would pass by rejecting everything"
    );
}
