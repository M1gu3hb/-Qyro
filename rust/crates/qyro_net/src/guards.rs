//! A structural guard over every production file in this crate.
//!
//! `qyro_net` reads bytes a stranger controls, before that stranger has proved
//! anything. A panic on that path is a remote denial of service that needs no
//! authentication to trigger: an unauthenticated peer reaches
//! `FrameStream::read_frame` on its first packet.
//!
//! The analysis is shared with the other crates; the list below is this one's.
//! See `rust/guards/source_guard.rs`.
//!
//! # Why the byte counts matter here
//!
//! QYR-0071: this same analysis once read 13 401 of a file's 30 861 bytes and
//! reported success, because an item shape it could not parse made it swallow
//! the rest. Every assertion built on it was measuring less than half of what it
//! claimed. `assert_analysis_reached_the_end` is the check that closes that, and
//! `the_analysis_reaches_the_end_of_every_production_file` below runs it over
//! each file in this crate and reports the sizes, so a future regression shows
//! up as a number that moved rather than as a silent pass.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the shared analysis serves several crates, reads files, and must \
              fail loudly when it cannot"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Every file compiled into a release build of this crate.
const PRODUCTION_FILES: [&str; 9] = [
    "beacon.rs",
    "lib.rs",
    "discovery.rs",
    "error.rs",
    "handshake.rs",
    "pairing.rs",
    "limits.rs",
    "listener.rs",
    "stream.rs",
];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

#[test]
fn every_net_error_has_a_construction_site() {
    // Fourteen variants at the time of writing. The minimum is the guard's
    // defence against the enum being silently truncated: if a future edit leaves
    // fewer, the guard says the parse stopped early rather than that the enum
    // shrank.
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "error.rs",
        "NetError",
        12,
        &[],
    );
}

/// Every refusal a pairing string can produce is produced somewhere.
///
/// All seven are exempt from the *cross-file* requirement and the exemption is
/// the point of writing it down: `PairingError` is declared and constructed in
/// the same file because the only thing that can refuse a pairing string is the
/// parser of pairing strings. Demanding a construction site elsewhere would be
/// satisfiable only by scattering the parse.
///
/// What the call still buys is the parse floor: if this enum ever stops being
/// found, it reports zero variants and fails rather than passing silently. The
/// reachability of all seven is held by
/// `tests/pairing_contract.rs::every_way_a_pairing_string_can_be_wrong_is_its_own_refusal`
/// and its two neighbours, which construct each one by parsing a string that
/// deserves it.
#[test]
fn every_pairing_error_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(
        &PRODUCTION_FILES,
        "pairing.rs",
        "PairingError",
        7,
        &[
            "NotAPairingString",
            "WrongFieldCount",
            "UnreadableAddress",
            "UnspecifiedAddress",
            "ZeroPort",
            "FingerprintWrongLength",
            "FingerprintNotLowercaseHex",
        ],
    );
}

#[test]
fn every_socket_op_has_a_construction_site() {
    // `SocketOp` is not an `Error` by name, so the workspace meta-guard does not
    // demand this one. It is here anyway: an operation label nothing ever
    // constructs is a label that will be wrong the first time someone reads it
    // in a bug report.
    assert_every_variant_has_a_construction_site(&PRODUCTION_FILES, "error.rs", "SocketOp", 8, &[]);
}

/// The analysis reaches the last line of every production file, and says how far.
///
/// The sizes are printed rather than merely asserted because the failure this
/// guards against is quiet: an analysis that stops early still returns
/// well-formed source and still passes every `contains` check built on it.
#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        let analysed = production_source(file);
        let raw = production_source_raw(file);
        assert_analysis_reached_the_end(file, &analysed);
        println!(
            "qyro_net/src/{file}: {} bytes analysed of {} raw",
            analysed.len(),
            raw.len()
        );
        assert!(
            !analysed.is_empty(),
            "src/{file} stripped to nothing, so nothing was analysed"
        );
    }
}

/// The number ADR-0041 froze, spelled the same in both consumers.
///
/// **This guard exists because phase 14 found three copies and no original.**
/// `qyro_cli` had `DEFAULT_PORT`, Dart had `qyroDefaultPort`, and the engine —
/// the one place both of them link against — had nothing. Two consumers each
/// carrying their own copy of a frozen number is the drift this codebase keeps
/// paying for; the difference here is that a mismatch is silent, and it shows up
/// as «the other device is not answering» on a network where it is.
///
/// Reading the Dart source from a Rust test is not elegant. It is the only
/// place the two languages can be compared without a running build of both, and
/// a guard that runs beats an elegance that does not.
#[test]
fn the_two_consumers_agree_on_the_port() {
    let dart = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/qyro/lib/transfer/transfer_service.dart");
    let source = std::fs::read_to_string(&dart)
        .unwrap_or_else(|error| panic!("the Dart consumer is at {}: {error}", dart.display()));

    let needle = "const int qyroDefaultPort =";
    let start = source.find(needle).unwrap_or_else(|| {
        panic!(
            "`{needle}` is gone from the Dart side -- if the constant was \
                                   renamed, rename it here too; if it was deleted, the GUI no \
                                   longer knows what port to reach and that is the bug"
        )
    });
    let tail = source
        .get(start + needle.len()..)
        .unwrap_or_default()
        .trim_start();
    let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
    let dart_port: u16 = digits
        .parse()
        .unwrap_or_else(|error| panic!("the Dart port reads as `{digits}`: {error}"));

    assert_eq!(
        dart_port,
        crate::QYRO_PORT,
        "the GUI reaches port {dart_port} and the engine listens on {} -- ADR-0041 fixed one \
         number and there are now two",
        crate::QYRO_PORT
    );
}

/// Android must declare `INTERNET`, or nothing in this crate can run there.
///
/// **QYR-0368, P0.** `android.permission.INTERNET` was declared **only** in
/// `app/src/debug/AndroidManifest.xml` and `app/src/profile/`. Those two source
/// sets do not reach a release build: Gradle merges `main` plus the source set
/// of the variant being built, so `flutter build apk --release` produced an APK
/// whose merged manifest had no `INTERNET` at all — and the comment at the top
/// of the main manifest said «The only permission this application declares»,
/// which made the absence look deliberate.
///
/// Everything Qyro does on a network is TCP, and it starts in this crate:
/// `TcpListener::bind` in `listener.rs` and `TcpStream::connect` in `stream.rs`.
/// Without the permission both throw, and what the person sees on the phone is
/// `SocketException: Permission denied (errno = 13)` from a socket call inside
/// the native library — a message that names neither Qyro nor a permission, on
/// the one build type they would ever install.
///
/// The whole product worked in every test and could not have moved a byte on a
/// real phone. Debug and profile builds have the permission, so an emulator run
/// and a `flutter run` both pass: the only build that fails is the only build
/// anybody installs.
///
/// This guard is here rather than beside the manifest because the failure is
/// this crate's: it is the crate whose entire purpose becomes unreachable. It
/// reads the source manifest, so it runs on every commit, on every platform,
/// inside `cargo test --workspace` — which is what `scripts/gate.ps1` runs. The
/// **merged** release manifest is checked separately, by
/// `apps/qyro/test/android_manifest_test.dart`, because that file only exists
/// after `flutter build apk` and a source-only check cannot see a permission a
/// plugin removes or adds.
#[test]
fn the_android_manifest_declares_internet() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../apps/qyro/android/app/src/main/AndroidManifest.xml");
    let source = std::fs::read_to_string(&manifest).unwrap_or_else(|error| {
        panic!("the Android manifest is at {}: {error}", manifest.display())
    });

    // Comments are stripped first. The manifest *names* permissions in prose to
    // explain why they are absent, and a substring search over the raw file
    // would read the explanation as the declaration. This is the same reason
    // `android_manifest_test.dart` strips them.
    let declared = declared_permissions(&source);

    assert!(
        declared
            .iter()
            .any(|name| name == "android.permission.INTERNET"),
        "`android.permission.INTERNET` is not declared in the manifest that \
         reaches a release build. It is declared in app/src/debug/ and \
         app/src/profile/, and neither of those reaches release. Everything \
         this crate does is TCP, so the release APK cannot open a socket: \
         listener.rs fails at bind and stream.rs at connect, with errno 13 and \
         no mention of Qyro. Declared here today: {declared:?}"
    );
}

/// The control, and it is not decoration.
///
/// A guard that only asserts a presence passes the day somebody writes the
/// permission into a **comment** while deleting the declaration — which is the
/// exact shape of the defect above, where prose said one thing and the merged
/// manifest another. So: the stripper must actually strip, proved on this very
/// file's own comments, and the parser must actually parse.
#[test]
fn the_permission_parser_reads_declarations_and_not_prose() {
    let commented = r#"
        <manifest>
          <!-- android.permission.INTERNET is explained here and not declared,
               and a substring search would find it anyway. -->
          <uses-permission android:name="android.permission.CAMERA" />
        </manifest>
    "#;
    assert_eq!(
        declared_permissions(commented),
        vec!["android.permission.CAMERA".to_owned()],
        "a permission named only inside a comment was read as declared"
    );

    let declaring = r#"
        <manifest>
          <uses-permission android:name="android.permission.INTERNET" />
        </manifest>
    "#;
    assert_eq!(
        declared_permissions(declaring),
        vec!["android.permission.INTERNET".to_owned()],
        "a real declaration was not read"
    );
}

/// Every `<uses-permission>` name in an Android manifest, comments removed.
///
/// Deliberately not a regex crate: this workspace has no regex dependency and
/// adding one so a guard can read XML would put a dependency in the shipped
/// graph for a test. The scan is literal and its behaviour is pinned by
/// `the_permission_parser_reads_declarations_and_not_prose`.
fn declared_permissions(xml: &str) -> Vec<String> {
    let mut stripped = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(open) = rest.find("<!--") {
        stripped.push_str(&rest[..open]);
        match rest[open..].find("-->") {
            Some(close) => rest = &rest[open + close + 3..],
            // An unterminated comment swallows the remainder, which is what an
            // XML parser does too.
            None => return collect_permissions(&stripped),
        }
    }
    stripped.push_str(rest);
    collect_permissions(&stripped)
}

fn collect_permissions(xml: &str) -> Vec<String> {
    let needle = "<uses-permission";
    let mut names = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(needle) {
        rest = &rest[start + needle.len()..];
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..end];
        let attribute = "android:name=\"";
        if let Some(open) = tag.find(attribute) {
            let value = &tag[open + attribute.len()..];
            if let Some(close) = value.find('"') {
                names.push(value[..close].to_owned());
            }
        }
        rest = &rest[end..];
    }
    names
}
