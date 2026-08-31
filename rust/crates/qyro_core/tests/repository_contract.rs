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
