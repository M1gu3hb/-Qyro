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
