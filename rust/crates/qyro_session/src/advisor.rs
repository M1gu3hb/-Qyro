//! Which way to send it, decided once for both faces.
//!
//! Specification: ADR-0046 §4. The speeds are `R8` §4 and §5.1.
//!
//! # Why this is one module and not two screens
//!
//! Phases 14, 15 and 16 each tell somebody something different about which path
//! to use. Left alone that is **three contradictory interfaces waiting to
//! exist** — and the two faces would contradict each other as well, because a
//! terminal and a phone would each grow their own version.
//!
//! It lives in `qyro_session` because that is the only thing both consumers
//! reach: the GUI through the FFI, the CLI directly. In the CLI it would be
//! invisible to the GUI; in Dart it would be invisible to the CLI; written twice
//! it *is* the problem.
//!
//! # It returns sentences, not codes
//!
//! ADR-0046 §5. Advice that arrives as an enum becomes «channel 3» in one face
//! and a paragraph in the other, and those are two products.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// What this machine can actually do right now.
///
/// Facts, not preferences — the caller establishes them and the advisor decides.
/// Splitting it this way is what lets the decision be tested without a network,
/// a cable or a camera.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Situation {
    /// A usable address exists on some interface.
    pub has_network: bool,
    /// Another Qyro answered a browse.
    pub peer_discovered: bool,
    /// This machine reports at least one serial port.
    pub has_serial_port: bool,
    /// The other machine can point a camera at this screen.
    pub other_has_camera: bool,
    /// Bytes to move.
    pub payload_len: u64,
}

/// A way to move the file, in the order ADR-0046 §4 fixed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Channel {
    /// A shared network. `R8` §4: about 10 MB/s.
    Network,
    /// A cable straight between the two machines, after APIPA.
    DirectCable,
    /// A DB9 and 115 200 8N1. `R8` §5.1: 9–11 KB/s.
    Serial,
    /// QR codes on a screen. `R8` §4: about 8 KB/s.
    Optical,
}

impl Channel {
    /// Bytes per second, from `R8`. **Deliberately pessimistic** where the
    /// measurement is a range: an estimate that runs under is a promise broken
    /// at minute nine.
    #[must_use]
    pub const fn bytes_per_second(self) -> u64 {
        match self {
            // `R8` §4. Not the wire speed: what a transfer actually sustains on
            // the hardware this product is for.
            Self::Network | Self::DirectCable => 10 * 1024 * 1024,
            // `R8` §5.1 measured 9-11 KB/s. The low end.
            Self::Serial => 9 * 1024,
            // `R8` §4.
            Self::Optical => 8 * 1024,
        }
    }

    /// The name a person reads.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Network => "the network you are both on",
            Self::DirectCable => "a cable between the two machines",
            Self::Serial => "a serial cable",
            Self::Optical => "QR codes on this screen",
        }
    }

    /// The command that starts it, so the advice is actionable.
    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            Self::Network | Self::DirectCable => "qyro find",
            Self::Serial => "qyro serial",
            Self::Optical => "qyro beam <file>",
        }
    }
}

/// How long `payload_len` takes on `channel`, in whole seconds.
#[must_use]
pub const fn seconds_for(channel: Channel, payload_len: u64) -> u64 {
    let rate = channel.bytes_per_second();
    if rate == 0 {
        0
    } else {
        payload_len.div_ceil(rate)
    }
}

/// A duration in the words somebody uses.
///
/// «2 minutes» and not «127 s», because the number exists to be *decided on*
/// rather than to be precise.
#[must_use]
pub fn plain_duration(seconds: u64) -> String {
    match seconds {
        0..=1 => "less than a second".to_owned(),
        2..=90 => format!("about {seconds} seconds"),
        91..=5400 => format!("about {} minutes", seconds.div_ceil(60)),
        _ => format!("about {} hours", seconds.div_ceil(3600)),
    }
}

/// The question that comes before any slow channel is offered.
///
/// **`FASE-16` §2, and it is not optional.** A CD-R moves 700 MB in five
/// minutes; a network cable moves 1 MB in under a second. Offering a channel
/// that takes seventeen minutes without ruling those out is bad product, however
/// well the slow one works.
pub const BORING_FIRST: &str = "\
Before using a slow channel, check the other machine for any of these:
  a CD or DVD burner, a floppy drive, a PCMCIA slot, or an ethernet port.
Any of them is between 10 and 10 000 times faster than what follows.";

/// The channels that would work here, best first.
#[must_use]
pub fn channels_for(situation: Situation) -> Vec<Channel> {
    let mut usable = Vec::new();
    if situation.has_network && situation.peer_discovered {
        usable.push(Channel::Network);
    }
    if situation.has_network && !situation.peer_discovered {
        // An address but nobody answering is exactly what a direct cable looks
        // like: link-local addresses on both ends and no responder in between.
        usable.push(Channel::DirectCable);
    }
    if situation.has_serial_port {
        usable.push(Channel::Serial);
    }
    if situation.other_has_camera {
        usable.push(Channel::Optical);
    }
    usable
}

/// The whole advice, as the sentence both faces show.
///
/// Returns the text and the channels behind it, so an interface can draw
/// buttons without re-deciding anything.
#[must_use]
pub fn advise(situation: Situation) -> (String, Vec<Channel>) {
    let channels = channels_for(situation);
    if channels.is_empty() {
        return (
            "There is no way to reach that machine yet.\n\
             Plug in a network cable, or connect a serial cable, or point its \
             camera at this screen -- Qyro needs one of those three."
                .to_owned(),
            channels,
        );
    }

    let mut text = String::new();
    // The boring question goes **before** the list, and only when the best
    // available option is already a slow one. Asking it when the network works
    // would be noise.
    if channels
        .first()
        .is_some_and(|best| *best >= Channel::Serial)
    {
        text.push_str(BORING_FIRST);
        text.push_str("\n\n");
    }

    text.push_str("Ways to send this, best first:\n");
    for channel in &channels {
        let seconds = seconds_for(*channel, situation.payload_len);
        text.push_str(&format!(
            "  {} -- {} ({})\n",
            channel.name(),
            plain_duration(seconds),
            channel.command()
        ));
    }
    (text, channels)
}

/// A name that cannot rewrite the line it is drawn in.
///
/// Specification: ADR-0047 §6.
///
/// # What a hostile name does to a terminal
///
/// A filename is attacker-controlled text and a terminal is an interpreter. A
/// name containing a carriage return **rewrites the line the person is
/// reading**; one containing an escape sequence can move the cursor, change
/// colours, or clear the screen. The receiver is about to be asked whether to
/// accept this file, so the one moment the name is drawn is the one moment it
/// must not lie.
///
/// # The rule, and it is one rule
///
/// Every C0 and C1 control (`U+0000`–`U+001F`, `U+007F`–`U+009F`) becomes
/// `U+FFFD`. **Substituted, never deleted**, for the reason ADR-0036 gave the
/// GUI: a name that was only controls must not collapse into an empty row,
/// because an empty row is a row nobody sees. One replacement per control also
/// keeps the length comparable, so a name padded with controls still looks
/// suspicious rather than short.
///
/// # This is for drawing only
///
/// The name that goes into the manifest and the name written to disk go through
/// ADR-0027's rules, which are stricter and different. Confusing the two would
/// mean sanitising for a screen and trusting the result on a filesystem.
#[must_use]
pub fn safe_terminal_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            let code = character as u32;
            let is_c0 = code <= 0x1F || code == 0x7F;
            let is_c1 = (0x80..=0x9F).contains(&code);
            if is_c0 || is_c1 {
                '\u{FFFD}'
            } else {
                character
            }
        })
        .collect()
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

    use super::{
        BORING_FIRST, Channel, Situation, advise, channels_for, plain_duration, safe_terminal_name,
        seconds_for,
    };

    const ONE_MEGABYTE: u64 = 1024 * 1024;

    #[test]
    fn a_carriage_return_cannot_rewrite_the_line_it_is_drawn_in() {
        // **The attack.** A file named with a carriage return and some
        // reassuring text draws that text *over* what the person was reading —
        // and they are about to decide whether to accept it.
        let hostile = "factura.pdf\r      <-- seguro";
        let safe = safe_terminal_name(hostile);
        assert!(!safe.contains('\r'), "{safe:?}");
        assert!(safe.starts_with("factura.pdf"));
    }

    #[test]
    fn an_escape_sequence_cannot_move_the_cursor() {
        let hostile = "nota\u{1b}[2J\u{1b}[Hborrado";
        let safe = safe_terminal_name(hostile);
        assert!(!safe.contains('\u{1b}'));
        // The brackets and letters survive: they are ordinary text once the
        // escape that gave them meaning is gone.
        assert!(safe.contains("[2J"), "{safe:?}");
    }

    #[test]
    fn a_name_that_was_only_controls_does_not_become_an_empty_row() {
        // ADR-0036 decided this for the GUI and ADR-0047 §6 keeps it: an empty
        // row is a row nobody sees, and "nothing" is not a safe rendering of a
        // name somebody chose to make hostile.
        let safe = safe_terminal_name("\u{0}\u{1}\u{7f}\u{9b}");
        assert_eq!(safe.chars().count(), 4);
        assert!(safe.chars().all(|c| c == '\u{FFFD}'), "{safe:?}");
    }

    #[test]
    fn the_c1_range_is_covered_and_not_just_the_obvious_c0() {
        // `U+009B` is CSI — one character that starts a control sequence on a
        // terminal decoding Latin-1. Sanitising only C0 leaves the same attack
        // available in one byte less.
        assert_eq!(safe_terminal_name("\u{9b}2J"), "\u{FFFD}2J");
    }

    #[test]
    fn ordinary_names_including_accents_and_emoji_are_untouched() {
        // The control. A sanitiser that mangled normal names would be worse
        // than none: people would stop trusting what they read.
        for name in [
            "informe.pdf",
            "año-2026.txt",
            "foto \u{1f4f7}.jpg",
            "отчёт.txt",
        ] {
            assert_eq!(safe_terminal_name(name), name, "{name:?} was altered");
        }
    }

    #[test]
    fn the_order_is_the_one_adr_0046_fixed_and_serial_beats_optical() {
        // Both are slow; serial is an order of magnitude faster and does not ask
        // anybody to hold a phone steady for minutes. The `Ord` derive is what
        // encodes it, so this is the test that stops a reordering of the enum
        // from silently changing the product's advice.
        assert!(Channel::Network < Channel::DirectCable);
        assert!(Channel::DirectCable < Channel::Serial);
        assert!(Channel::Serial < Channel::Optical);
    }

    #[test]
    fn the_estimates_are_r8s_numbers_and_not_rounder_ones() {
        // A megabyte, on each channel, from `R8` §4 and §5.1. If these drift the
        // advice starts lying, and an estimate that runs under is a promise
        // broken at minute nine.
        assert_eq!(seconds_for(Channel::Network, ONE_MEGABYTE), 1);
        assert_eq!(seconds_for(Channel::Serial, ONE_MEGABYTE), 114); // ~2 min
        assert_eq!(seconds_for(Channel::Optical, ONE_MEGABYTE), 128); // ~2 min
        // And a photo, which is the case `R8` §4 calls out: 6-17 minutes.
        let photo = 5 * ONE_MEGABYTE;
        let minutes = seconds_for(Channel::Optical, photo) / 60;
        assert!(
            (6..=17).contains(&minutes),
            "a 5 MB photo reads as {minutes} min"
        );
    }

    #[test]
    fn the_boring_question_comes_before_a_slow_channel_and_not_before_a_fast_one() {
        // `FASE-16` §2. Asking it when the network works would be noise; not
        // asking it before seventeen minutes of QR codes is bad product.
        let slow = Situation {
            has_serial_port: true,
            payload_len: ONE_MEGABYTE,
            ..Situation::default()
        };
        let (text, _) = advise(slow);
        assert!(
            text.contains(BORING_FIRST),
            "the slow path skipped the question"
        );

        let fast = Situation {
            has_network: true,
            peer_discovered: true,
            payload_len: ONE_MEGABYTE,
            ..Situation::default()
        };
        let (text, _) = advise(fast);
        assert!(
            !text.contains(BORING_FIRST),
            "the question was asked when the network was available, which is noise"
        );
    }

    #[test]
    fn an_address_with_nobody_answering_reads_as_a_direct_cable() {
        // Which is what it is: link-local at both ends and no responder in
        // between. Calling it "network" would send somebody to `qyro find`
        // twice.
        let situation = Situation {
            has_network: true,
            peer_discovered: false,
            payload_len: 1024,
            ..Situation::default()
        };
        assert_eq!(channels_for(situation), vec![Channel::DirectCable]);
    }

    #[test]
    fn with_nothing_available_it_says_so_and_says_what_to_plug_in() {
        // Not an empty list. An interface handed an empty list draws a blank
        // screen, and a blank screen is where somebody decides the product is
        // broken.
        let (text, channels) = advise(Situation::default());
        assert!(channels.is_empty());
        assert!(text.contains("no way to reach"));
        assert!(
            text.contains("serial") && text.contains("camera"),
            "it did not say what to plug in: {text}"
        );
    }

    #[test]
    fn every_line_of_advice_carries_a_command_somebody_can_type() {
        // Advice without a command is a diagnosis. The whole point is that the
        // next step is one line away.
        let situation = Situation {
            has_network: true,
            peer_discovered: true,
            has_serial_port: true,
            other_has_camera: true,
            payload_len: ONE_MEGABYTE,
        };
        let (text, channels) = advise(situation);
        assert_eq!(
            channels.len(),
            3,
            "network, serial and optical: {channels:?}"
        );
        for channel in channels {
            assert!(
                text.contains(channel.command()),
                "{channel:?} had no command"
            );
        }
    }

    #[test]
    fn durations_are_words_a_person_uses() {
        assert_eq!(plain_duration(0), "less than a second");
        assert_eq!(plain_duration(45), "about 45 seconds");
        assert_eq!(plain_duration(120), "about 2 minutes");
        assert_eq!(plain_duration(7200), "about 2 hours");
        // The boundary that matters: 90 seconds is still seconds, 91 is minutes.
        // A jump from "about 90 seconds" to "about 2 minutes" is the same fact
        // said two ways, and either is fine -- what is not fine is "about 127
        // seconds", which nobody says.
        assert_eq!(plain_duration(91), "about 2 minutes");
    }
}
