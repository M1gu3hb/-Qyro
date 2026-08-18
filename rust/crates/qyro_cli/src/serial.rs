//! `qyro serial` — a file into a machine that cannot read a QR.
//!
//! Specification: ADR-0045.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::time::Duration;

use crate::term::Vt;

/// ADR-0045 §6. The last speed a 16550 UART of that era holds without framing
/// errors — and the machine at the other end is precisely of that era.
pub const BAUD: u32 = 115_200;

/// The boring question, asked before the slow channel is offered.
///
/// **ADR-0045 §2, and it goes in the interface rather than a README.** A CD-R
/// moves 700 MB in five minutes and a network cable moves 1 MB in under a
/// second; serial moves 1 MB in 1.6 minutes. Offering the slow channel without
/// ruling out the fast ones is bad product, however well the slow one works.
const FASTER_FIRST: &str = "\
  Before using this: does that machine have any of these?

    a CD or DVD burner ....... 700 MB in about five minutes
    a floppy drive ........... 1.44 MB, and it still beats serial for small files
    a PCMCIA or ExpressCard slot
    an ethernet port ......... 1 MB in under a second -- use `qyro find` instead

  Any of them is between 10 and 10 000 times faster than a serial cable.
  Serial is the answer when the others are gone, not before.";

/// `qyro serial` — lists the ports and prints the receiver for one.
pub fn overview(port: Option<&str>, vt: Vt) -> i32 {
    println!();
    println!("  THE SERIAL CHANNEL");
    println!();
    println!("{FASTER_FIRST}");
    println!();

    let ports = match serialport::available_ports() {
        Ok(ports) => ports,
        Err(error) => {
            eprintln!("qyro: could not ask this machine for its serial ports: {error}");
            return 1;
        }
    };

    if ports.is_empty() {
        println!("  This machine has no serial ports that the system reports.");
        println!("  A USB-to-serial adapter shows up here once its driver is installed.");
        return 1;
    }

    println!("  Ports on THIS machine:");
    for found in &ports {
        println!("    {}{}{}", vt.green(), found.port_name, vt.reset());
    }
    println!();

    let Some(port) = port else {
        println!("  Then, for the receiver script:");
        println!("    qyro serial --port {}", first_port_name(&ports));
        return 0;
    };

    // The warning goes in front of the person **before** anything is sent, not
    // after (ADR-0045 §5.1). Telling somebody afterwards is telling them what
    // they can no longer undo.
    println!("  {}READ THIS FIRST{}", vt.red(), vt.reset());
    println!();
    for line in qyro_serial::DEGRADED_WARNING.lines() {
        println!("  {line}");
    }
    println!();
    println!("  Paste this into the OTHER machine (Windows 7 or newer):");
    println!();
    println!(
        "{}",
        qyro_serial::receiver_for(
            qyro_serial::Target::WindowsPowerShell,
            port,
            "qyro-received.bin",
            BAUD
        )
    );
    println!();
    println!("  If that machine is Windows XP:");
    println!();
    println!(
        "{}",
        qyro_serial::receiver_for(qyro_serial::Target::WindowsXp, port, "", BAUD)
    );
    println!();
    println!("  If it is an old Linux:");
    println!();
    println!(
        "{}",
        qyro_serial::receiver_for(
            qyro_serial::Target::Linux,
            "/dev/ttyS0",
            "qyro-received.bin",
            BAUD
        )
    );
    println!();
    0
}

fn first_port_name(ports: &[serialport::SerialPortInfo]) -> String {
    ports
        .first()
        .map_or_else(|| "COM1".to_owned(), |found| found.port_name.clone())
}

/// `qyro send <file> --serial <port>`.
pub fn send(file: &str, port: &str, vt: Vt) -> i32 {
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("qyro: could not read '{file}': {error}");
            return 2;
        }
    };

    let handle = match serialport::new(port, BAUD)
        .timeout(Duration::from_secs(5))
        .flow_control(serialport::FlowControl::Hardware)
        .open()
    {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!(
                "qyro: could not open {port}: {error}\n\
                 Run `qyro serial` to see the ports this machine reports."
            );
            return 1;
        }
    };

    println!();
    for line in qyro_serial::DEGRADED_WARNING.lines() {
        println!("  {line}");
    }
    println!();
    println!("  Sending {} bytes over {port} at {BAUD} ...", bytes.len());

    let mut writer = handle;
    let mut reader = match writer.try_clone() {
        Ok(clone) => BufReader::new(clone),
        Err(error) => {
            eprintln!("qyro: could not read and write the same port: {error}");
            return 1;
        }
    };

    let outcome = qyro_serial::send_all(&bytes, |line| {
        writeln!(writer, "{line}").map_err(|_| qyro_serial::SerialError::Wire)?;
        writer.flush().map_err(|_| qyro_serial::SerialError::Wire)?;
        let mut answer = String::new();
        // A read that times out is silence, not a failure: the far end may be a
        // script that is still starting. `send_all` treats silence as another
        // attempt and stops after MAX_ATTEMPTS.
        match reader.read_line(&mut answer) {
            Ok(0) | Err(_) => Ok(None),
            Ok(_) => Ok(qyro_serial::Reply::of(&answer)),
        }
    });

    match outcome {
        Ok(tally) => {
            println!();
            println!("  sent {} blocks, {} re-sent.", tally.blocks, tally.retries);
            if tally.retries > 0 {
                // A number somebody can act on. This is the reason ADR-0045 §4
                // chose ARQ over the fountain: a retry is observable and an
                // overhead percentage is not.
                println!(
                    "  {}Re-sends mean the line is noisy{} -- a shorter cable, or a \
                     lower speed, will make it quieter.",
                    vt.red(),
                    vt.reset()
                );
            }
            println!("  Compare the SHA-256 the other machine printed against yours.");
            0
        }
        Err(error) => {
            eprintln!("\nqyro: {error}");
            1
        }
    }
}

/// `qyro recv --serial <port>` — for when both machines can run Qyro.
pub fn receive(port: &str, out: &str) -> i32 {
    let handle = match serialport::new(port, BAUD)
        .timeout(Duration::from_secs(5))
        .flow_control(serialport::FlowControl::Hardware)
        .open()
    {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("qyro: could not open {port}: {error}");
            return 1;
        }
    };

    let mut writer = match handle.try_clone() {
        Ok(clone) => clone,
        Err(error) => {
            eprintln!("qyro: could not read and write the same port: {error}");
            return 1;
        }
    };
    let mut reader = BufReader::new(handle);

    println!();
    println!("  Listening on {port} at {BAUD}. Ctrl-C to stop.");

    // Bounded, because a receiver that waited forever on an unplugged cable is
    // a hang and a hang is the failure nobody can diagnose. 200 000 lines is
    // about 100 MB of blocks, well past what this channel is for.
    let outcome = qyro_serial::receive_all(
        |reply| {
            if let Some(reply) = reply {
                writeln!(writer, "{}", reply.line()).map_err(|_| qyro_serial::SerialError::Wire)?;
                writer.flush().map_err(|_| qyro_serial::SerialError::Wire)?;
            }
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => Ok(None),
                Ok(_) => Ok(Some(line)),
            }
        },
        200_000,
    );

    match outcome {
        Ok(bytes) => match std::fs::write(out, &bytes) {
            Ok(()) => {
                println!("  received {} bytes into {out}.", bytes.len());
                0
            }
            Err(error) => {
                eprintln!("qyro: could not write {out}: {error}");
                1
            }
        },
        Err(error) => {
            eprintln!("\nqyro: {error}");
            1
        }
    }
}
