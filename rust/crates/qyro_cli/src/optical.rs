//! Drawing a QR code in a terminal, at a size a phone can actually read.
//!
//! Specification: ADR-0044 §2 and §6 — **the CLI draws, the phone reads**.
//!
//! # Half blocks, and why not `██`
//!
//! A terminal cell is about twice as tall as it is wide, so a QR drawn one cell
//! per module comes out stretched 2:1 and decoders reject it. The two usual
//! answers are two spaces per module — which doubles the width and puts a v27
//! code at 250 columns, wider than most terminals — or the half-block
//! `U+2584 LOWER HALF BLOCK`, which packs **two module rows into one cell** and
//! comes out square.
//!
//! Half blocks win, and they cost nothing: 125 modules of a v27 code become 125
//! columns by 63 rows, which fits a maximised terminal.
//!
//! # Inverted on purpose
//!
//! A QR needs **dark modules on a light field**. A terminal is usually light on
//! dark, so drawing dark modules as filled characters produces a photographic
//! negative that most decoders refuse. So this draws the *quiet* modules filled
//! and the dark ones blank — a code that looks inverted on screen and reads
//! correctly through a camera. Getting this backwards produces something that
//! looks perfect and never scans, which is the worst kind of wrong.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Modules of light margin around the code.
///
/// The QR specification requires four, and this is the field that gets dropped
/// first by somebody trying to fit a code on screen. Dropping it is why a code
/// that looks fine does not scan: the decoder needs the margin to find the
/// finder patterns at all.
pub const QUIET_ZONE: usize = 4;

/// A square grid of modules: `true` is a dark module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Modules {
    size: usize,
    dark: Vec<bool>,
}

impl Modules {
    /// Wraps a row-major grid, refusing anything that is not square.
    #[must_use]
    pub fn new(size: usize, dark: Vec<bool>) -> Option<Self> {
        (size > 0 && dark.len() == size * size).then_some(Self { size, dark })
    }

    /// Modules per side, without the quiet zone.
    #[must_use]
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Whether the module at `(x, y)` is dark, **counting the quiet zone**.
    ///
    /// Anything outside the code is light, which is what makes the margin fall
    /// out of the same lookup instead of being a second code path.
    #[must_use]
    pub fn is_dark(&self, x: isize, y: isize) -> bool {
        let Ok(size) = isize::try_from(self.size) else {
            return false;
        };
        if x < 0 || y < 0 || x >= size || y >= size {
            return false;
        }
        usize::try_from(y)
            .ok()
            .zip(usize::try_from(x).ok())
            .and_then(|(row, column)| self.dark.get(row * self.size + column))
            .copied()
            .unwrap_or(false)
    }
}

/// The four characters a half-block renderer needs.
///
/// Named rather than inlined because getting one of them wrong produces a code
/// that is subtly unreadable, and a subtly unreadable code is indistinguishable
/// from a camera that is out of focus.
const FULL: char = '\u{2588}'; // both halves light
const UPPER: char = '\u{2580}'; // top half light
const LOWER: char = '\u{2584}'; // bottom half light
const EMPTY: char = ' '; // both halves dark

/// Draws the code as text, two module rows per line.
///
/// Inverted deliberately: see the module docs. A dark module is drawn as *no*
/// ink so that the surrounding light field is what the terminal fills, which is
/// what a camera needs to see.
#[must_use]
pub fn render(modules: &Modules) -> String {
    let span = modules.size() + QUIET_ZONE * 2;
    let Ok(margin) = isize::try_from(QUIET_ZONE) else {
        return String::new();
    };
    let Ok(width) = isize::try_from(span) else {
        return String::new();
    };

    let mut out = String::with_capacity(span * span / 2 + span);
    let mut row = 0_isize;
    while row < width {
        for column in 0..width {
            let top = modules.is_dark(column - margin, row - margin);
            let bottom = modules.is_dark(column - margin, row + 1 - margin);
            out.push(match (top, bottom) {
                (false, false) => FULL,
                (false, true) => UPPER,
                (true, false) => LOWER,
                (true, true) => EMPTY,
            });
        }
        out.push('\n');
        row += 2;
    }
    out
}

/// Encodes `payload` and draws it, at the lowest version that fits.
///
/// **Error correction stays at L** (ADR-0044 §2). A screen is not paper: the
/// QR's error correction protects against dirt and creases that do not exist
/// here, and what actually goes missing is whole frames — which the fountain
/// code handles and the EC level does not. Anything above L spends capacity on
/// a problem this channel does not have.
///
/// # Errors
///
/// A sentence, not a code. **There was a two-variant enum here** and it was
/// deleted: its only consumer printed it with `{:?}`, so `TooLong` reached a
/// person as the word "TooLong". The workspace's variant guard asked for a
/// construction-site check and the honest answer was that the enum did not
/// earn its keep — a failure the caller cannot branch on is a message, and a
/// message should be written for the person reading it.
pub fn draw(payload: &[u8]) -> Result<String, &'static str> {
    use qrcode::{EcLevel, QrCode};

    let code = QrCode::with_error_correction_level(payload, EcLevel::L)
        .map_err(|_| "that is more data than any QR code can hold")?;
    let width = code.width();
    let dark: Vec<bool> = code
        .to_colors()
        .into_iter()
        .map(|colour| colour == qrcode::Color::Dark)
        .collect();

    let modules =
        Modules::new(width, dark).ok_or("the QR encoder produced a grid that is not square")?;
    Ok(render(&modules))
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

    use super::{EMPTY, FULL, Modules, QUIET_ZONE, render};

    /// How many terminal rows and columns a code of `module_count` needs.
    ///
    /// **A test helper and not production code**, deliberately. It was a `pub
    /// fn` until `qyro qr` stopped needing it: the flow now measures the string
    /// it just drew instead of predicting it, which is exact where this is
    /// arithmetic that can drift from the renderer. Kept here because the
    /// assertion below is about ADR-0044's choice of version 27, and that is
    /// worth checking; shipping it would have been one more thing with no
    /// caller.
    const fn footprint(module_count: usize) -> (usize, usize) {
        let span = module_count + QUIET_ZONE * 2;
        (span, span.div_ceil(2))
    }

    fn checker(size: usize) -> Modules {
        Modules::new(size, (0..size * size).map(|i| i % 2 == 0).collect()).expect("a square grid")
    }

    #[test]
    fn the_quiet_zone_is_there_and_it_is_four_modules() {
        // The margin the QR specification requires, and the first thing somebody
        // drops to make a code fit. Dropping it is why a code that looks fine
        // does not scan: without it the decoder cannot find the finder patterns.
        let modules = checker(21);
        let drawn = render(&modules);
        let lines: Vec<&str> = drawn.lines().collect();

        // Two module rows per line, so the quiet zone is two lines top and
        // bottom, and they are entirely light.
        assert!(
            lines
                .first()
                .is_some_and(|line| line.chars().all(|c| c == FULL)),
            "the top margin has ink in it"
        );
        assert!(
            lines
                .get(1)
                .is_some_and(|line| line.chars().all(|c| c == FULL))
        );
        assert!(
            lines
                .last()
                .is_some_and(|line| line.chars().all(|c| c == FULL)),
            "the bottom margin has ink in it"
        );
        assert!(
            lines
                .first()
                .is_some_and(|line| line.chars().count() == 21 + QUIET_ZONE * 2)
        );
    }

    #[test]
    fn a_dark_module_is_drawn_as_no_ink_and_not_the_other_way_round() {
        // **The mistake that produces a beautiful code nobody can scan.** A QR
        // is dark-on-light; a terminal is light-on-dark. Drawing dark modules
        // as filled characters makes a photographic negative, and most decoders
        // refuse those.
        let all_dark = Modules::new(2, vec![true; 4]).expect("a square grid");
        let drawn = render(&all_dark);
        assert!(
            drawn.contains(EMPTY),
            "an all-dark code drew ink everywhere, which is a photographic \
             negative and will not scan:\n{drawn}"
        );

        // And the control, which is the half that makes the assertion mean
        // something: an all-*light* code must contain no blank at all. Without
        // it, a renderer that emitted blanks unconditionally would pass above.
        let all_light = Modules::new(2, vec![false; 4]).expect("a square grid");
        let drawn = render(&all_light);
        assert!(
            !drawn.contains(EMPTY),
            "an all-light code left gaps in the field the camera needs:\n{drawn}"
        );
        assert!(drawn.lines().all(|line| line.chars().all(|c| c == FULL)));
    }

    #[test]
    fn two_module_rows_share_one_line_or_the_code_comes_out_stretched() {
        // A terminal cell is about twice as tall as it is wide. One cell per
        // module gives a 2:1 code and decoders reject it.
        let modules = checker(25);
        let lines = render(&modules).lines().count();
        let span = 25 + QUIET_ZONE * 2;
        assert_eq!(lines, span.div_ceil(2));
        assert_eq!(footprint(25), (span, lines));
    }

    #[test]
    fn a_version_27_code_fits_a_terminal_somebody_actually_has() {
        // ADR-0044 §2 fixes v27: 125 modules. If this needed more rows than a
        // maximised terminal has, the decision would be wrong and this is where
        // it would show.
        let (columns, rows) = footprint(125);
        assert_eq!((columns, rows), (133, 67));
        assert!(columns <= 200 && rows <= 70, "{columns}x{rows}");
    }

    #[test]
    fn a_grid_that_is_not_square_is_refused_rather_than_drawn_wrong() {
        // A non-square grid drawn anyway produces a shifted code: every row
        // after the first is offset, and the result looks like a QR and decodes
        // as nothing.
        assert!(Modules::new(5, vec![false; 24]).is_none());
        assert!(Modules::new(0, Vec::new()).is_none());
        assert!(Modules::new(5, vec![false; 25]).is_some());
    }
}
