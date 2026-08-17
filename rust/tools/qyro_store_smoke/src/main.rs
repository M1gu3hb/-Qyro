//! Two-process persistence harness. **Never shipped.**
//!
//! One invocation creates an identity and exits. A *separate* invocation loads
//! it and compares the fingerprint. That separation is the entire point: two
//! calls inside one process share address space and prove nothing about a blob
//! surviving a process, which is the thing the sprint set out to demonstrate.
//!
//! `publish = false`, no crate of the product depends on it, and the same two
//! isolation guards that cover `qyro_crypto_smoke` cover this one (ADR-0023).
//!
//! Usage, and the two commands a CI log should show:
//!
//!     qyro_store_smoke create <path>
//!     qyro_store_smoke load   <path> <expected-fingerprint>
//!
//! And the pair that ADR-0040 added, which is the one that matters:
//!
//!     qyro_store_smoke session-open <path> [expected-fingerprint]
//!
//! The difference is not cosmetic. `create`/`load` drive
//! `qyro_win_dpapi::WindowsIdentityStore` **directly**, so the CI step
//! STATUS.md cited as proof that "an identity survives two process
//! invocations" proved that DPAPI round-trips — not that the product keeps an
//! identity. `session-open` goes through `qyro_session::open`, which is the
//! path the application takes. **Run twice against one path on any commit
//! before phase 11 and the two fingerprints differ**, because every session
//! generated its own keypair.

// Not `forbid(unsafe_code)`… it is. This harness has no unsafe of its own; the
// platform crate it calls is the one with the exception.
#![forbid(unsafe_code)]

use std::process::ExitCode;

#[cfg(test)]
mod guards;

/// Stable exit codes, so a runner reads a process status rather than a string.
///
/// Most are only read on Windows, where a backend exists. Every value stays an
/// explicit literal; the unsupported outcome is compiled only where reachable.
#[cfg_attr(not(windows), allow(dead_code, reason = "no backend off Windows"))]
mod code {
    pub const OK: u8 = 0;
    pub const USAGE: u8 = 1;
    pub const CREATE_FAILED: u8 = 2;
    pub const LOAD_FAILED: u8 = 3;
    pub const FINGERPRINT_MISMATCH: u8 = 4;
    /// `session-open` could not open an identity at all. ADR-0040.
    pub const SESSION_OPEN_FAILED: u8 = 6;
    #[cfg(not(windows))]
    pub const UNSUPPORTED_PLATFORM: u8 = 5;
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        eprintln!("usage: qyro_store_smoke <create|load|session-open> <path> [fingerprint]");
        return ExitCode::from(code::USAGE);
    };
    // Available on every platform, unlike `create`/`load`: the sandbox
    // protection of ADR-0040 needs no platform wrapper, so the cross-process
    // property can be exercised on the Linux runner as well as on Windows.
    if command == "session-open" {
        return session_open(&args);
    }
    run(command, &args)
}

/// Opens the identity **through the engine** and prints its fingerprint.
///
/// With a third argument, compares and exits `FINGERPRINT_MISMATCH` on
/// disagreement. Two invocations of this against one path is the whole test:
/// the second process shares no memory with the first, so an identity that
/// matches came off the disk.
fn session_open(args: &[String]) -> ExitCode {
    let Some(path) = args.get(1) else {
        eprintln!("usage: qyro_store_smoke session-open <path> [fingerprint]");
        return ExitCode::from(code::USAGE);
    };

    // Sandbox rather than Platform so the command behaves identically on every
    // runner. What is under test is that the engine keeps one identity across
    // processes, not which wrapper protected it -- that is `create`/`load`'s
    // job on Windows, and it still runs.
    if let Err(error) = qyro_session::open(
        std::path::Path::new(path),
        qyro_session::Protection::Sandbox,
    ) {
        eprintln!("session-open failed: {error}");
        return ExitCode::from(code::SESSION_OPEN_FAILED);
    }

    let fingerprint = match qyro_session::fingerprint() {
        Ok(text) => text,
        Err(error) => {
            eprintln!("fingerprint failed: {error}");
            return ExitCode::from(code::SESSION_OPEN_FAILED);
        }
    };
    println!("{fingerprint}");

    match args.get(2) {
        None => ExitCode::from(code::OK),
        Some(expected) if expected == &fingerprint => ExitCode::from(code::OK),
        Some(expected) => {
            eprintln!("expected {expected}, got {fingerprint}");
            ExitCode::from(code::FINGERPRINT_MISMATCH)
        }
    }
}

#[cfg(windows)]
fn run(command: &str, args: &[String]) -> ExitCode {
    use qyro_identity_store::IdentityStore;
    use qyro_win_dpapi::WindowsIdentityStore;

    let Some(path) = args.get(1) else {
        eprintln!("usage: qyro_store_smoke <create|load> <path> [fingerprint]");
        return ExitCode::from(code::USAGE);
    };
    let store = WindowsIdentityStore::at(std::path::PathBuf::from(path));

    match command {
        "create" => match qyro_crypto::DeviceIdentity::generate() {
            Ok(identity) => match store.create(&identity) {
                Ok(()) => {
                    // The fingerprint goes to stdout so the second invocation
                    // can be handed it without the two sharing any memory.
                    println!("{}", identity.fingerprint());
                    ExitCode::from(code::OK)
                }
                Err(error) => {
                    eprintln!("create failed: {error}");
                    ExitCode::from(code::CREATE_FAILED)
                }
            },
            Err(error) => {
                eprintln!("identity generation failed: {error}");
                ExitCode::from(code::CREATE_FAILED)
            }
        },
        "load" => {
            let Some(expected) = args.get(2) else {
                eprintln!("load needs the fingerprint the first process printed");
                return ExitCode::from(code::USAGE);
            };
            match store.load() {
                Ok(identity) => {
                    let actual = identity.fingerprint().to_string();
                    if &actual == expected {
                        println!("{actual}");
                        ExitCode::from(code::OK)
                    } else {
                        eprintln!("fingerprint mismatch: expected {expected}, loaded {actual}");
                        ExitCode::from(code::FINGERPRINT_MISMATCH)
                    }
                }
                Err(error) => {
                    eprintln!("load failed: {error}");
                    ExitCode::from(code::LOAD_FAILED)
                }
            }
        }
        other => {
            eprintln!("unknown command {other}");
            ExitCode::from(code::USAGE)
        }
    }
}

#[cfg(not(windows))]
fn run(_command: &str, _args: &[String]) -> ExitCode {
    // Deliberately explicit rather than a silent success. A harness that exits
    // zero on a platform where it did nothing is how a CI step comes to mean
    // "this ran" when it means "this compiled".
    eprintln!("qyro_store_smoke: no identity store backend on this platform");
    ExitCode::from(code::UNSUPPORTED_PLATFORM)
}
