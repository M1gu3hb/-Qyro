//! The three flows: send, receive, this device.
//!
//! ADR-0042 §3 — the menu and the flags reach **these same functions**. There is
//! no second implementation for interactive use.

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

use std::io::Write as _;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};

use qyro_session::{Protection, Session, SessionState, parse_pairing};
use std::time::Duration;

use crate::term::{self, Vt};

/// The port a receiver binds, unless somebody names another.
///
/// ADR-0041 §3. **Now genuinely not re-derived.** This constant used to carry
/// the literal `49_517` under a comment saying two copies of a port number are
/// two ports the day one of them changes — while being the second copy, with a
/// third in Dart. It is the engine's number now, and
/// `qyro_net::guards::the_two_consumers_agree_on_the_port` fails if the Dart
/// side drifts from it.
pub const DEFAULT_PORT: u16 = qyro_session::QYRO_PORT;

/// Where this device's identity lives.
///
/// Beside the executable rather than in a config directory, because ADR-0042
/// and `R7` §3 say the binary is copied and run: something that writes into
/// `%APPDATA%` has installed itself, whatever it calls the act.
fn identity_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("qyro-identity.bin")
}

/// Opens the process identity, or explains why it could not.
///
/// ADR-0040: a session without an identity is refused rather than given a
/// throwaway keypair, so this runs before anything else and its failure is
/// final.
fn ensure_identity() -> Result<(), String> {
    let path = identity_path();
    // Platform on Windows (DPAPI); sandbox elsewhere, which is what ADR-0040 §7
    // ships on Android and is the honest answer on a Linux box too -- there is
    // no per-user secret store this binary can rely on without installing one.
    let protection = if cfg!(windows) {
        Protection::Platform
    } else {
        Protection::Sandbox
    };
    qyro_session::open(&path, protection).map_err(|error| {
        format!(
            "could not open this device's identity at {}: {error}",
            path.display()
        )
    })
}

/// Every address this device can be reached at, with its interface.
///
/// ADR-0041 §4: all the candidates, never a guess. Loopback is excluded because
/// a code naming it works only against oneself.
///
/// **No name is ever resolved** (ADR-0042 §9): these are literal addresses and
/// the connect side takes literal addresses.
fn local_addresses() -> Vec<(String, IpAddr)> {
    let mut found = Vec::new();
    // `std` has no interface enumeration, and `if-addrs` is already in the
    // graph through `mdns-sd` on Windows only -- so on other targets this
    // answers with what a bound socket reveals rather than pulling a
    // dependency into the portable binary for one screen.
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        // Connecting a UDP socket sends nothing; it only asks the routing table
        // which local address would be used. The destination is a documentation
        // address (RFC 5737) and deliberately **not** a public resolver: a
        // program that promises never to talk to the cloud does not write
        // somebody else's IP into its source, not even to not send to it.
        if socket.connect("192.0.2.1:9").is_ok()
            && let Ok(local) = socket.local_addr()
            && !local.ip().is_loopback()
        {
            found.push(("this machine".to_owned(), local.ip()));
        }
    }
    found
}

/// `qyro whoami` — the fingerprint and where this device can be reached.
pub fn whoami(vt: Vt) -> i32 {
    if let Err(why) = ensure_identity() {
        eprintln!("qyro: {why}");
        return 1;
    }

    let fingerprint = match qyro_session::fingerprint() {
        Ok(text) => text,
        Err(error) => {
            eprintln!("qyro: could not read this device's fingerprint: {error}");
            return 1;
        }
    };

    println!();
    println!("  THIS DEVICE");
    println!();
    // The core formats it, never this file (ADR-0035 §4, ADR-0042 §8). Two
    // devices rendering the same fingerprint differently would make reading it
    // aloud mean nothing.
    println!("  fingerprint  {}{fingerprint}{}", vt.green(), vt.reset());
    println!();

    let addresses = local_addresses();
    if addresses.is_empty() {
        println!("  no network address yet.");
        println!("  On a direct cable this can take up to a minute (APIPA).");
    } else {
        println!("  pairing code -- read this to the other device:");
        println!();
        let compact = fingerprint.replace('-', "");
        for (name, ip) in &addresses {
            println!("    [{name}]");
            println!("    QYRO1|{ip}:{DEFAULT_PORT}|{compact}");
        }
    }
    println!();
    0
}

/// `qyro send <file> --to <code>`.
pub fn send(file: &str, to: &str, expect: Option<&str>, vt: Vt) -> i32 {
    if let Err(why) = ensure_identity() {
        eprintln!("qyro: {why}");
        return 1;
    }

    let path = Path::new(file);
    if !path.is_file() {
        eprintln!("qyro: '{file}' is not a file that exists here");
        return 2;
    }

    let Some(address) = address_of(to) else {
        eprintln!(
            "qyro: '{to}' is not a pairing code and not an ip:port.\n\
             A code looks like QYRO1|192.168.1.5:{DEFAULT_PORT}|<fingerprint>.\n\
             Names are never resolved -- use the address the other device shows."
        );
        return 2;
    };

    let root = path.parent().unwrap_or(Path::new("."));
    // The name still has to be text, and the check stays even though the value
    // is no longer passed separately: a file whose name is not UTF-8 cannot be
    // put on this wire, and finding that out here gives a sentence instead of a
    // manifest error three layers down.
    if path.file_name().and_then(|name| name.to_str()).is_none() {
        eprintln!("qyro: that file's name is not text this can put on the wire");
        return 2;
    }

    println!("\n  connecting to {address} ...");
    // **The full path, not the bare name.** `open_sender` derives each file's
    // name on the wire by `strip_prefix(root)`, so a bare `p.bin` against a root
    // of `C:\folder` cannot strip and every send returned `BadArgument`.
    //
    // `qyro send` had this from the day it was written in phase 13 and it was
    // released: the command has **never** moved a byte. Nothing caught it
    // because both halves were tested apart -- `open_sender` has its own tests
    // with correct arguments, and the CLI's tests never reached a socket. It
    // took putting the two faces against each other, which is what phase 21 is
    // for. QYR-0361.
    let mut session = match Session::open_sender(address, root, &[path.to_path_buf()], None) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("\nqyro: could not connect: {error}");
            return 1;
        }
    };

    let peer = session.peer_fingerprint();
    println!("  the other device says it is:");
    println!("    {}{peer}{}", vt.green(), vt.reset());

    // `--expect` is not `--yes`. It is a decision made **before** the run, and a
    // fingerprint that does not match is a refusal, not a question (ADR-0042 §4).
    if let Some(wanted) = expect
        && !fingerprint_matches(&peer, wanted)
    {
        eprintln!(
            "\n{}qyro: REFUSED.{} You expected\n    {wanted}\n  and it is\n    {peer}",
            vt.red(),
            vt.reset()
        );
        return 3;
    }

    println!();
    match drive(&mut session) {
        Some(SessionState::Completed) => {
            println!("\n  sent.");
            0
        }
        _ => {
            eprintln!("\n  the transfer did not complete.");
            1
        }
    }
}

/// The menu's send: asks for the two values, then calls [`send`].
pub fn send_interactive(vt: Vt) -> i32 {
    let Some(file) = ask("  file to send: ") else {
        return 0;
    };
    let Some(code) = ask("  pairing code from the other device: ") else {
        return 0;
    };
    send(file.trim(), code.trim(), None, vt)
}

/// `qyro recv [--out <dir>]`.
pub fn receive(out: Option<&str>, expect: Option<&str>, vt: Vt) -> i32 {
    if let Err(why) = ensure_identity() {
        eprintln!("qyro: {why}");
        return 1;
    }

    let destination = PathBuf::from(out.unwrap_or("."));
    if let Err(error) = std::fs::create_dir_all(&destination) {
        eprintln!("qyro: cannot write to {}: {error}", destination.display());
        return 2;
    }

    // The code is shown **before** binding, which is the whole of ADR-0041: the
    // port is known in advance, so there is nothing to ask a socket.
    let _ = whoami(vt);
    println!("  waiting for the other device. Ctrl-C to stop.");

    let bind = SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT));
    let mut session = match Session::open_receiver(bind, &destination, None) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("\nqyro: could not listen on port {DEFAULT_PORT}: {error}");
            return 1;
        }
    };

    let peer = session.peer_fingerprint();
    println!("\n  someone connected. They say they are:");
    println!("    {}{peer}{}", vt.green(), vt.reset());

    // **QYR-0364: a question with no object is a formality, not a decision.**
    // This used to ask «accept from this device?» with a fingerprint and nothing
    // else — no names, no count, no sizes — while the GUI had shown the files all
    // along. ADR-0036 §1 says nothing is ever accepted on its own.
    //
    // Every name goes through `safe_terminal_name` (ADR-0047 §6), because a
    // filename is attacker-controlled text and a terminal is an interpreter: a
    // carriage return in a name rewrites the line the person is reading, at the
    // exact moment they are deciding whether to accept it.
    let offered = session.offered_files();
    if offered.is_empty() {
        println!("  they have not said what they are sending yet.");
    } else {
        let total: u64 = offered.iter().map(|(_, size)| *size).sum();
        println!(
            "\n  they want to send {} file(s), {total} bytes:",
            offered.len()
        );
        for (name, size) in &offered {
            println!(
                "    {} ({size} bytes)",
                qyro_session::safe_terminal_name(name)
            );
        }
    }

    if let Some(wanted) = expect {
        if !fingerprint_matches(&peer, wanted) {
            eprintln!(
                "\n{}qyro: REFUSED.{} You expected\n    {wanted}\n  and it is\n    {peer}",
                vt.red(),
                vt.reset()
            );
            return 3;
        }
    } else if !confirm("  accept from this device? [y/N] ") {
        // **Nothing is ever accepted on its own** (ADR-0036 §1). There is no
        // timer here that says yes out of tiredness.
        println!("  refused.");
        return 3;
    }

    println!();
    let ending = drive(&mut session);

    // **QYR-0357.** Nothing arrives without this: `finish` verifies each digest
    // and renames the `.qyro-part` to its final name. It runs on every ending,
    // not only the happy one, because a receiver that stopped early leaves a
    // part per started item and nothing else removes it.
    let materialised = session.finish().unwrap_or(0);

    match ending {
        Some(SessionState::Completed) => {
            println!(
                "\n  {materialised} file(s) saved in {}",
                destination.display()
            );
            0
        }
        _ => {
            eprintln!("\n  the transfer did not complete, and nothing was kept.");
            1
        }
    }
}

/// Steps a session to its ending, drawing the bar with `\r`.
/// Returns the ending, or `None` when the session failed on the way — the error
/// has already been printed by then, and returning it would say the same thing
/// twice in two different wordings.
fn drive(session: &mut Session) -> Option<SessionState> {
    let mut state = SessionState::InProgress;
    while state == SessionState::InProgress {
        match session.step() {
            Ok(next) => state = next,
            Err(error) => {
                eprintln!("\n  {error}");
                return None;
            }
        }
        let progress = session.progress();
        print!("{}", term::progress_bar(progress.done, progress.total));
        let _ = std::io::stdout().flush();
    }
    println!();
    Some(state)
}

/// The address inside a pairing code, or a literal `ip:port`.
///
/// Both go through the same parser the other device uses. **No name is ever
/// resolved** — `"pc-de-juan.local".parse::<SocketAddr>()` fails, and that is
/// the intended answer, not a gap (ADR-0042 §9).
fn address_of(text: &str) -> Option<SocketAddr> {
    if let Ok(address) = parse_pairing(text) {
        return address.parse::<SocketAddr>().ok();
    }
    text.parse::<SocketAddr>().ok()
}

/// Whether a fingerprint the user typed names the peer that connected.
///
/// Compared without the grouping dashes and case-insensitively, because a
/// person reading one aloud will not reproduce the punctuation and refusing
/// over a hyphen would teach them to stop checking.
fn fingerprint_matches(actual: &str, wanted: &str) -> bool {
    fn normalise(text: &str) -> String {
        text.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }
    let (actual, wanted) = (normalise(actual), normalise(wanted));
    !wanted.is_empty() && actual == wanted
}

/// Reads one line, or `None` at end of input.
fn ask(prompt: &str) -> Option<String> {
    print!("{prompt}");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line),
    }
}

/// A yes/no where **anything that is not yes is no**.
fn confirm(prompt: &str) -> bool {
    match ask(prompt) {
        Some(answer) => matches!(answer.trim(), "y" | "Y" | "yes" | "Yes"),
        None => false,
    }
}

/// `qyro find` — who is announcing themselves on this network.
///
/// **The first production caller `qyro_net::MdnsDiscovery` has ever had.** It
/// was written in phase 04b and nothing called it for three phases; ADR-0043 §5
/// says connect it rather than rewrite it, and this is the connection.
///
/// An empty list is a **true statement about this network**, not a failure:
/// routers with client isolation are the common case. The message says so and
/// points at the typed code, which is the path that works everywhere.
pub fn find(vt: Vt) -> i32 {
    if let Err(why) = ensure_identity() {
        eprintln!("qyro: {why}");
        return 1;
    }

    // **The cable assistant** (ADR-0043 §2, phase 14 §3.4). A person who plugs a
    // cable in and sees nothing for a minute concludes the cable is broken.
    // `R8` §8 measures that window: the DHCP client tries and fails before
    // APIPA assigns, and it is tens of seconds. So the wait is counted out loud
    // rather than hidden, and it ends in advice rather than an error.
    let start = std::time::Instant::now();
    let link = qyro_session::wait_for_link(
        || local_addresses().into_iter().map(|(_, ip)| ip).collect(),
        move || start.elapsed(),
        |state| match state {
            qyro_session::LinkState::Waiting { elapsed } => {
                print!(
                    "\r  waiting for a network address ... {}s -- this is normal on a direct cable      ",
                    elapsed.as_secs()
                );
                let _ = std::io::stdout().flush();
            }
            qyro_session::LinkState::Ready(address) => {
                println!("\r  address: {address}                                   ");
            }
            qyro_session::LinkState::StillNothing => {
                println!("\r  still no address after 60 seconds.                     ");
                // Advice, not an error. Auto-MDI-X is IEEE 802.3 clause 40.4.4,
                // the **1000BASE-T** clause: a 10/100-only NIC -- exactly the
                // one in the machine this was built for -- may not have it.
                println!("  If a cable joins the two machines, try a crossover cable.");
                println!("  If they share a network, the typed pairing code works anyway.");
            }
        },
        qyro_session::APIPA_BUDGET,
        std::time::Duration::from_secs(1),
    );

    if matches!(link, qyro_session::LinkState::StillNothing) {
        return 1;
    }

    println!(
        "
  looking for other devices for 3 seconds ..."
    );
    match qyro_session::browse(Duration::from_secs(3)) {
        Ok(peers) if peers.is_empty() => {
            println!(
                "
  nobody answered."
            );
            println!("  That is normal: most routers block devices from seeing");
            println!("  each other, and every public Wi-Fi does. Ask the other");
            println!("  device for its pairing code and use that -- it works");
            println!("  on every network.");
            0
        }
        Ok(peers) => {
            println!();
            for peer in &peers {
                println!("    {}{}{}", vt.green(), peer.pairing_string(), vt.reset());
            }
            println!(
                "
  copy one of those into: qyro send <file> --to <code>"
            );
            0
        }
        Err(error) => {
            // A backend this build does not have is **not** "nobody answered",
            // and saying so is the whole point: a person told "no devices found"
            // concludes the other machine is off, and goes looking in the wrong
            // place.
            eprintln!(
                "
  this build cannot look for devices on this platform: {error}"
            );
            eprintln!("  Use the pairing code the other device shows.");
            1
        }
    }
}

/// `qyro qr` — this device's pairing code, drawn for a camera.
///
/// **The direction ADR-0044 §6 fixed: the CLI draws, the phone reads.** There is
/// no scanner here and there is not going to be one — reading a code needs a
/// camera this machine does not have, and 400–700 lines of COM `unsafe` to talk
/// to one it might. The phone already has a camera and an app that can read a
/// QR, so the work goes where the hardware already is.
pub fn qr(vt: Vt) -> i32 {
    if let Err(why) = ensure_identity() {
        eprintln!("qyro: {why}");
        return 1;
    }

    let fingerprint = match qyro_session::fingerprint() {
        Ok(text) => text,
        Err(error) => {
            eprintln!("qyro: could not read this device's fingerprint: {error}");
            return 1;
        }
    };

    let addresses = local_addresses();
    let Some((_, ip)) = addresses.first() else {
        println!();
        println!("  no network address yet, so there is no code to show.");
        println!("  On a direct cable this can take up to a minute (APIPA).");
        return 1;
    };

    let compact = fingerprint.replace('-', "");
    let code = format!("QYRO1|{ip}:{DEFAULT_PORT}|{compact}");

    println!();
    println!("  Point the other device's camera at this.");
    println!("  {}{code}{}", vt.green(), vt.reset());
    println!();

    match crate::optical::draw(code.as_bytes()) {
        Ok(drawing) => {
            print!("{drawing}");
            println!();
            // **Measured from the drawing, never estimated.** The first draft
            // guessed the module count from the payload length and printed
            // "37 columns" for a code that is 41 wide. Advice that understates
            // the width is worse than none: somebody widens the terminal to
            // exactly what it said, the code still wraps, and now the tool has
            // lied to them once.
            let columns = drawing
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);
            let rows = drawing.lines().count();
            println!(
                "  If it does not scan, widen the terminal: it needs {columns} columns and {rows} rows."
            );
            if let Some(advice) = camera_advice(&drawing) {
                println!("{advice}");
            }
            0
        }
        Err(error) => {
            eprintln!("qyro: {error}.");
            1
        }
    }
}

/// Qué cámara hace falta para leer el código que se acaba de dibujar.
///
/// **El llamante de producción de la aritmética de `R10` §8 T1**, y la razón de
/// que esa aritmética viva en código y no en prosa: quien apunta el teléfono no
/// sabe que la resolución de captura decide si esto funciona, y nadie se lo iba
/// a decir.
///
/// La versión se deriva del dibujo —anchura menos la zona de silencio, y de ahí
/// los módulos— en vez de pasarse aparte: dos sitios que sepan qué versión se
/// dibujó son dos sitios que pueden discrepar.
fn camera_advice(drawing: &str) -> Option<String> {
    let columns = drawing.lines().map(|line| line.chars().count()).max()?;
    let modules = u32::try_from(columns.checked_sub(8)?).ok()?;
    if modules < 21 || (modules - 21) % 4 != 0 {
        return None;
    }
    let version = u8::try_from((modules - 21) / 4 + 1).ok()?;
    let at_720 = qyro_eye::pixels_per_module(version, 720)?;
    let at_480 = qyro_eye::pixels_per_module(version, 480)?;

    // Tres tramos, no dos. El primer intento decía «justo en el suelo» para
    // cualquier valor por encima de él, así que un código pequeño con 10
    // px/módulo salía descrito como al borde del precipicio. Un consejo que
    // asusta cuando no toca se deja de leer, y entonces no sirve el día que sí.
    let floor = qyro_eye::PIXELS_PER_MODULE_FLOOR;
    let verdict = if at_480 >= floor * 1.3 {
        format!("A 640x480 da {at_480:.1}, que tambien vale.")
    } else if at_480 >= floor {
        format!(
            "A 640x480 da {at_480:.1}: justo en el suelo del decodificador, puede leer y puede no leer."
        )
    } else {
        format!("A 640x480 da {at_480:.1}, por debajo del suelo: ahi no lee.")
    };

    Some(format!(
        "  Camara: a 1280x720 este codigo da {at_720:.1} px/modulo. {verdict}"
    ))
}

/// Comprueba que lo que esta terminal dibuja se puede volver a leer.
///
/// **El llamante de producción de `qyro_eye`, y no es una prueba disfrazada.**
/// Lo que se imprime depende de la fuente de la terminal: una que dibuje los
/// medios bloques con un píxel de separación, o que no tenga `U+2584` y sustituya
/// por un cuadro, produce un código **perfecto a la vista e ilegible para
/// cualquier lector**. Sin esto, la forma de enterarse es que alguien sostenga
/// un teléfono diez minutos delante de una pantalla que nunca iba a funcionar.
///
/// Cuesta un dibujo y una decodificación —decenas de milisegundos— una vez, al
/// principio, contra una sesión que dura minutos.
fn preflight(sample: &[u8], vt: Vt) -> bool {
    let Ok(drawing) = crate::optical::draw(sample) else {
        return false;
    };
    // 4 px/módulo: la banda fiable de `R10` §8 T1. Comprobar a una escala más
    // generosa que la real diría que sí donde la cámara dirá que no.
    let (luma, width, height) = crate::optical::rasterise(&drawing, 4);
    let mut eye = qyro_eye::Eye::new();
    if eye.look(&luma, width, height) != qyro_eye::Look::Nothing {
        return true;
    }

    eprintln!();
    eprintln!(
        "{}qyro: esta terminal dibuja un codigo que un lector no reconoce.{}",
        vt.red(),
        vt.reset()
    );
    eprintln!("  Casi siempre es la fuente: hace falta una que tenga los medios");
    eprintln!("  bloques U+2580 y U+2584 y los dibuje pegados, sin separacion.");
    eprintln!("  Prueba con otra fuente de terminal, o usa otro canal.");
    eprintln!();
    eprintln!("  Se comprueba aqui y no despues porque la otra forma de");
    eprintln!("  enterarse es apuntar un telefono diez minutos a una pantalla");
    eprintln!("  que nunca iba a funcionar.");
    false
}

/// `qyro beam <file>` — a file, as an endless stream of QR codes.
///
/// ADR-0044. **The only channel that works with no network at all**: no cable,
/// no Wi-Fi, no shared anything. A screen and a camera.
///
/// The stream never ends and that is the design, not an oversight. There are no
/// piece numbers to miss (ADR-0044 §4): the receiver collects frames until it
/// has enough, and a frame lost at 90 % costs one frame instead of the transfer.
/// Whoever is holding the phone stops it when their side says it is done.
pub fn beam(file: &str, vt: Vt) -> i32 {
    let path = Path::new(file);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("qyro: could not read '{file}': {error}");
            return 2;
        }
    };

    // ADR-0044 §5: above 20 MB this refuses **and says how long it would have
    // taken**. A channel that silently accepts a two-hour video is not generous,
    // it is a trap.
    const REFUSE_ABOVE: usize = 20 * 1024 * 1024;
    const BYTES_PER_SECOND: usize = 8 * 1024;
    if bytes.len() > REFUSE_ABOVE {
        let minutes = bytes.len() / BYTES_PER_SECOND / 60;
        eprintln!(
            "qyro: '{file}' is {} MB. Over a screen that is about {minutes} minutes,
             and a session that long fails almost every time -- a screensaver, a
             notification or thermal throttling ends it. Send it over the network,
             or split it.",
            bytes.len() / (1024 * 1024)
        );
        return 2;
    }

    // One block per QR. v27-L holds 1 465 bytes (ADR-0044 §2) and that is the
    // **ceiling, not the size**: a payload smaller than one block gets a block
    // its own size, so a 4 KB key draws a small code instead of the largest and
    // hardest-to-scan one the standard offers. The first version of this drew a
    // full v27 for a 51-byte file.
    const V27_L_CAPACITY: usize = 1465;
    let widest = V27_L_CAPACITY - qyro_fountain::FRAME_HEADER_LEN;
    let block_size = match u16::try_from(widest.min(bytes.len().max(1))) {
        Ok(size) => size,
        Err(_) => {
            eprintln!("qyro: the frame size does not fit a QR");
            return 1;
        }
    };
    let payload_len = match u32::try_from(bytes.len()) {
        Ok(len) if len > 0 => len,
        _ => {
            eprintln!("qyro: '{file}' is empty, so there is nothing to show");
            return 2;
        }
    };

    let shape = qyro_fountain::Shape {
        payload_len,
        block_size,
    };
    let blocks = qyro_fountain::split(&bytes, block_size);

    let seconds = bytes.len() / BYTES_PER_SECOND;
    println!();
    println!(
        "  {}{}{} -- {} bytes in {} blocks",
        vt.green(),
        file,
        vt.reset(),
        bytes.len(),
        blocks.len()
    );
    println!("  About {seconds}s of showing, if the camera keeps up. Ctrl-C to stop.");
    println!("  The stream never ends on purpose: the other side stops when it has enough.");
    println!();

    // El vuelo de comprobación, antes del primer frame de verdad.
    if !preflight(
        &qyro_fountain::encode_frame(&qyro_fountain::encode(&blocks, shape, 1)),
        vt,
    ) {
        return 1;
    }

    // ADR-0044 §3: five frames a second. The limit is not bandwidth, it is lost
    // frames -- screen and camera are not synchronised and anything caught
    // mid-transition is rubbish, so the screen stays well under fps/2. txqr
    // measured 6-7, Coldcard recommends 4, Sparrow ships 5.
    let frame_time = Duration::from_millis(200);
    let mut seed = 1_u64;
    loop {
        let frame = qyro_fountain::encode(&blocks, shape, seed);
        let wire = qyro_fountain::encode_frame(&frame);
        match crate::optical::draw(&wire) {
            Ok(drawing) => {
                // Home the cursor rather than clearing: a clear makes the screen
                // flash white between frames, and a camera that catches the
                // flash gets a frame of nothing.
                print!("{}{drawing}", vt.home());
                let _ = std::io::stdout().flush();
            }
            Err(why) => {
                eprintln!("qyro: {why}.");
                return 1;
            }
        }
        seed = seed.wrapping_add(1);
        std::thread::sleep(frame_time);
    }
}

/// `qyro how [file]` — which way to send it, decided by the engine.
///
/// **ADR-0046 §4: one module decides and both faces call it.** Phases 14, 15 and
/// 16 each had something to say about which path to use, and three interfaces
/// each inventing their own order is how a product ends up contradicting itself.
/// Nothing here chooses anything: it establishes the facts and prints what
/// `qyro_session::advise` returned.
pub fn how(file: Option<&str>, vt: Vt) -> i32 {
    let payload_len = file
        .and_then(|path| std::fs::metadata(path).ok())
        .map_or(1024 * 1024, |meta| meta.len());

    // Facts, established here because this is the face that can see them. The
    // decision is not made here.
    let has_network = !local_addresses().is_empty();
    let peer_discovered = qyro_session::browse(Duration::from_secs(2))
        .map(|peers| !peers.is_empty())
        .unwrap_or(false);
    let has_serial_port = serialport::available_ports()
        .map(|ports| !ports.is_empty())
        .unwrap_or(false);

    let situation = qyro_session::Situation {
        has_network,
        peer_discovered,
        has_serial_port,
        // This face cannot know whether the other machine has a camera, and
        // guessing would put an option in front of somebody that may not exist.
        // The optical channel is offered explicitly with `qyro beam`.
        other_has_camera: false,
        payload_len,
    };

    let (advice, channels) = qyro_session::advise(situation);
    println!();
    if let Some(path) = file {
        println!(
            "  {}{path}{} -- {payload_len} bytes",
            vt.green(),
            vt.reset()
        );
        println!();
    }
    for line in advice.lines() {
        println!("  {line}");
    }
    if channels.is_empty() {
        println!();
        return 1;
    }
    println!();
    println!("  If none of those work, QR codes always do: qyro beam <file>");
    println!();
    0
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

    use super::{DEFAULT_PORT, address_of, fingerprint_matches};

    #[test]
    fn a_pairing_code_and_a_bare_address_reach_the_same_place() {
        let bare = address_of("192.168.1.5:49517").expect("a literal address");
        assert_eq!(bare.port(), DEFAULT_PORT);
    }

    #[test]
    fn a_hostname_is_refused_rather_than_resolved() {
        // ADR-0042 §9, and it is a rule rather than a limitation to hide: on
        // musl there is no NSS, and on the networks this targets there is
        // nothing to resolve a name with. Failing here, next to the mistake, is
        // better than failing inside a connect.
        assert!(address_of("pc-de-juan.local:49517").is_none());
        assert!(address_of("localhost:49517").is_none());
    }

    #[test]
    fn a_fingerprint_matches_across_the_punctuation_people_drop() {
        // A person reading a fingerprint aloud will not reproduce the hyphens.
        // Refusing over one would teach them to stop checking, which is worse
        // than the hyphen.
        assert!(fingerprint_matches("ab12-cd34-ef56", "AB12CD34EF56"));
        assert!(fingerprint_matches("ab12-cd34", "ab12 cd34"));
    }

    #[test]
    fn but_a_different_fingerprint_never_matches_and_neither_does_nothing() {
        // The control. A comparison that normalises everything away would
        // satisfy the test above and accept anybody.
        assert!(!fingerprint_matches("ab12-cd34", "ab12-cd35"));
        assert!(!fingerprint_matches("ab12-cd34", ""));
        assert!(!fingerprint_matches("ab12-cd34", "----"));
    }
}
