//! Drawing that works on a console from 2006.
//!
//! Specification: ADR-0042 §7, and the measurements behind it are `R8` §11.
//!
//! # The rule, and it is a hard one
//!
//! **Everything drawn here must work with only `\r` and `\n`.** The conhost of
//! Windows 7 does not support VT sequences at all; a program that assumes them
//! prints garbage on the exact machine this whole face exists for.
//!
//! VT is an **upgrade**, detected the way Microsoft documents — `SetConsoleMode`
//! returns 0 and `GetLastError` gives `ERROR_INVALID_PARAMETER`, and the
//! documented response is to *"gracefully degrade behavior and try again"*. It
//! is never the baseline.
//!
//! # What is deliberately absent
//!
//! No box-drawing beyond ASCII, no Braille, no quadrant blocks: `R8` §11
//! verified character by character that `⠀` and `▖` **do not exist in cp437**.
//! No call to `chcp 65001` on the user's behalf — with the raster font of
//! `cmd.exe` on Windows 7 it fixes nothing and breaks the I/O.

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

use std::fmt::Write as _;

/// Whether this terminal accepts VT escape sequences.
///
/// Two states and not three: a terminal either takes them or it does not, and
/// "probably" is how a program ends up printing `←[32m` at somebody.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vt {
    /// ANSI sequences are safe.
    Enabled,
    /// Only `\r` and `\n`. The baseline, and the default when unsure.
    Absent,
}

impl Vt {
    /// Green, or nothing at all.
    ///
    /// "Nothing at all" is an acceptable answer; a binary that does not start
    /// is not.
    #[must_use]
    pub const fn green(self) -> &'static str {
        match self {
            Self::Enabled => "\u{1b}[32m",
            Self::Absent => "",
        }
    }

    /// Puts the cursor back at the top-left, without clearing.
    ///
    /// **Not a clear.** `ESC[2J` blanks the screen before the next frame is
    /// drawn, so a camera running at 30 fps against a 5 fps stream catches the
    /// white flash and reads a frame of nothing — which on this channel is
    /// indistinguishable from a frame it simply missed, except that it happens
    /// on a schedule. Homing overwrites in place and never shows an empty
    /// screen (ADR-0044 §3).
    ///
    /// Without a terminal that understands it this is empty and each frame
    /// scrolls. Ugly, and it still works: the newest code is the one at the
    /// bottom.
    #[must_use]
    pub const fn home(self) -> &'static str {
        match self {
            Self::Enabled => "\u{1b}[H",
            Self::Absent => "",
        }
    }

    /// Red, for the one thing that must never be missed.
    #[must_use]
    pub const fn red(self) -> &'static str {
        match self {
            Self::Enabled => "\u{1b}[31m",
            Self::Absent => "",
        }
    }

    /// Back to normal.
    #[must_use]
    pub const fn reset(self) -> &'static str {
        match self {
            Self::Enabled => "\u{1b}[0m",
            Self::Absent => "",
        }
    }
}

/// The width every panel is drawn to.
///
/// Fixed at 64 rather than queried: an 80-column console is the floor on every
/// machine this targets, and a layout that reflows is a layout that can break
/// on the one terminal nobody can test.
const WIDTH: usize = 64;

/// The banner and the menu.
///
/// ADR-0042 §6: English only in the v1.x, and the last line **says so** rather
/// than leaving a Spanish speaker to wonder whether something is broken.
#[must_use]
pub fn menu(version: &str, vt: Vt) -> String {
    let mut out = String::new();
    let bar = "=".repeat(WIDTH);

    let _ = writeln!(out, "{bar}");
    let _ = writeln!(out, "  QYRO {version}");
    let _ = writeln!(out, "  direct file transfer -- no cloud, no accounts");
    let _ = writeln!(out, "{bar}");
    let _ = writeln!(out);
    let _ = writeln!(out, "  1) Send a file");
    let _ = writeln!(out, "  2) Receive a file");
    let _ = writeln!(out, "  3) This device");
    let _ = writeln!(out, "  4) Look for devices on this network");
    let _ = writeln!(out, "  q) Quit");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {}(the graphical app is in English and Spanish; this one is{}",
        vt.green(),
        vt.reset()
    );
    let _ = writeln!(out, "  {}English only for now){}", vt.green(), vt.reset());
    let _ = writeln!(out);
    out
}

/// A progress bar redrawn in place with `\r`.
///
/// `\r` and not a cursor-movement sequence, which is the whole point: this is
/// the one animation that works on every console ever shipped, including the
/// XP that `R8` §5 says receives files through HyperTerminal.
///
/// The caller writes it followed by nothing; the next call overwrites it.
#[must_use]
pub fn progress_bar(done: u64, total: u64) -> String {
    const CELLS: usize = 32;

    // A total of zero is a real state -- a manifest arrives before its bytes do
    // -- and dividing by it would be the first panic in a binary whose whole
    // promise is that it starts.
    let filled = if total == 0 {
        0
    } else {
        let ratio = (done as u128 * CELLS as u128) / total as u128;
        usize::try_from(ratio).unwrap_or(CELLS).min(CELLS)
    };

    let percent = if total == 0 {
        0
    } else {
        let value = (done as u128 * 100) / total as u128;
        u8::try_from(value).unwrap_or(100).min(100)
    };

    let mut bar = String::with_capacity(CELLS + 24);
    bar.push('\r');
    bar.push('[');
    for cell in 0..CELLS {
        bar.push(if cell < filled { '#' } else { '.' });
    }
    let _ = write!(bar, "] {percent:>3}%  {done} / {total} bytes");
    bar
}

/// Detects VT support, degrading rather than failing.
///
/// On Windows this is `SetConsoleMode` with
/// `ENABLE_VIRTUAL_TERMINAL_PROCESSING` (0x0004), available since Windows 10
/// v1607; anything older returns 0 and the documented response is to degrade.
///
/// **This crate forbids `unsafe`**, so the Win32 call cannot live here. Until a
/// place for it exists, the honest answer on Windows is [`Vt::Absent`]: the
/// baseline renders correctly everywhere, and claiming VT where there is none
/// prints escape codes at somebody.
#[must_use]
pub fn detect_vt() -> Vt {
    decide_vt(cfg!(windows), |name| std::env::var(name).ok())
}

/// The decision, with the world passed in so it can be tested.
///
/// **QYR-0385, and the cost of the pessimism was bigger than «colour».** The
/// comment here used to weigh «a broken screen on Windows 7» against «a
/// cosmetic loss of colour on Windows 11», and picked correctly for that
/// trade — but the trade had a third term nobody had written down: **`qyro beam`
/// homes the cursor between frames**, and `Vt::Absent` makes `home()` the empty
/// string. So on Windows every frame was **appended**: a 67-row QR scrolling
/// five times a second, which is not a degraded optical channel, it is none at
/// all. On the one platform that draws them.
///
/// # `WT_SESSION`, and why this is not the guess the old comment feared
///
/// Windows Terminal sets `WT_SESSION` in every session it hosts, and Windows
/// Terminal enables VT processing. It is a **specific documented marker of a
/// specific program**, not an inference from a version number or a heuristic
/// over `TERM`. Its absence still means `Absent`, so `conhost` — the old console
/// this face was built for — keeps the safe answer it had.
///
/// Over-claiming remains the worse mistake and this cannot over-claim: nothing
/// but Windows Terminal sets that variable.
#[must_use]
pub fn decide_vt(windows: bool, env: impl Fn(&str) -> Option<String>) -> Vt {
    if windows {
        return match env("WT_SESSION") {
            Some(session) if !session.is_empty() => Vt::Enabled,
            _ => Vt::Absent,
        };
    }
    // Unix consoles have accepted these since the seventies. `TERM=dumb` is the
    // one that says otherwise, and it says it explicitly.
    match env("TERM") {
        Some(term) if term == "dumb" => Vt::Absent,
        Some(_) => Vt::Enabled,
        None => Vt::Absent,
    }
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

    use super::{Vt, decide_vt, menu, progress_bar};

    #[test]
    fn en_windows_solo_windows_terminal_promete_vt() {
        // QYR-0385. `qyro beam` coloca el cursor arriba entre frames, y con
        // `Vt::Absent` esa secuencia es la cadena vacia: cada frame se ANADE, y
        // un QR de 67 filas se va scrolleando cinco veces por segundo. En la
        // unica plataforma que dibuja.
        //
        // `WT_SESSION` lo pone Windows Terminal en cada sesion que hospeda, y
        // Windows Terminal tiene VT. Es la marca de un programa concreto, no una
        // inferencia sobre una version.
        let con_terminal = |name: &str| (name == "WT_SESSION").then(|| "abc-123".to_owned());
        assert_eq!(decide_vt(true, con_terminal), Vt::Enabled);

        // Y el control, que es la mitad que el comentario viejo defendia con
        // razon: sin esa marca, la respuesta segura sigue siendo la de antes.
        // `conhost` -- la consola para la que se escribio esta cara -- no la pone.
        assert_eq!(decide_vt(true, |_| None), Vt::Absent);
        // Vacia es no puesta. Una variable a "" es lo que deja un `set WT_SESSION=`.
        assert_eq!(
            decide_vt(true, |name| (name == "WT_SESSION").then(String::new)),
            Vt::Absent
        );
    }

    #[test]
    fn fuera_de_windows_manda_term_y_dumb_sigue_siendo_dumb() {
        // El control del control: cambiar la rama de Windows no puede haber
        // tocado la otra.
        assert_eq!(
            decide_vt(false, |name| (name == "TERM").then(|| "xterm".to_owned())),
            Vt::Enabled
        );
        assert_eq!(
            decide_vt(false, |name| (name == "TERM").then(|| "dumb".to_owned())),
            Vt::Absent
        );
        assert_eq!(decide_vt(false, |_| None), Vt::Absent);
        // Y `WT_SESSION` fuera de Windows no promete nada: la variable puede
        // sobrevivir a un `ssh` desde Windows Terminal a una maquina Unix.
        assert_eq!(
            decide_vt(false, |name| (name == "WT_SESSION").then(|| "x".to_owned())),
            Vt::Absent
        );
    }

    /// The escape byte. If any of these appear without VT, a Windows 7 console
    /// shows them as text.
    const ESC: char = '\u{1b}';

    #[test]
    fn nothing_drawn_without_vt_contains_an_escape_byte() {
        let screen = menu("1.1.0", Vt::Absent);
        assert!(
            !screen.contains(ESC),
            "the menu emitted an escape sequence with VT absent, which is what \
             prints garbage on the console this face exists for"
        );
        for (done, total) in [(0, 0), (0, 100), (50, 100), (100, 100), (7, 3)] {
            let bar = progress_bar(done, total);
            assert!(!bar.contains(ESC), "the bar escaped at {done}/{total}");
        }
    }

    #[test]
    fn and_the_same_screen_does_contain_one_with_vt() {
        // **The control, and without it the test above proves nothing.** A
        // renderer that emitted no colour in either mode would satisfy the
        // first assertion perfectly.
        let screen = menu("1.1.0", Vt::Enabled);
        assert!(
            screen.contains(ESC),
            "with VT enabled nothing was coloured, so the first test cannot \
             tell a degraded render from an empty one"
        );
    }

    #[test]
    fn the_bar_redraws_in_place_and_never_divides_by_zero() {
        // A total of zero is a real state: the manifest arrives before the
        // bytes. This is the first place a naive percentage panics, and a
        // binary whose whole promise is "it starts" may not panic on arithmetic.
        let bar = progress_bar(0, 0);
        assert!(bar.starts_with('\r'), "the bar must redraw in place");
        assert!(bar.contains("  0%"));

        // And a done greater than total -- which a buggy peer can cause -- is
        // clamped rather than overflowing the bar or the percentage.
        let over = progress_bar(7, 3);
        assert!(over.contains("100%"), "{over}");
        assert_eq!(
            over.matches('#').count(),
            32,
            "the bar overflowed its own width"
        );
    }

    #[test]
    fn the_menu_says_which_languages_it_has() {
        // ADR-0042 §6. English-only is a decision, and a decision a person
        // cannot see is indistinguishable from a bug.
        let screen = menu("1.1.0", Vt::Absent);
        assert!(screen.contains("English only"), "{screen}");
        assert!(screen.contains("Spanish"), "{screen}");
    }
}
