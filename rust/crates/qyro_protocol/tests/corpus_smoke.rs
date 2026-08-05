//! Replays the committed fuzz corpus on stable Rust.
//!
//! `cargo-fuzz` needs nightly, so a real campaign cannot run in the normal CI
//! job. This test feeds the same seed corpus through the same assertions the
//! fuzz target makes, which keeps the corpus honest and catches a regression
//! that a known-bad input would trigger. It is a smoke test, not fuzzing: it
//! proves nothing about inputs nobody has thought of yet.

use std::fs;
use std::path::PathBuf;

use qyro_protocol::{DecodedFrame, FrameDecoder, HEADER_LEN, MAX_PAYLOAD_LEN};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/corpus/frame_decoder")
        .canonicalize()
        .expect("frame decoder corpus is committed")
}

#[test]
fn every_corpus_input_is_handled_without_panicking() {
    let directory = corpus_dir();
    let mut seen = 0usize;

    for entry in fs::read_dir(&directory).expect("corpus directory is readable") {
        let path = entry.expect("directory entry").path();
        if !path.is_file() {
            continue;
        }
        let data = fs::read(&path).expect("corpus file is readable");
        seen += 1;

        // Replay at several chunk sizes: framing bugs usually hide at a split.
        for chunk_size in [1usize, 7, 48, 4096] {
            let mut decoder = FrameDecoder::new();
            let mut chunks = data.chunks(chunk_size);
            loop {
                let Some(chunk) = chunks.next() else {
                    break;
                };
                if decoder.push(chunk).is_err() {
                    break;
                }
                let mut drained = true;
                while drained {
                    match decoder.next_frame() {
                        Ok(Some(DecodedFrame::Message(frame))) => {
                            assert!(
                                frame.payload().len() <= MAX_PAYLOAD_LEN,
                                "{} accepted an oversize payload",
                                path.display()
                            );
                            assert_eq!(
                                frame.payload().len(),
                                frame.header().payload_len() as usize,
                                "{} payload disagreed with its header",
                                path.display()
                            );
                            assert_eq!(frame.header().header_len() as usize, HEADER_LEN);
                            assert_eq!(frame.header().trailer_len(), 0);
                        }
                        // An unknown type is delimited, so the stream survives.
                        // A sealed frame keeps its ciphertext; no corpus seed is
                        // sealed today, but the arm must exist.
                        Ok(Some(DecodedFrame::Unsupported(_) | DecodedFrame::Encrypted(_))) => {}
                        Ok(None) => drained = false,
                        Err(_) => {
                            drained = false;
                            // A framing error poisons the decoder, so stop here.
                            chunks = data[data.len()..].chunks(chunk_size);
                        }
                    }
                }
            }
        }
    }

    assert!(
        seen >= 20,
        "expected the committed corpus, found only {seen} inputs"
    );
}
