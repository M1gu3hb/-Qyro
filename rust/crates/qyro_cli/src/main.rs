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
        /// The port to listen on, when the fixed one is not available.
        ///
        /// ADR-0041 §3 chose a fixed port so the Windows firewall is answered
        /// **once** and the pairing code is predictable, and in the same
        /// paragraph said what happens when it is taken: «se dice, no se mueve
        /// [...] y ofrece elegir otro». This flag is the «elegir otro», and it
        /// costs neither property — the pairing code carries the port inside,
        /// so a hand-picked one still works; what is lost is the convenience,
        /// not the function.
        port: Option<u16>,
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
        "recv" | "receive" => {
            // Parsed here rather than inside the flow, so a value that is not a
            // port is refused **before** anything prints a pairing code. A
            // `--port` that fell back to the default in silence would leave a
            // person listening on 49517 while believing otherwise, holding a
            // code that names a port nobody is on.
            let port = match flag(args, "--port") {
                None => None,
                Some(text) => match text.parse::<u16>() {
                    Ok(0) => {
                        return Command::Refused(
                            "--port 0 asks the system to pick one, and by then the pairing \
                             code is already printed with a port nobody is on. Name a \
                             number between 1 and 65535."
                                .to_owned(),
                        );
                    }
                    Ok(port) => Some(port),
                    Err(_) => {
                        return Command::Refused(format!(
                            "--port {text} is not a port. A port is a number between 1 and \
                             65535; the one Qyro uses unless told otherwise is 49517."
                        ));
                    }
                },
            };
            Command::Receive {
                out: flag(args, "--out"),
                expect: flag(args, "--expect"),
                port,
            }
        }
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
        Command::Receive { out, expect, port } => {
            flows::receive(out.as_deref(), expect.as_deref(), port, vt)
        }
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
            "2" => return flows::receive(None, None, None, vt),
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
         \x20 qyro recv --port <number>             ... on another port\n\
         \x20 qyro whoami                           this device's code\n\
         \x20 qyro find                             who else is on this network\n\
         \x20 qyro qr                               draw this device's code as a QR\n\
         \n\
         PUT THE CODE IN DOUBLE QUOTES\n\
         \x20 qyro send informe.pdf --to \"QYRO1|192.168.1.5:49517|ab12cd34\"\n\
         \n\
         \x20 The `|` in a pairing code is a pipe in PowerShell and in cmd, so\n\
         \x20 an unquoted code never reaches Qyro: the console splits the line\n\
         \x20 and complains about something that is not a command. The error\n\
         \x20 does not mention Qyro, which is why this is written here.\n\
         \x20 whoami, recv and find already print the code with its quotes --\n\
         \x20 copy the whole thing, quotes and all.\n\
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
    fn recv_acepta_otro_puerto_y_rechaza_uno_imposible() {
        // ADR-0041 §3, la parte que el codigo no tenia: «si el puerto esta
        // ocupado: se dice, no se mueve. Qyro dice que puerto esta ocupado y
        // ofrece elegir otro». No habia con que elegir otro: `recv` no tenia
        // bandera de puerto, asi que la unica salida era rendirse.
        //
        // En Windows esto no es teorico. Los rangos que reservan Hyper-V, WSL2 y
        // Docker rechazan la ligadura con WSAEACCES (10013) --
        // `netsh interface ipv4 show excludedportrange protocol=tcp` los
        // enseña-- y quien instalo Docker una vez hace dos anos se encuentra un
        // receptor que no arranca.
        let Command::Receive { port, .. } =
            parse(&["recv".to_owned(), "--port".to_owned(), "49518".to_owned()])
        else {
            panic!("recv --port no produjo una recepcion");
        };
        assert_eq!(port, Some(49518));

        // Y sin la bandera sigue siendo el puerto fijo, que es lo que compra el
        // permiso del cortafuegos una sola vez.
        let Command::Receive { port, .. } = parse(&["recv".to_owned()]) else {
            panic!("recv sin banderas dejo de ser una recepcion");
        };
        assert_eq!(port, None);
    }

    #[test]
    fn un_puerto_que_no_es_un_puerto_se_rechaza_por_su_nombre() {
        // El control. Un `--port` que se ignorara en silencio cuando no parsea
        // dejaria a la persona escuchando en 49517 mientras cree que escucha en
        // otro sitio, y el codigo que le ensena Qyro seria el bueno para un
        // puerto donde no hay nadie.
        for bad in ["cero", "70000", "-1", "0"] {
            let refusal = parse(&["recv".to_owned(), "--port".to_owned(), bad.to_owned()]);
            let Command::Refused(why) = refusal else {
                panic!("--port {bad} fue aceptado");
            };
            assert!(
                why.contains(bad),
                "el rechazo de --port {bad} no dice que valor fue: {why}"
            );
        }
        // Y el puerto 0 se rechaza con su propio motivo: ADR-0041 §3 dice que
        // el 0 es una peticion, nunca una respuesta -- el sistema elegiria uno
        // y la cadena de emparejamiento ya estaria impresa con otro.
        let Command::Refused(why) =
            parse(&["recv".to_owned(), "--port".to_owned(), "0".to_owned()])
        else {
            panic!("--port 0 fue aceptado");
        };
        assert!(
            why.contains("0 "),
            "el rechazo del puerto 0 no lo distingue de un numero ilegible: {why}"
        );
    }

    #[test]
    fn el_help_dice_que_el_codigo_va_entre_comillas() {
        // El `|` del codigo de emparejamiento es una tuberia en PowerShell y en
        // `cmd`, asi que un codigo sin comillas no llega nunca a Qyro y el error
        // que sale no menciona a Qyro. `qyro whoami`, `qyro recv` y `qyro find`
        // ya lo imprimen entrecomillado; la ayuda tiene que decir por que, o el
        // dia que alguien copie el codigo de otro sitio vuelve a romperse sin
        // saber por que.
        let help = help_text();
        assert!(
            help.contains("--to \"QYRO1|"),
            "el ejemplo de la ayuda no lleva el codigo entre comillas:\n{help}"
        );
        assert!(
            help.contains("PowerShell") && help.contains("cmd"),
            "la ayuda no dice en que consolas hace falta, asi que parece capricho:\n{help}"
        );
    }

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
