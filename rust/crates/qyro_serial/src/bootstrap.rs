//! The receiver somebody pastes into a machine that cannot install anything.
//!
//! Specification: ADR-0045 §5.
//!
//! # What this is for
//!
//! The far machine may not be able to run Qyro at all. What it certainly has,
//! if it is Windows 7 or newer, is **PowerShell 2.0**, which exposes
//! `System.IO.Ports.SerialPort` directly — a complete receiver is about fifteen
//! lines. Windows XP has HyperTerminal instead. An old Linux has `stty` and
//! `cat`.
//!
//! So Qyro prints the receiver and the person types or pastes it. **With the
//! real values filled in**, never `<port>`: a script with a placeholder in it is
//! a script somebody has to understand before they can use it, and the whole
//! point is that they should not have to.
//!
//! # What it cannot do, said here and on screen
//!
//! Fifteen lines of PowerShell cannot do X25519 or ChaCha20-Poly1305. The
//! degraded mode is **not authenticated and not confidential** (ADR-0045 §5.1).
//! That sentence goes in front of the person before they send, not in a footnote.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Which receiver the far machine can run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// Windows 7 and newer: PowerShell 2.0 and `System.IO.Ports`.
    WindowsPowerShell,
    /// Windows XP: HyperTerminal ships with it.
    WindowsXp,
    /// An old Linux: `stty` and a redirect.
    Linux,
}

/// The warning that goes in front of the person before anything is sent.
///
/// **Not a footnote.** ADR-0045 §5.1: the degraded receiver is fifteen lines of
/// script, so there is no authentication and no confidentiality. Saying it after
/// the transfer would be telling somebody what they already cannot undo.
pub const DEGRADED_WARNING: &str = "\
This receiver is a script, so this transfer is NOT encrypted.
It is also NOT authenticated: anyone with access to the cable could read
what you send, or send something to that machine themselves.
On a one-metre cable between two of your own machines that is usually fine --
but it is your decision, and it is different from every other Qyro channel.
What you do get: each block is checked with a CRC32, and Qyro prints the
SHA-256 of the whole file so you can compare it at the other end.";

/// The receiver script or instructions, with the real values filled in.
#[must_use]
pub fn receiver_for(target: Target, port: &str, output: &str, baud: u32) -> String {
    match target {
        Target::WindowsPowerShell => windows_powershell(port, output, baud),
        Target::WindowsXp => windows_xp(port, baud),
        Target::Linux => linux(port, output, baud),
    }
}

/// PowerShell 2.0. No modules, no `Add-Type`, nothing to install.
fn windows_powershell(port: &str, output: &str, baud: u32) -> String {
    // `certutil -decode` reassembles the binary, and it has shipped with every
    // Windows since XP. The script writes Base64 lines to a temporary file and
    // decodes once at the end rather than per block: `certutil` is a process
    // launch, and one per 512 bytes would be slower than the cable.
    format!(
        "\
$p = New-Object System.IO.Ports.SerialPort '{port}',{baud},'None',8,'One'
$p.Handshake = 'RequestToSend'
$p.ReadTimeout = 5000
$p.Open()
$b64 = \"{output}.b64\"
Set-Content $b64 '' -Encoding Ascii
while ($true) {{
  try {{ $line = $p.ReadLine() }} catch {{ break }}
  if ($line -notmatch '^QS1 ') {{ continue }}
  $f = $line.Split(' ')
  Add-Content $b64 $f[4] -Encoding Ascii
  $p.WriteLine('OK ' + $f[1])
  if ([int]$f[1] -eq [int]$f[2] - 1) {{ break }}
}}
$p.Close()
certutil -decode $b64 '{output}'
certutil -hashfile '{output}' SHA256"
    )
}

/// HyperTerminal, which is the only thing XP has.
fn windows_xp(port: &str, baud: u32) -> String {
    // XP has no PowerShell, so there is no script to paste -- these are the
    // settings a person clicks. Written out because "configure HyperTerminal"
    // is not an instruction; every one of these fields has a wrong default.
    format!(
        "\
Windows XP has no PowerShell, so this is HyperTerminal instead:

  Start -> Programs -> Accessories -> Communications -> HyperTerminal
  Connect using:    {port}
  Bits per second:  {baud}
  Data bits:        8
  Parity:           None
  Stop bits:        1
  Flow control:     Hardware

  Then: Transfer -> Receive File, and choose the folder.
  Protocol:         Ymodem

Ymodem and not Zmodem: HyperTerminal has both, and Ymodem is the one whose
implementations have not spent thirty years being a security advisory."
    )
}

/// An old Linux, where `stty` and a redirect are the whole receiver.
fn linux(port: &str, output: &str, baud: u32) -> String {
    // `raw` matters more than it looks: without it the tty translates carriage
    // returns and eats control characters, and the file arrives subtly wrong.
    format!(
        "\
stty -F {port} {baud} cs8 -cstopb -parenb raw -echo crtscts
cat < {port} > {output}.b64
base64 -d {output}.b64 > {output}
sha256sum {output}"
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

    use super::{DEGRADED_WARNING, Target, receiver_for};

    #[test]
    fn every_receiver_carries_the_real_values_and_no_placeholder() {
        // **The rule ADR-0045 §5 sets and the reason it is a rule.** A script
        // with `<port>` in it is a script somebody has to understand before they
        // can use it, and the entire point of printing one is that they should
        // not have to.
        for target in [Target::WindowsPowerShell, Target::WindowsXp, Target::Linux] {
            let script = receiver_for(target, "COM3", "C:\\recibido.bin", 115_200);
            assert!(script.contains("COM3"), "{target:?} lost the port");
            assert!(script.contains("115200"), "{target:?} lost the speed");
            for placeholder in ["<port>", "<PORT>", "<file>", "<baud>", "TODO"] {
                assert!(
                    !script.contains(placeholder),
                    "{target:?} printed the placeholder {placeholder}"
                );
            }
        }
    }

    #[test]
    fn the_powershell_receiver_answers_every_block() {
        // Without the acknowledgement the sender re-offers each block until it
        // gives up, and the transfer ends in a retry storm that looks like a
        // broken cable.
        let script = receiver_for(Target::WindowsPowerShell, "COM4", "out.bin", 115_200);
        assert!(
            script.contains("WriteLine('OK '"),
            "no acknowledgement:\n{script}"
        );
        assert!(
            script.contains("QS1 "),
            "it does not filter this protocol's lines"
        );
        assert!(
            script.contains("certutil -decode"),
            "nothing reassembles the binary"
        );
        assert!(
            script.contains("certutil -hashfile"),
            "nothing lets the person compare the hash, which is the only \
             integrity check this mode has over the whole file"
        );
    }

    #[test]
    fn the_linux_receiver_puts_the_line_in_raw_mode() {
        // Without `raw` the tty translates carriage returns and eats control
        // characters, and the file arrives subtly wrong -- which is worse than
        // not arriving.
        let script = receiver_for(Target::Linux, "/dev/ttyS0", "out.bin", 115_200);
        assert!(script.contains(" raw "), "the tty was left in cooked mode");
        assert!(
            script.contains("-echo"),
            "the tty would echo back at the sender"
        );
    }

    #[test]
    fn the_warning_says_the_two_things_that_are_actually_lost() {
        // ADR-0045 §5.1. A warning that says "less secure" teaches nobody
        // anything; these two words are the ones that let a person decide.
        assert!(DEGRADED_WARNING.contains("NOT encrypted"));
        assert!(DEGRADED_WARNING.contains("NOT authenticated"));
        // And what remains, because a warning that only takes things away
        // leaves somebody thinking nothing is checked at all.
        assert!(DEGRADED_WARNING.contains("CRC32"));
        assert!(DEGRADED_WARNING.contains("SHA-256"));
    }
}
