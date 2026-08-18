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
mod serial;
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
    /// Which way to send it, decided by the engine (ADR-0046 §4).
    How {
        file: Option<String>,
    },
    /// The serial channel: ports, and the receiver to paste into the old machine.
    Serial {
        port: Option<String>,
    },
    SerialSend {
        file: String,
        port: String,
    },
    SerialReceive {
        port: String,
        out: String,
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
        "how" => Command::How {
            file: args.get(1).cloned(),
        },
        "serial" => Command::Serial {
            port: flag(args, "--port"),
        },
        "beam" => {
            let Some(file) = args.get(1).cloned() else {
                return Command::Refused("beam needs a file: qyro beam <file>".to_owned());
            };
            Command::Beam { file }
        }
        "send" => {
            // **`--self` manda el propio binario** (fase 20 §2). Es la respuesta
            // al arranque: una vez hay un Qyro corriendo en una maquina, Qyro
            // puede llevarse a si mismo a la siguiente -- 800 KB, que por serie
            // son ochenta segundos y por QR un minuto y medio.
            //
            // Se resuelve aqui, en el analisis de argumentos, para que el flujo
            // de envio siga recibiendo una ruta y no tenga dos caminos.
            let file = if args.iter().any(|arg| arg == "--self") {
                match std::env::current_exe() {
                    Ok(path) => match path.to_str() {
                        Some(text) => text.to_owned(),
                        None => {
                            return Command::Refused(
                                "la ruta de este binario no es texto que se pueda poner en el cable"
                                    .to_owned(),
                            );
                        }
                    },
                    Err(error) => {
                        return Command::Refused(format!(
                            "no se pudo averiguar donde esta este binario: {error}"
                        ));
                    }
                }
            } else {
                let Some(file) = args.get(1).cloned() else {
                    return Command::Refused(
                        "send needs a file: qyro send <file> --to <code>, o qyro send --self"
                            .to_owned(),
                    );
                };
                file
            };
            let to = flag(args, "--to");
            if let Some(port) = flag(args, "--serial") {
                return Command::SerialSend { file, port };
            }
            let Some(to) = to else {
                return Command::Refused(
                    "send needs --to <code>, the pairing code the other device shows,                      or --serial <port>"
                        .to_owned(),
                );
            };
            Command::Send {
                file,
                to,
                expect: flag(args, "--expect"),
            }
        }
        "recv" | "receive" if flag(args, "--serial").is_some() => Command::SerialReceive {
            port: flag(args, "--serial").unwrap_or_default(),
            out: flag(args, "--out").unwrap_or_else(|| "qyro-received.bin".to_owned()),
        },
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
        Command::How { file } => flows::how(file.as_deref(), vt),
        Command::Serial { port } => serial::overview(port.as_deref(), vt),
        Command::SerialSend { file, port } => serial::send(&file, &port, vt),
        Command::SerialReceive { port, out } => serial::receive(&port, &out),
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
         \x20 qyro send --self --to <code>          send THIS binary (bootstrap)\n\
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

    #[test]
    fn send_self_manda_este_binario_y_no_pide_una_ruta() {
        // Fase 20 §2: la respuesta al arranque. Una vez hay un Qyro corriendo,
        // se lleva a si mismo a la siguiente maquina.
        let args = vec![
            "send".to_owned(),
            "--self".to_owned(),
            "--to".to_owned(),
            "QYRO1|1.2.3.4:49517|ab".to_owned(),
        ];
        let Command::Send { file, to, .. } = parse(&args) else {
            panic!("--self no produjo un envio");
        };
        assert_eq!(to, "QYRO1|1.2.3.4:49517|ab");
        let expected = std::env::current_exe().expect("un binario en marcha sabe donde esta");
        assert_eq!(
            std::path::Path::new(&file),
            expected,
            "--self mando otra cosa que no es este binario"
        );
    }

    #[test]
    fn y_sin_self_sigue_haciendo_falta_una_ruta() {
        // El control. Un `--self` que se aplicara siempre convertiria
        // `qyro send informe.pdf` en `qyro send qyro.exe`, en silencio.
        let args = vec![
            "send".to_owned(),
            "informe.pdf".to_owned(),
            "--to".to_owned(),
            "QYRO1|1.2.3.4:49517|ab".to_owned(),
        ];
        let Command::Send { file, .. } = parse(&args) else {
            panic!("un envio normal dejo de serlo");
        };
        assert_eq!(file, "informe.pdf");

        // Y sin ruta ni --self, se refusa con una frase que nombra las dos
        // salidas en vez de repetir la pregunta.
        let Command::Refused(why) = parse(&["send".to_owned()]) else {
            panic!("un send sin nada fue aceptado");
        };
        assert!(
            why.contains("--self"),
            "la negativa no menciona --self: {why}"
        );
    }

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
