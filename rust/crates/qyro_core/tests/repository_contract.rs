//! Facts about **the repository** that no single crate owns, asserted where the
//! gate can see them.
//!
//! `scripts/gate.ps1` runs the `cargo` lines it reads out of `ci.yml`, and one
//! of them is `cargo test --workspace`. So a contract written here runs on every
//! commit, on every platform, in CI and locally, with no extra tool installed.
//! That is the property being bought: the alternative homes for these checks are
//! a PowerShell script that only runs in one job and a comment nobody executes.
//!
//! `qyro_core` rather than a crate of its own: it is the crate with no
//! dependencies and no platform, so a repository-level contract costs nothing
//! here and needs no new workspace member.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    // `qyro_core` is at rust/crates/qyro_core.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the repository root is three levels above this crate")
}

/// The signing key's password file must be impossible to commit by accident.
///
/// **This repository is public.** `apps/qyro/android/key.properties` holds
/// `storePassword`, `keyPassword`, `keyAlias` and the path to the keystore, in
/// plain text — `app/build.gradle.kts` reads exactly those four keys — and the
/// owner's machine has that file, because the v1.0 APK is signed with a real key
/// whose certificate `docs/release/v1.0.md` publishes.
///
/// **It was already ignored, and that is not the same as guarded.** The rule
/// lived in `apps/qyro/android/.gitignore`, which is a file **Flutter
/// generates**: `flutter create .` over an existing project rewrites it from the
/// template, and the template's version of that line is there for the Android
/// plugin's sake, not for this repository's secret. So the coverage was real and
/// borrowed. This test asks git the effective question — «would committing this
/// path be refused?» — so it keeps passing wherever the rule lives, and turns
/// red the moment no rule covers it at all. The root `.gitignore` now carries
/// its own copy, which is what makes the answer survive a regenerated Android
/// project.
///
/// `key.properties.example` stays tracked on purpose: it is how somebody learns
/// what the real file needs, and it has no secrets in it.
#[test]
fn the_signing_passwords_cannot_be_committed_by_accident() {
    let root = repo_root();
    let secret = "apps/qyro/android/key.properties";
    let example = "apps/qyro/android/key.properties.example";

    if !root.join(".git").exists() {
        eprintln!("[skip] no .git here, so `git check-ignore` has nothing to answer");
        return;
    }

    // `check-ignore` answers with every rule in force, in the order git applies
    // them, which is the actual behaviour. Grepping the root `.gitignore` would
    // have missed the nested rule entirely and demanded a duplicate for a
    // property that already held.
    let ignored = |path: &str| {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["check-ignore", "-q", path])
            .status()
            .expect("git must be runnable to answer what is ignored")
            .success()
    };

    assert!(
        ignored(secret),
        "no .gitignore rule covers {secret}. This repository is public and that \
         file holds the keystore passwords in plain text; \
         app/build.gradle.kts reads storePassword, keyPassword, keyAlias and \
         storeFile straight out of it. A `git add -A` on the signing machine \
         publishes them."
    );

    // The control, and it is the half that makes the assertion above mean
    // something. A `.gitignore` that ignored everything would satisfy it while
    // making the repository unbuildable-by-instruction for the next person.
    assert!(
        !ignored(example),
        "{example} is ignored too, so nobody can learn what the real file needs"
    );
    assert!(
        root.join(example).exists(),
        "{example} is gone, so nobody can learn what the real file needs"
    );
}

/// And no secret is tracked **right now**, whatever `.gitignore` says.
///
/// The rule above is about the future. This is about the present: `.gitignore`
/// has no effect on a path git already tracks, so a file added before the rule
/// existed stays tracked and stays published. Asked of git rather than of the
/// filesystem, because that is the question — «is this in the repository», not
/// «is this on this disk».
#[test]
fn no_secret_is_tracked_today() {
    let root = repo_root();
    if !root.join(".git").exists() {
        // A source tarball has no git. The contract above still ran, and this
        // one has nothing to ask. Said out loud rather than passed silently.
        eprintln!("[skip] no .git here, so `git ls-files` has nothing to answer");
        return;
    }

    let listing = Command::new("git")
        .arg("-C")
        .arg(&root)
        .arg("ls-files")
        .output()
        .expect("git must be runnable to answer what is tracked");
    assert!(listing.status.success(), "`git ls-files` failed");
    let tracked = String::from_utf8_lossy(&listing.stdout);

    // Exact file names and extensions, not a keyword sweep: a sweep for "key"
    // matches `keystore_wrapper.kt` and teaches people to ignore this test.
    let offenders: Vec<&str> = tracked
        .lines()
        .filter(|path| {
            let name = path.rsplit('/').next().unwrap_or(path);
            name == "key.properties"
                || name == ".env"
                || name.ends_with(".jks")
                || name.ends_with(".keystore")
                || name.ends_with(".p12")
                || name.ends_with(".pfx")
                || name.ends_with(".mobileprovision")
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "these are tracked in a public repository and must not be: {offenders:?}. \
         Removing the file is not enough -- it stays in the history, so the key \
         has to be rotated."
    );

    // The control. A filter that matched nothing would satisfy the assertion
    // above forever, so prove the same filter does find the example file it is
    // deliberately shaped to let through, and would find the real one.
    let name_of = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    assert_eq!(
        name_of("apps/qyro/android/key.properties"),
        "key.properties"
    );
    assert_ne!(
        name_of("apps/qyro/android/key.properties.example"),
        "key.properties",
        "the example would be flagged as a secret, which would make this test \
         unpassable"
    );
    assert!(
        tracked
            .lines()
            .any(|path| path.ends_with("key.properties.example")),
        "the example is not tracked, so this filter has nothing to prove itself \
         against"
    );
}

/// The optical channel must have a door on the phone.
///
/// **QYR-0371, and it is the twelfth capability this project has found written,
/// tested and unreachable from the product.** `PeersScreen` has taken an
/// `onScan` callback since phase 24B, with a comment beside the button calling
/// itself «el llamante de producción del escáner». The one production
/// construction of that screen —`TransferHome`'s— never passed one, so
/// `onPressed` was `null`: Material draws such a button greyed out and refuses
/// to tap it. The scanner, the Kotlin channel, CameraX, `qyro_eye` and
/// `qyro_fountain` were all complete, and there was no way in.
///
/// **Why this is asserted from Rust.** A widget test can prove the screen
/// honours the callback it is handed — and there is one, in
/// `transfer_screens_test.dart`. What a widget test cannot prove is that
/// *production* hands it one, because production's caller is `HomeScreen`
/// building the real engine, which a test does not do. This is the same shape as
/// `qyro_net::guards::the_two_consumers_agree_on_the_port`: reading another
/// language's source from a Rust test is not elegant, and it is the only place
/// the wiring can be checked without a running build of both. It runs inside
/// `cargo test --workspace`, which is what the gate runs.
#[test]
fn the_optical_channel_has_a_door() {
    let root = repo_root();
    let home = root.join("apps/qyro/lib/home/home_screen.dart");
    let source = std::fs::read_to_string(&home)
        .unwrap_or_else(|error| panic!("the home screen is at {}: {error}", home.display()));

    // Comments stripped: the file *explains* this wiring at length, and a
    // substring search over the raw text would read the explanation as the
    // thing it explains -- the same trap `android_manifest_test.dart` documents.
    let code = strip_line_comments(&source);

    assert!(
        code.contains("TransferHome("),
        "HomeScreen no longer builds TransferHome, so this guard is checking a \
         wiring that has moved. Follow it rather than deleting this."
    );
    assert!(
        code.contains("onScan:"),
        "HomeScreen builds TransferHome without `onScan:`, so PeersScreen gets \
         a null callback and the «Scan codes» button renders disabled. The \
         optical channel -- the one that works with no network of any kind -- \
         has no entrance on the phone."
    );
    assert!(
        code.contains("ScanScreen("),
        "nothing here opens ScanScreen, so `onScan` leads somewhere else or \
         nowhere. The button being enabled is not the property; reaching the \
         scanner is."
    );

    // The control. If the stripper ate the code as well as the comments, every
    // assertion above would fail loudly rather than pass emptily -- but the
    // reverse mistake is silent, so prove the stripper actually strips.
    assert!(
        source.contains("// **QYR-0371"),
        "the explanation of this wiring is gone from home_screen.dart; if the \
         wiring went with it, the assertions above should have said so first"
    );
    assert!(
        !code.contains("QYR-0371"),
        "the comment stripper did not strip, so the assertions above may be \
         reading prose instead of code"
    );
}

/// Dart line comments removed, string literals left alone.
///
/// Deliberately only `//`: this is not a Dart parser and does not need to be.
/// The files it reads use line comments for prose, and a `//` inside a string
/// literal in those files would at worst delete code and make the assertions
/// fail loudly, which is the safe direction for a mistake to fall.
fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => line.get(..at).unwrap_or(""),
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `archivo:línea` in the parity table points at something nameable.
///
/// **A citation by line number ages the moment somebody edits the file above
/// it**, and this repository has now paid for that twice. The first time,
/// fifteen of the table's citations pointed at `setState(() {`, `};`, `}` and a
/// comment — thirteen rows of fourteen — while the document said it was checked.
/// The second time was the round that closed QYR-0368 to QYR-0371: adding lines
/// to `flows.rs`, `native_transfer_service.dart` and `transfer_screens.dart`
/// pushed eleven of the fourteen off their targets, eleven days after the first
/// repair.
///
/// `scripts/check_parity.ps1` makes the same check and is the specification for
/// it (ADR-0046 §3). This is not a replacement: it is the same rule inside
/// `cargo test --workspace`, which is what `scripts/gate.ps1` runs and what CI
/// runs on every commit. The PowerShell one has to be remembered; this one
/// cannot be forgotten.
///
/// **What «nameable» excludes**, and it is deliberately narrow: a bare brace, a
/// doc or line comment, a blank line. Whether the line corresponds to the
/// *capability* is not mechanisable and pretending otherwise is how the first
/// repair produced «Rechazar con motivo → `_drainReceive`» — precision that is
/// false, which is worse than a stale number because it no longer looks stale.
#[test]
fn the_parity_table_still_points_at_code() {
    let root = repo_root();
    let table = root.join("docs/PARIDAD-GUI-CLI.md");
    let source = std::fs::read_to_string(&table)
        .unwrap_or_else(|error| panic!("the parity table is at {}: {error}", table.display()));

    let body = source
        .split_once("<!-- PARIDAD-INICIO -->")
        .and_then(|(_, rest)| rest.split_once("<!-- PARIDAD-FIN -->"))
        .map(|(rows, _)| rows)
        .expect("the table must keep its PARIDAD-INICIO / PARIDAD-FIN markers");

    let citations = citations_in(body);
    // The reader has to work before it can be believed. A pattern that matched
    // nothing would make this test pass while every citation rotted.
    assert!(
        citations.len() >= 20,
        "found {} citations in the parity table, so the reader broke rather \
         than the table: {citations:?}",
        citations.len()
    );

    let mut rotten = Vec::new();
    for (path, number) in &citations {
        let file = root.join(path);
        let Ok(content) = std::fs::read_to_string(&file) else {
            rotten.push(format!("{path}:{number} — the file does not exist"));
            continue;
        };
        let Some(line) = content.lines().nth(number - 1) else {
            rotten.push(format!(
                "{path}:{number} — the file has only {} lines",
                content.lines().count()
            ));
            continue;
        };
        let text = line.trim();
        let nameable = !text.is_empty()
            && !text.starts_with("//")
            && !text.starts_with('#')
            && !text
                .trim_matches(|c: char| "{}()[];,".contains(c) || c.is_whitespace())
                .is_empty();
        if !nameable {
            rotten.push(format!("{path}:{number} — points at `{text}`"));
        }
    }

    assert!(
        rotten.is_empty(),
        "the parity table cites lines that say nothing. A citation that points \
         at a brace or a comment reads as verified and is not:\n  {}\n\
         Put them back by hand against the declaration listing -- resolving each \
         number to the nearest symbol above was tried and thrown away, because \
         it produced false precision.",
        rotten.join("\n  ")
    );
}

/// Every `` `path:line` `` inside a markdown fragment.
fn citations_in(body: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for chunk in body.split('`').skip(1).step_by(2) {
        let Some((path, number)) = chunk.rsplit_once(':') else {
            continue;
        };
        if !(path.ends_with(".rs") || path.ends_with(".dart")) {
            continue;
        }
        if let Ok(number) = number.parse::<usize>()
            && number > 0
        {
            found.push((path.to_owned(), number));
        }
    }
    found
}

/// The front-page documents may not re-assert what the code disproved.
///
/// **Three documents described a Qyro from before phase 12, and one of them told
/// agents it was canonical.** `AGENTS.md` said the scope «no incluye
/// transferencia, transporte, LAN» and closed with «Qyro sigue sin transferir
/// archivos». `PROTOCOL.md` said «nada pone todavía un frame en un socket».
/// `README.md` said discovery «no existe para la aplicación» and that «no se
/// escanea (no hay cámara)». All four were false, and `qyro_net`, `qyro_transfer`,
/// `qyro_discovery.dart`, `ScanScreen` and the `CAMERA` permission are the
/// files that say so.
///
/// A stale sentence in a document nobody reads is a nuisance. A stale sentence
/// on the front page of a repository, in the file that calls itself the source
/// of truth, is a person building on a project that does not exist. So the
/// retired claims are named here, and re-asserting one turns the gate red.
///
/// **Narrow on purpose.** This forbids **five exact sentences** that were
/// measured false against named files. It is not a style check and must never
/// become one: a documentation guard that starts having opinions gets disabled,
/// and then it is not there for the sentence that matters.
#[test]
fn the_front_page_does_not_reassert_what_the_code_disproved() {
    let root = repo_root();
    // Each: the document, the retired claim, and the file that disproves it.
    let retired: [(&str, &str, &str); 5] = [
        (
            "AGENTS.md",
            "Qyro sigue sin transferir archivos",
            "rust/crates/qyro_net/src/stream.rs",
        ),
        (
            "AGENTS.md",
            "no** incluye transferencia",
            "rust/crates/qyro_transfer/src/session.rs",
        ),
        (
            "PROTOCOL.md",
            "nada pone todavía un frame en un socket",
            "rust/crates/qyro_net/src/listener.rs",
        ),
        (
            "README.md",
            "no existe para la aplicación",
            "apps/qyro/lib/discovery/qyro_discovery.dart",
        ),
        (
            "README.md",
            "no se escanea",
            "apps/qyro/lib/scanner/scan_screen.dart",
        ),
    ];

    let mut resurrected = Vec::new();
    for (document, claim, disproved_by) in retired {
        let path = root.join(document);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{document} must exist to be checked: {error}"));

        // The blockquotes that *record* each retired sentence are the point of
        // this being readable, so the sentence is allowed to appear inside one.
        // Everything else is an assertion.
        let asserted = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('>'))
            .any(|line| line.contains(claim));
        if asserted {
            resurrected.push(format!(
                "{document} asserts «{claim}» again, and \
                 {disproved_by} is the file that disproves it"
            ));
        }

        // And the file that disproves it must still be there. If the code went,
        // the claim might be true again and this guard would be the last thing
        // standing between that and a document nobody re-read.
        assert!(
            root.join(disproved_by).exists(),
            "{disproved_by} is gone. It is what made «{claim}» false in \
             {document}; if the capability was removed, this guard is now \
             wrong and the document may be right."
        );
    }

    assert!(resurrected.is_empty(), "{}", resurrected.join("\n"));

    // The control, and without it this test passes on an empty file. Each
    // document must still carry the blockquote that records what it retired,
    // which also proves the blockquote exemption above is exercised rather than
    // theoretical.
    for (document, claim) in [
        ("AGENTS.md", "Qyro sigue sin transferir archivos"),
        ("PROTOCOL.md", "nada pone todavía un frame en un socket"),
    ] {
        let text = std::fs::read_to_string(root.join(document)).expect("readable");
        assert!(
            text.contains(claim),
            "{document} no longer records that it once said «{claim}», so the \
             blockquote exemption above is checking nothing and the sentence \
             could come back unremarked"
        );
    }
}

/// The phone must be told where to write, and by the side that knows.
///
/// **QYR-0373, and it was a P0 on the phone.** `defaultDestination()` returned,
/// on Android, `Directory.current.path + "/Qyro"`, under a comment that said
/// «Android hands the app its own directory; the Kotlin side passes it in. Until
/// it does, the process working directory is the honest answer».
///
/// **A process on Android has `/` as its working directory.** So the answer was
/// `/Qyro` — the root of the filesystem, which no application can write.
/// `Directory('/Qyro').createSync(recursive: true)` throws, and it throws inside
/// `receive()` **before a single state is emitted**: tapping Receive on the
/// phone did nothing visible. The Kotlin side that was going to pass the path
/// was never written, and the comment saying so had been reading like a
/// temporary note ever since.
///
/// This guard is in Rust for the same reason as the two above: the property is
/// «two languages agree», and there is no other place both can be read without a
/// build of each. A Dart test can prove the bridge asks correctly — and
/// `qyro_paths_test.dart` does — but not that Kotlin answers, nor that the
/// screen asks at all.
#[test]
fn the_phone_is_told_where_to_write() {
    let root = repo_root();

    let kotlin = root.join("apps/qyro/android/app/src/main/kotlin/com/owner/qyro/PathsChannel.kt");
    let kotlin_source = std::fs::read_to_string(&kotlin).unwrap_or_else(|error| {
        panic!(
            "the channel that answers where to write is at {}: {error}",
            kotlin.display()
        )
    });
    assert!(
        kotlin_source.contains("getExternalFilesDir"),
        "PathsChannel no longer uses getExternalFilesDir. It is the one\
         directory that needs no permission, that the person can reach over USB, \
         and that goes away on uninstall. getFilesDir is writable too and \
         invisible from outside: a file that arrives and its owner cannot open \
         has not arrived."
    );

    let activity =
        root.join("apps/qyro/android/app/src/main/kotlin/com/owner/qyro/MainActivity.kt");
    let activity_source = std::fs::read_to_string(&activity).expect("MainActivity is readable");
    assert!(
        activity_source.contains("PathsChannel.CHANNEL"),
        "MainActivity does not register PathsChannel, so the channel exists and \
         nothing answers on it -- which is the same as not existing, and is how \
         the Kotlin side of this went missing the first time"
    );

    let screen = root.join("apps/qyro/lib/transfer/transfer_screens.dart");
    let screen_source = std::fs::read_to_string(&screen).expect("the screens are readable");
    let code = strip_line_comments(&screen_source);
    assert!(
        code.contains("androidDestination()"),
        "the receive screen does not ask where to write, so it falls back to \
         defaultDestination(), which on Android is `/Qyro` and is not writable"
    );
    assert!(
        !code.contains("destination: '',"),
        "the receive screen still passes an empty destination, which means «use \
         the default» -- and the default on Android is the root of the filesystem"
    );

    // The control: the constants this guard matches on are real, not hopeful.
    // A guard that asserts on strings nobody defines passes for the wrong reason.
    assert!(
        kotlin_source.contains("const val CHANNEL = \"dev.qyro/paths\""),
        "PathsChannel does not declare the channel name this guard assumes"
    );
    let bridge = std::fs::read_to_string(root.join("apps/qyro/lib/transfer/qyro_paths.dart"))
        .expect("the Dart bridge is readable");
    assert!(
        bridge.contains("MethodChannel('dev.qyro/paths')"),
        "the Dart side opens a different channel than Kotlin registers, so the \
         call would return MissingPluginException and the screen would silently \
         fall back to the broken default"
    );
}
