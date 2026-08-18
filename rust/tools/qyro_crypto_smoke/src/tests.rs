use super::{SmokeOutcome, qyro_crypto_smoke_run, run};

#[test]
fn the_smoke_passes_on_this_host() {
    let outcome = run();
    assert_eq!(outcome, SmokeOutcome::Success, "{outcome}");
}

#[test]
fn the_c_entry_point_agrees_with_the_rust_one() {
    assert_eq!(qyro_crypto_smoke_run(), 0);
}

#[test]
fn every_outcome_has_a_distinct_stable_code() {
    // A runner reads these numbers out of an exit status. Two steps sharing a
    // code would make a failing log ambiguous.
    let outcomes = [
        SmokeOutcome::Success,
        SmokeOutcome::IdentityGeneration,
        SmokeOutcome::Handshake,
        SmokeOutcome::SessionMismatch,
        SmokeOutcome::FrameCryptoDerivation,
        SmokeOutcome::Seal,
        SmokeOutcome::WireRoundTrip,
        SmokeOutcome::Open,
        SmokeOutcome::PayloadMismatch,
        SmokeOutcome::ReplayNotDetected,
        SmokeOutcome::TamperNotDetected,
        SmokeOutcome::ReverseDirection,
    ];
    let mut seen = std::collections::HashSet::new();
    for outcome in outcomes {
        assert!(seen.insert(outcome.code()), "{outcome} reuses a code");
        assert!(!format!("{outcome}").is_empty());
    }
    assert_eq!(SmokeOutcome::Success.code(), 0, "0 means success");
}

#[test]
fn the_harness_exports_nothing_but_the_smoke_entry_point() {
    // The C surface is one function returning an integer. Anything that
    // returned bytes would have to say who owns them, and for key material the
    // answer is that they must not cross at all.
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is readable");
    let exports = source.matches("extern \"C\"").count();
    assert_eq!(exports, 1, "exactly one C export");
    for forbidden in ["*const", "*mut", "as_bytes", "key(", "seed"] {
        assert!(
            !source.contains(&format!("pub extern \"C\" fn {forbidden}")),
            "no C export may hand out {forbidden}"
        );
    }
}
