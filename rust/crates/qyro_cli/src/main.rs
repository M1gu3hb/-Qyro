//! `qyro` — the terminal face of the engine.
//!
//! Specification: `docs/adr/ADR-0042-cli.md`, and the requirement it serves is
//! `R7` §2, in the owner's words: *«en su terminal pongo el comando, listo, sale
//! el logo, le das en recibir o enviar»*.
//!
//! # Why this exists next to a working GUI
//!
//! The machine this is for **cannot install anything**, may have no GPU Flutter
//! accepts, and may not run Windows 10. What it certainly has is a terminal.
//!
//! # What it is not
//!
//! Not a second engine. This calls `qyro_session` directly — Rust talking to
//! Rust, no C boundary, because that boundary exists so Dart can cross a
//! language limit and paying its toll here buys nothing (ADR-0042 §2).
//!
//! **The consequence, written where it will be read:** the engine now has two
//! consumers, and **a capability is not done until both reach it** or until it
//! is declared for one. That is exactly how the v1.0 broke — `Session::finish`
//! was alive for one consumer and dead for the other.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod flows;
mod optical;
mod term;

#[cfg(test)]
mod guards;

#[cfg(test)]
mod round_trip;

use std::io::{IsTerminal as _, Write as _};

use term::Vt;

/// What the binary was asked to do.
///
/// Parsed into a value before anything runs, so the menu and the flags reach
/// **the same code** (ADR-0042 §3). Two paths that do the same thing are two
/// paths that diverge.
#[derive(Debug, Eq, PartialEq)]
enum Command {
    /// No arguments: open the menu, if there is somebody to answer it.
    Menu,
    Send {
        file: String,
        to: String,
        expect: Option<String>,
    },
    Receive {
        out: Option<String>,
        expect: Option<String>,
    },
    WhoAmI,
    Find,
    /// Draw this device's pairing code as a QR the other phone can read.
    Qr,
    Beam {
        file: String,
    },
    Help,
    /// The arguments did not make sense, and the message says which.
    Refused(String),
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = parse(&args);
    let vt = term::detect_vt();

    let code = run(command, vt);
    std::process::exit(code);
}

/// Turns arguments into a [`Command`], and never fails silently.
fn parse(args: &[String]) -> Command {
    let Some(first) = args.first().map(String::as_str) else {
        return Command::Menu;
    };

    match first {
        "help" | "--help" | "-h" => Command::Help,
        "whoami" => Command::WhoAmI,
        "find" => Command::Find,
        "qr" => Command::Qr,
        "beam" => {
            let Some(file) = args.get(1).cloned() else {
                return Command::Refused("beam needs a file: qyro beam <file>".to_owned());
            };
            Command::Beam { file }
        }
        "send" => {
            let Some(file) = args.get(1).cloned() else {
                return Command::Refused(
                    "send needs a file: qyro send <file> --to <code>".to_owned(),
                );
            };
            let Some(to) = flag(args, "--to") else {
                return Command::Refused(
                    "send needs --to <code>, the pairing code the other device shows".to_owned(),
                );
            };
            Command::Send {
                file,
                to,
                expect: flag(args, "--expect"),
            }
        }
        "recv" | "receive" => Command::Receive {
            out: flag(args, "--out"),
            expect: flag(args, "--expect"),
        },
        other => Command::Refused(format!("unknown command '{other}'. Try: qyro help")),
    }
}

/// The value after `name`, if it is there.
fn flag(args: &[String], name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    args.get(index + 1).cloned()
}

/// Runs a command and returns the process exit code.
///
/// Separated from `main` so every path can be tested: a `main` that calls
/// `exit` is a `main` no test can observe.
fn run(command: Command, vt: Vt) -> i32 {
    match command {
        Command::Help => {
            print!("{}", help_text());
            0
        }
        Command::Menu => {
            // **ADR-0042 §3: a menu nobody can answer is a hang, not a menu.**
            // Detected rather than assumed, and the message names the flag that
            // was wanted -- an error that does not say what to do instead is
            // half an error.
            if !std::io::stdin().is_terminal() {
                eprintln!(
                    "qyro: no interactive terminal, so there is nobody to answer \
                     a menu.\n\
                     Use a command instead:\n\
                     \x20 qyro send <file> --to <code>\n\
                     \x20 qyro recv --out <directory>\n\
                     \x20 qyro whoami"
                );
                return 2;
            }
            menu_loop(vt)
        }
        Command::WhoAmI => flows::whoami(vt),
        Command::Find => flows::find(vt),
        Command::Qr => flows::qr(vt),
        Command::Beam { file } => flows::beam(&file, vt),
        Command::Send { file, to, expect } => flows::send(&file, &to, expect.as_deref(), vt),
        Command::Receive { out, expect } => flows::receive(out.as_deref(), expect.as_deref(), vt),
        Command::Refused(why) => {
            eprintln!("qyro: {why}");
            2
        }
    }
}

/// The interactive menu.
///
/// It collects the same values the flags carry and calls the same functions.
fn menu_loop(vt: Vt) -> i32 {
    loop {
        print!("{}", term::menu(env!("CARGO_PKG_VERSION"), vt));
        print!("> ");
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() || line.is_empty() {
            // EOF. Not an error: a person pressed Ctrl-D, or the terminal went
            // away.
            return 0;
        }

        match line.trim() {
            "1" => return flows::send_interactive(vt),
            "2" => return flows::receive(None, None, vt),
            "3" => return flows::whoami(vt),
            "4" => return flows::find(vt),
            "q" | "Q" | "quit" | "exit" => return 0,
            "" => {}
            other => println!("\n  '{other}' is not one of the choices.\n"),
        }
    }
}

fn help_text() -> String {
    format!(
        "qyro {version} -- direct file transfer, no cloud, no accounts\n\
         \n\
         USAGE\n\
         \x20 qyro                                  open the menu\n\
         \x20 qyro send <file> --to <code>          send without asking\n\
         \x20 qyro recv [--out <directory>]         receive without asking\n\
         \x20 qyro whoami                           this device's code\n\
         \x20 qyro find                             who else is on this network\n\
         \x20 qyro qr                               draw this device's code as a QR\n\
         \n\
         OPTIONS\n\
         \x20 --expect <fingerprint>   refuse unless the other device's\n\
         \x20                          fingerprint matches. There is no --yes:\n\
         \x20                          nothing is ever accepted on its own, and\n\
         \x20                          naming the fingerprint is how a script\n\
         \x20                          decides beforehand instead of blindly.\n\
         \n\
         An address is an IP and a port. Names are never resolved: on the\n\
         networks this is for there is nothing to resolve them with.\n",
        version = env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "a test that cannot fail loudly is not a test"
    )]

    use super::{Command, flag, help_text, parse};

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn no_arguments_is_the_menu() {
        assert_eq!(parse(&args(&[])), Command::Menu);
    }

    #[test]
    fn send_needs_both_halves_and_says_which_is_missing() {
        // An error that does not say what to do instead is half an error.
        let Command::Refused(why) = parse(&args(&["send"])) else {
            panic!("a send with no file was accepted");
        };
        assert!(why.contains("file"), "{why}");

        let Command::Refused(why) = parse(&args(&["send", "a.txt"])) else {
            panic!("a send with no destination was accepted");
        };
        assert!(why.contains("--to"), "{why}");
    }

    #[test]
    fn send_parses_its_three_values() {
        let parsed = parse(&args(&[
            "send",
            "a.txt",
            "--to",
            "QYRO1|1.2.3.4:49517|ab",
            "--expect",
            "cd",
        ]));
        assert_eq!(
            parsed,
            Command::Send {
                file: "a.txt".to_owned(),
                to: "QYRO1|1.2.3.4:49517|ab".to_owned(),
                expect: Some("cd".to_owned()),
            }
        );
    }

    #[test]
    fn an_unknown_command_is_refused_by_name() {
        let Command::Refused(why) = parse(&args(&["frobnicate"])) else {
            panic!("an unknown command was accepted");
        };
        assert!(why.contains("frobnicate"), "{why}");
        assert!(why.contains("qyro help"), "{why}");
    }

    #[test]
    fn a_flag_without_a_value_is_absent_rather_than_empty() {
        // `--to` at the end of the line has no value. Treating that as an empty
        // string would build a session against `""` and fail somewhere far from
        // the mistake.
        assert_eq!(flag(&args(&["send", "a.txt", "--to"]), "--to"), None);
    }

    #[test]
    fn the_help_refuses_to_promise_a_yes_flag() {
        // ADR-0036 §1 has no exception for a terminal, and ADR-0042 §4 says so
        // in the help itself: somebody looking for `--yes` must find the reason
        // it is not there, not silence.
        let help = help_text();
        assert!(help.contains("no --yes"), "{help}");
        assert!(help.contains("--expect"), "{help}");
        assert!(
            help.contains("Names are never resolved"),
            "the help must say that names are not resolved, or somebody will \
             type a hostname and get a failure with no explanation"
        );
    }
}
