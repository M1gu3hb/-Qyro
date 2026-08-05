#![no_main]
//! Drives the replay window through arbitrary sequences of transitions.
//!
//! The window is the one piece of AEAD state a peer influences directly: the
//! sequence number is in the header, and the header is whatever arrived. Its
//! two invariants are worth hunting for counterexamples to.
//!
//! 1. `check` decides and changes nothing, so calling it any number of times
//!    cannot alter what a later `record` does.
//! 2. Once a sequence is recorded, it is never accepted again — which is the
//!    entire purpose of the structure.

use libfuzzer_sys::fuzz_target;
use qyro_crypto::aead::AeadError;
use qyro_crypto::fuzzing::replay_window;

fuzz_target!(|data: &[u8]| {
    let mut window = replay_window();
    let mut recorded: Vec<u64> = Vec::new();

    for chunk in data.chunks_exact(8) {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(chunk);
        let sequence = u64::from_be_bytes(bytes);

        // `check` is pure: five calls must agree with each other and with what
        // `record` then does.
        let first = window.check(sequence);
        for _ in 0..4 {
            assert_eq!(window.check(sequence), first, "check must not mutate");
        }

        match window.record(sequence) {
            Ok(()) => {
                assert!(first.is_ok(), "record accepted what check refused");
                recorded.push(sequence);
                assert_eq!(
                    window.check(sequence),
                    Err(AeadError::ReplayDetected { sequence }),
                    "a recorded sequence must never be accepted again"
                );
            }
            Err(error) => {
                assert_eq!(Err(error), first, "record and check must agree");
                assert!(
                    matches!(
                        error,
                        AeadError::ReplayDetected { .. } | AeadError::SequenceTooOld { .. }
                    ),
                    "the window has exactly two ways to say no, got {error:?}"
                );
            }
        }
    }

    // Nothing that was accepted may be acceptable now, whatever happened after.
    for sequence in recorded {
        assert!(
            window.check(sequence).is_err(),
            "{sequence} was accepted once and must never be accepted again"
        );
    }
});
