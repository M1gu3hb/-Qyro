//! Runs the Qyro crypto smoke and reports the result. **Never shipped.**
//!
//! Exists so a runner can execute the flow on a platform where `cargo test`
//! cannot: an Android emulator takes a binary over `adb`, not a test harness.
//!
//! Prints one line and, with `--json`, a machine-readable report. Neither
//! carries key material: the report holds a target triple, an outcome and a
//! duration, which is everything a CI log needs and nothing a secret would fit
//! into.

use std::process::ExitCode;
use std::time::Instant;

use qyro_crypto_smoke::{SmokeOutcome, run};

fn main() -> ExitCode {
    let json = std::env::args().any(|argument| argument == "--json");

    let started = Instant::now();
    let outcome = run();
    let elapsed = started.elapsed();

    if json {
        // Hand-written rather than pulled in with serde: this crate is a test
        // harness that has to cross-compile to four platforms, and three fields
        // do not justify a dependency on every one of them.
        println!(
            "{{\"target\":\"{}\",\"outcome\":\"{}\",\"code\":{},\"duration_ms\":{}}}",
            current_target(),
            outcome,
            outcome.code(),
            elapsed.as_millis()
        );
    } else if outcome == SmokeOutcome::Success {
        println!(
            "[PASS] qyro crypto smoke on {}: handshake, seal, wire, open, replay and tamper ({} ms)",
            current_target(),
            elapsed.as_millis()
        );
    } else {
        eprintln!(
            "[FAIL] qyro crypto smoke on {}: {} (code {})",
            current_target(),
            outcome,
            outcome.code()
        );
    }

    if outcome == SmokeOutcome::Success {
        ExitCode::SUCCESS
    } else {
        // Codes stay under 256 by construction; the enum has twelve variants.
        ExitCode::from(u8::try_from(outcome.code()).unwrap_or(u8::MAX))
    }
}

/// The platform this binary is running on.
///
/// Built from `std::env::consts`, which the compiler fills in for the *target*
/// triple, so a report produced inside an Android emulator says `android-x86_64`
/// and not the triple of the machine that cross-compiled it.
fn current_target() -> String {
    format!(
        "{}-{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    )
}
