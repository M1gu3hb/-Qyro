#![no_main]
//! Mutates a genuine sealed frame and feeds it to the opener.
//!
//! A target that built a random session would spend its whole budget on an
//! eight-byte session comparison: essentially every input would die at
//! `WrongSession` before the AEAD ran. So the session is fixed, through
//! `qyro_crypto::fuzzing`, which exists only under `--cfg fuzzing`.
//!
//! Three properties, and the second is the one worth the compute:
//!
//! 1. no input panics;
//! 2. plaintext comes out only for frames that authenticate;
//! 3. a failed authentication does not move the replay window — the property
//!    that keeps someone without the key from stranding a session with sixteen
//!    random bytes.

use libfuzzer_sys::fuzz_target;
use qyro_crypto::aead::AeadError;
use qyro_crypto::fuzzing::{deterministic_session, plain_frame};
use qyro_protocol::{DecodedFrame, FrameDecoder};

const PAYLOAD: &[u8] = b"qyro frame opener fuzz payload";

fuzz_target!(|data: &[u8]| {
    let Some(mut session) = deterministic_session() else {
        return;
    };
    let Some(frame) = plain_frame(PAYLOAD.to_vec()) else {
        return;
    };
    let Ok(sealed) = session.sealer.seal(&frame) else {
        return;
    };
    let genuine = sealed.encode();

    // The input chooses where and how to corrupt the genuine frame, so the
    // fuzzer walks the header, the ciphertext and the tag rather than producing
    // bytes that die in the decoder.
    let mut mutated = genuine.clone();
    for pair in data.chunks_exact(3) {
        let offset = ((usize::from(pair[0]) << 8) | usize::from(pair[1])) % mutated.len();
        mutated[offset] ^= pair[2];
    }

    let mut decoder = FrameDecoder::new();
    if decoder.push(&mutated).is_err() {
        return;
    }
    let Ok(Some(DecodedFrame::Encrypted(envelope))) = decoder.next_frame() else {
        return;
    };

    match session.opener.open(&envelope) {
        Ok(opened) => {
            // Only an untouched frame can authenticate, so the plaintext is the
            // one that was sealed and nothing else.
            assert_eq!(opened.payload(), PAYLOAD);
            assert_eq!(opened.session_id(), session.session_id);
        }
        Err(AeadError::AuthenticationFailed) => {
            // The window must not have moved: the genuine frame still opens.
            let mut fresh = FrameDecoder::new();
            fresh.push(&genuine).expect("the genuine frame fits");
            if let Ok(Some(DecodedFrame::Encrypted(original))) = fresh.next_frame() {
                let opened = session
                    .opener
                    .open(&original)
                    .expect("a forged frame must not consume a real sequence");
                assert_eq!(opened.payload(), PAYLOAD);
            }
        }
        Err(_) => {}
    }
});
