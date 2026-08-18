//! Unicode format characters in peer-supplied paths.
//!
//! QYR-0021. `char::is_control()` covers the Unicode general category `Cc` and
//! nothing else, so every character in category `Cf` — the invisible formatting
//! and bidirectional controls — passed the filter, was stored verbatim, and
//! survived an encode/decode round trip byte for byte.
//!
//! That is not cosmetic. `U+202E RIGHT-TO-LEFT OVERRIDE` reverses the rendering
//! of everything after it, so `invoice<RLO>fdp.exe` is displayed as
//! `invoiceexe.pdf` by every bidi-aware renderer, including the file pickers and
//! terminals a receiver would confirm the transfer in. The crate's own
//! documentation, ADR-0019 and THREAT_MODEL.md all claimed an executable could
//! not be presented as a document.
//!
//! See `docs/adr/ADR-0019-manifest-display-name.md` for the rule and its source.

mod common;

use common::{RawItem, RawManifest};
use qyro_manifest::{ManifestError, PathError, RelativePath, codec};

/// Asserts a candidate is refused specifically as a format character.
fn assert_format_character(candidate: &str, expected: char) {
    assert_eq!(
        RelativePath::parse(candidate),
        Err(PathError::FormatCharacter { found: expected }),
        "U+{:04X} must be refused as a format character, not accepted or \
         mistaken for something else",
        u32::from(expected)
    );
}

#[test]
fn unicode_format_characters_are_rejected() {
    // One representative from each hazard class the category carries: the zero
    // width joiners, the bidi embeddings and overrides, the bidi isolates, the
    // byte order mark, the soft hyphen and the word joiner.
    for character in [
        '\u{200B}', // ZERO WIDTH SPACE
        '\u{200C}', // ZERO WIDTH NON-JOINER
        '\u{200D}', // ZERO WIDTH JOINER
        '\u{202A}', // LEFT-TO-RIGHT EMBEDDING
        '\u{202B}', // RIGHT-TO-LEFT EMBEDDING
        '\u{202C}', // POP DIRECTIONAL FORMATTING
        '\u{202D}', // LEFT-TO-RIGHT OVERRIDE
        '\u{202E}', // RIGHT-TO-LEFT OVERRIDE
        '\u{2066}', // LEFT-TO-RIGHT ISOLATE
        '\u{2067}', // RIGHT-TO-LEFT ISOLATE
        '\u{2068}', // FIRST STRONG ISOLATE
        '\u{2069}', // POP DIRECTIONAL ISOLATE
        '\u{FEFF}', // ZERO WIDTH NO-BREAK SPACE (byte order mark)
        '\u{00AD}', // SOFT HYPHEN
        '\u{2060}', // WORD JOINER
    ] {
        assert_format_character(&format!("file{character}.txt"), character);
        // And in a directory segment, not only in the final name.
        assert_format_character(&format!("dir{character}/file.txt"), character);
    }
}

#[test]
fn a_right_to_left_override_cannot_disguise_an_extension() {
    // Renders as `invoiceexe.pdf`. A receiver confirming this transfer would be
    // agreeing to a document and receiving an executable.
    assert_format_character("invoice\u{202E}fdp.exe", '\u{202E}');
    assert_format_character("factura\u{202E}gpj.exe", '\u{202E}');
    // The same trick with the left-to-right override, and inside a directory.
    assert_format_character("reports/2026\u{202D}fdp.exe", '\u{202D}');
}

#[test]
fn a_zero_width_space_cannot_hide_between_a_name_and_its_extension() {
    // `safe\u{200B}.txt.exe` reads as `safe.txt.exe`, but a UI that elides on a
    // zero-width boundary, or a human scanning it, sees `safe .txt`.
    assert_format_character("safe\u{200B}.txt", '\u{200B}');
    assert_format_character("report\u{200B}.pdf.exe", '\u{200B}');
}

#[test]
fn a_name_that_differs_only_by_an_invisible_character_is_rejected() {
    // `\u{FEFF}budget.xlsx` and `budget.xlsx` are different byte strings that
    // are indistinguishable on screen. Rejecting the first is what stops a
    // manifest from carrying two entries a receiver cannot tell apart.
    assert!(RelativePath::parse("budget.xlsx").is_ok());
    assert_format_character("\u{FEFF}budget.xlsx", '\u{FEFF}');
    assert_format_character("budget.xlsx\u{FEFF}", '\u{FEFF}');
}

#[test]
fn the_decoder_refuses_a_disguised_extension_too() {
    // The construction API cannot build this manifest, so the bytes are written
    // by hand: this is the path a peer actually takes.
    let bytes = RawManifest::new(vec![RawItem::file(1, "invoice\u{202E}fdp.exe", 16)]).encode();

    assert_eq!(
        codec::decode(&bytes),
        Err(ManifestError::InvalidPath {
            index: 0,
            source: PathError::FormatCharacter { found: '\u{202E}' },
        }),
        "a hostile path must not survive the decoder"
    );
}

#[test]
fn legitimate_unicode_names_are_still_accepted() {
    // Rejecting a category is only correct if it rejects that category. A rule
    // that also refuses ordinary names in other scripts is a different defect,
    // and the crate has shipped one before: an earlier folding table stripped
    // diacritics and made `año.txt` collide with `ano.txt`.
    for candidate in [
        "año.txt",
        "résumé.pdf",
        "日本語/ファイル.txt",
        "Привет.txt",
        "العربية.txt",
        "emoji-🎉.png",
        "देवनागरी.txt",
    ] {
        assert!(
            RelativePath::parse(candidate).is_ok(),
            "{candidate:?} is an ordinary name and must be accepted"
        );
    }
}
