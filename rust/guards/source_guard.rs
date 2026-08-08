// Source analysis shared by the three anti-panic guards.
//
// **Not a crate.** This file is pulled into `qyro_crypto`, `qyro_protocol` and
// `qyro_manifest` with `include!`, from each crate's `src/guards.rs`:
//
// ```ignore
// include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../guards/source_guard.rs"));
// ```
//
// One file rather than three copies, because a duplicated analysis is three
// analyses that can disagree about what counts as production code — and the
// guard exists precisely to stop that kind of drift. `include!` rather than a
// workspace member because a new crate would appear in `Cargo.lock` and in the
// audited dependency count for a file that only ever compiles under
// `cfg(test)`.
//
// Every crate that includes this keeps its own `PRODUCTION_FILES` list: the
// analysis is shared, the policy is not.
//
// # What it does that a Clippy `#![deny(...)]` cannot
//
// *It notices a module nobody added the lint to.* An attribute protects the
// module it is written in, and a new file with no attribute is unprotected and
// looks exactly like a protected one.
//
// *It catches `assert!`*, which has no lint and ends the process exactly as
// `panic!` does.
//
// *It derives which modules are exempt from the code itself.* A hand-written
// exemption list can be satisfied by deleting a `#[cfg(test)]`, which is the
// same shape of defect the guard exists to close (QYR-0042).
//
// The analysis refuses to guess: anything it cannot account for makes it fail
// rather than pass.

use std::collections::BTreeSet;
use std::fs;

/// Macros that end the process, and the three that end it only in debug builds.
///
/// `debug_assert!` is here because a release build silently skipping a check is
/// not the property these crates want either: an invariant worth stating is
/// worth returning an error for.
const PROCESS_ENDING: [&str; 12] = [
    ".unwrap()",
    ".expect(",
    "panic!(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
    "assert!(",
    "assert_eq!(",
    "assert_ne!(",
    "debug_assert!(",
    "debug_assert_eq!(",
    "debug_assert_ne!(",
];

/// The `src` directory of the crate that included this file.
fn source_root() -> String {
    format!("{}/src", env!("CARGO_MANIFEST_DIR"))
}

/// Reads one production file with everything non-production removed.
pub(crate) fn production_source(relative_path: &str) -> String {
    let path = format!("{}/{relative_path}", source_root());
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    // Comments first: a doc comment naming `panic!`, or prose carrying an
    // unbalanced brace, must not reach any pass below.
    let source = strip_comments(&source);
    let source = strip_compile_time_assertions(&source);
    strip_test_only_items(&source)
}

/// Reads one production file with only comments removed.
///
/// The reference the overrun check compares against: comments go because the
/// stripped source has none either, and nothing else is touched.
fn production_source_raw(relative_path: &str) -> String {
    let path = format!("{}/{relative_path}", source_root());
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    strip_comments(&source)
}

/// Removes line comments, so prose about `panic!` is not mistaken for one.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Removes assertions that are evaluated at compile time.
///
/// Two spellings, both of which stop the *build* rather than a running process:
/// `const _: () = assert!(A == B);` as an item, and an inline `const { ... }`
/// block, which is how a generic function states a bound on its own parameters.
///
/// The exemption is deliberately narrow — those two exact prefixes and nothing
/// wider — so it cannot quietly grow to cover a runtime assertion.
fn strip_compile_time_assertions(source: &str) -> String {
    const MARKERS: [&str; 2] = ["const _: () =", "const {"];
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    loop {
        let Some((position, marker)) = MARKERS
            .iter()
            .filter_map(|marker| rest.find(marker).map(|position| (position, *marker)))
            .min_by_key(|(position, _)| *position)
        else {
            break;
        };
        out.push_str(&rest[..position]);
        // An inline block is re-read from its opening brace so `item_end` sees
        // the body; an item is read from just after its `=`.
        let after = if marker == "const {" {
            &rest[position + "const".len()..]
        } else {
            &rest[position + marker.len()..]
        };
        let end = item_end(after)
            .unwrap_or_else(|| panic!("a compile-time assertion with no end: {marker}"));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// Removes every item that does not exist in a release build.
fn strip_test_only_items(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;

    loop {
        let Some((position, marker)) = GATE_MARKERS
            .iter()
            .filter_map(|marker| rest.find(marker).map(|position| (position, *marker)))
            .min_by_key(|(position, _)| *position)
        else {
            break;
        };
        out.push_str(&rest[..position]);
        let after = &rest[position + marker.len()..];
        let end = item_end(after)
            .unwrap_or_else(|| panic!("a test-only item with neither a body nor a `;`"));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The attributes that keep an item out of a release build.
///
/// `#[cfg(not(any(test, fuzzing)))]` is production and must not match: every
/// literal here begins `#[cfg(` followed by the condition itself, so a negated
/// form never matches any of them.
const GATE_MARKERS: [&str; 3] = [
    "#[cfg(test)]",
    "#[cfg(any(test, fuzzing))]",
    "#[cfg(fuzzing)]",
];

/// Finds where one item ends: the first `;` at depth zero, or the close of the
/// first `{` opened at depth zero.
///
/// Depth-aware on purpose. Deciding by "whichever of `;` and `{` comes first"
/// gets `-> &[u8; 32] {` wrong, because the return type carries a semicolon
/// before the body opens, and gets `#[allow(...)] mod corpus;` wrong in the
/// other direction. Both spellings are in these crates.
///
/// String literals are skipped so a brace inside one cannot unbalance the
/// count. Character literals are not: `&'a str` is indistinguishable from an
/// opening quote without parsing Rust, and these crates have lifetimes and no
/// braces in char literals.
fn item_end(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut index = 0usize;
    let mut opened_body = false;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += if bytes[index] == b'\\' { 2 } else { 1 };
                }
            }
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => {
                if round == 0 && square == 0 && curly == 0 {
                    opened_body = true;
                }
                curly += 1;
            }
            b'}' => {
                curly = curly.saturating_sub(1);
                if opened_body && curly == 0 && round == 0 && square == 0 {
                    return Some(index + 1);
                }
            }
            b';' if round == 0 && square == 0 && curly == 0 => return Some(index + 1),
            // A struct field or enum variant ends at a comma and has neither a
            // body nor a semicolon. Without this the scan runs past the closing
            // brace of the enclosing type — `}` cannot return, because no body
            // was opened at depth zero — and swallows whatever follows.
            //
            // `#[cfg(test)] peak_content_held: usize,` in `qyro_transfer` did
            // exactly that: the analysis saw 13 401 bytes of a 30 861-byte file
            // and the panic guard had been reading less than half of it since
            // sprint 5A (QYR-0071).
            b',' if round == 0 && square == 0 && curly == 0 => return Some(index + 1),
            _ => {}
        }
        index += 1;
    }
    None
}

/// Every `.rs` file under `src`, as a path relative to it.
fn collect_sources(directory: &str, prefix: &str, out: &mut Vec<String>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| panic!("{directory}: {error}"));
    for entry in entries {
        let entry = entry.expect("a readable directory entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().expect("a knowable file type");
        if kind.is_dir() {
            collect_sources(&entry.path().to_string_lossy(), &relative, out);
        } else if name.ends_with(".rs") {
            out.push(relative);
        }
    }
}

/// The name of a module declared, without a body, by this item.
fn declared_module_name(item: &str) -> Option<String> {
    let trimmed = item.trim();
    // An inline `mod name { ... }` has a body and no file of its own.
    if !trimmed.ends_with(';') {
        return None;
    }
    let position = trimmed.rfind("mod ")?;
    let name = trimmed
        .get(position + "mod ".len()..)?
        .trim_end_matches(';')
        .trim();
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(name.to_owned())
}

/// Module names this source declares behind a test-only gate.
fn gated_child_modules(source: &str) -> Vec<String> {
    let source = strip_comments(source);
    let mut out = Vec::new();
    let mut rest = source.as_str();

    loop {
        let Some((position, marker)) = GATE_MARKERS
            .iter()
            .filter_map(|marker| rest.find(marker).map(|position| (position, *marker)))
            .min_by_key(|(position, _)| *position)
        else {
            break;
        };
        let after = &rest[position + marker.len()..];
        let Some(end) = item_end(after) else { break };
        if let Some(name) = declared_module_name(&after[..end]) {
            out.push(name);
        }
        rest = &after[end..];
    }
    out
}

/// The directory a module's children live in, for a file relative to `src`.
///
/// `lib.rs` owns the crate root, `aead/mod.rs` owns `aead/`, and `identity.rs`
/// owns `identity/`.
fn module_directory(file: &str) -> String {
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    if stem == "lib" {
        return String::new();
    }
    stem.strip_suffix("/mod").unwrap_or(stem).to_owned()
}

/// Every file that a test-only module declaration keeps out of a release build.
///
/// Derived, never listed. A hand-written exemption list is satisfied by
/// deleting the `#[cfg(test)]` it was written for, which turns a test file into
/// an unguarded production file with nothing failing (QYR-0042). Here the gate
/// *is* the exemption, so removing it moves the file into the production set.
fn gated_files() -> BTreeSet<String> {
    let root = source_root();
    let mut files = Vec::new();
    collect_sources(&root, "", &mut files);

    let mut gated = BTreeSet::new();
    for file in &files {
        let source = fs::read_to_string(format!("{root}/{file}")).unwrap_or_default();
        let directory = module_directory(file);
        for name in gated_child_modules(&source) {
            let base = if directory.is_empty() {
                name
            } else {
                format!("{directory}/{name}")
            };
            gated.insert(format!("{base}.rs"));
            gated.insert(format!("{base}/mod.rs"));
            // A gated module gates everything beneath it.
            let nested = format!("{base}/");
            for candidate in &files {
                if candidate.starts_with(&nested) {
                    gated.insert(candidate.clone());
                }
            }
        }
    }
    gated
}

/// Fails if any listed production file can end the process.
/// Fails if a variant of `enum_name` is declared and never constructed.
///
/// Lifted out of `qyro_crypto` in sprint 5B.1. It was written there in 4C.2,
/// after `HandshakeError` declared four variants nothing produced, and then it
/// stayed there: `qyro_transfer` arrived in 5A with unconstructed variants and
/// no guard to say so (QYR-0070). A check that lives in one crate protects one
/// crate, and this is the file every crate already includes.
///
/// `exempt` is the escape hatch, and it is deliberately noisy: a variant listed
/// there has to be argued at the call site. Being unconstructed and unlisted is
/// the failure.
fn assert_every_variant_has_a_construction_site(
    production: &[&str],
    declared_in: &str,
    enum_name: &str,
    minimum_variants: usize,
    exempt: &[&str],
) {
    let declaration = production_source(declared_in);
    let body = declaration
        .split(&format!("pub enum {enum_name} {{"))
        .nth(1)
        .unwrap_or_else(|| panic!("{enum_name} is declared in {declared_in}"));

    let variants: Vec<String> = body
        .lines()
        .take_while(|line| !line.starts_with('}'))
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .map(str::trim)
        .filter(|line| line.chars().next().is_some_and(char::is_uppercase))
        .map(|line| {
            line.trim_end_matches(',')
                .split([' ', '{', '(', '='])
                .next()
                .unwrap_or(line)
                .to_owned()
        })
        .collect();

    // The parse has to have worked. Without this, an enum that stopped being
    // found would report zero variants and pass — the failure mode this whole
    // family of guards exists to avoid.
    assert!(
        variants.len() >= minimum_variants,
        "the parse of {enum_name} found {} variants against a floor of \
         {minimum_variants}, which means it stopped reading the enum rather \
         than that the enum shrank",
        variants.len()
    );

    let elsewhere: String = production
        .iter()
        .filter(|file| **file != declared_in)
        .map(|file| production_source(file))
        .collect();

    for variant in &variants {
        if exempt.contains(&variant.as_str()) {
            continue;
        }
        assert!(
            elsewhere.contains(&format!("{enum_name}::{variant}")),
            "{enum_name}::{variant} is declared and nothing constructs it. A \
             variant a peer can never see is a check that is not there, and a \
             caller matching on it believes otherwise. Either produce it, \
             delete it, or list it as exempt with the argument written down."
        );
    }

    // And the exemption list must not name something that no longer exists.
    for name in exempt {
        assert!(
            variants.iter().any(|variant| variant == name),
            "{enum_name} has no variant {name}, but it is listed as exempt"
        );
    }
}

/// Whether `analysed` still carries the last non-empty line of `raw`.
///
/// Split out from the assertion so it can be tested directly. A check that only
/// exists inside an `assert!` can only be exercised by breaking the thing it
/// watches, and then it is covered by accident rather than on purpose.
fn analysis_reached_the_end(raw: &str, analysed: &str) -> bool {
    raw.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_none_or(|last| analysed.contains(last.trim()))
}

/// Fails if stripping ran off the end of the file.
///
/// The gate-stripper walks item by item, and an item shape it does not know
/// makes it consume everything after. The result still looks like source, still
/// passes every `contains` check, and quietly covers half of what it claims to.
/// This compares the last non-empty line of the raw file with what survived: if
/// the scan overran, that line is gone.
///
/// Written because it happened (QYR-0071) and nothing noticed for a sprint.
fn assert_analysis_reached_the_end(file: &str, analysed: &str) {
    let raw = production_source_raw(file);
    assert!(
        analysis_reached_the_end(&raw, analysed),
        "the gate analysis of src/{file} does not reach its last line. It read \
         {} of {} bytes, so the stripper met an item shape it does not know and \
         consumed the rest. Every guard built on this was covering less than it \
         claimed.",
        analysed.len(),
        raw.len()
    );
}

fn assert_no_production_path_can_panic(production: &[&str]) {
    for file in production {
        let source = production_source(file);
        assert_analysis_reached_the_end(file, &source);
        for forbidden in PROCESS_ENDING {
            assert!(
                !source.contains(forbidden),
                "src/{file} uses {forbidden} on the production path. Every input \
                 that reaches this crate is chosen by a peer, so ending the \
                 process is a remote denial of service. An invariant that can \
                 fail must return a typed error."
            );
        }
    }
}

/// Fails if a production file is missing from the list, or a listed file is
/// gated out of the build.
fn assert_the_production_list_matches_the_source(production: &[&str]) {
    let root = source_root();
    let mut files = Vec::new();
    collect_sources(&root, "", &mut files);
    let gated = gated_files();

    assert!(
        !files.is_empty(),
        "the walk found no source files, so every assertion below is vacuous"
    );

    for file in &files {
        if gated.contains(file) {
            continue;
        }
        assert!(
            production.contains(&file.as_str()),
            "src/{file} is compiled into a release build and no guard covers it. \
             Add it to PRODUCTION_FILES, or gate its module declaration with \
             #[cfg(test)] if it is a test file."
        );
    }

    for file in production {
        assert!(
            files.iter().any(|found| found == file),
            "PRODUCTION_FILES names src/{file}, which does not exist"
        );
        assert!(
            !gated.contains(*file),
            "src/{file} is listed as production but its module declaration is \
             gated out of release builds. One of the two is wrong."
        );
    }
}

#[test]
fn the_analysis_actually_strips() {
    // Without this, every assertion above could be passing because the analysis
    // silently produced an empty string. It runs once per crate that includes
    // this file, which is the point: each crate checks its own copy.
    let stripped = strip_test_only_items(
        "fn kept() {}\n#[cfg(test)]\nmod tests {\n    fn inner() { assert!(true); }\n}\nfn also_kept() {}\n",
    );
    assert!(stripped.contains("fn kept"), "production code survives");
    assert!(stripped.contains("fn also_kept"), "and so does what follows");
    assert!(!stripped.contains("assert!"), "the test body is gone");

    // An attribute between the gate and the declaration, which is how the AEAD
    // module opts its test children out of the lint.
    let attributed = strip_test_only_items(
        "#[cfg(test)]\n#[allow(clippy::unwrap_used)]\nmod corpus;\nfn kept() {}\n",
    );
    assert!(
        attributed.contains("fn kept"),
        "an attribute before the declaration must not swallow what follows"
    );

    // A semicolon inside a return type, before the body opens.
    let typed = strip_test_only_items(
        "#[cfg(test)]\nfn keys(&self) -> &[u8; 32] { self.0.expect(\"x\") }\nfn kept() {}\n",
    );
    assert!(typed.contains("fn kept"), "the body ends at its own brace");
    assert!(!typed.contains(".expect("), "and the body is gone");

    // A negated cfg is production and must survive untouched.
    let negated =
        strip_test_only_items("#[cfg(not(any(test, fuzzing)))]\nfn real() { }\nfn kept() {}\n");
    assert!(negated.contains("fn real"), "a negated cfg is production");
    assert!(negated.contains("fn kept"));

    // A brace inside a string literal must not unbalance the count.
    let quoted = strip_test_only_items(
        "#[cfg(test)]\nfn f() { let s = \"}\"; }\nfn kept() {}\nfn tail() { }\n",
    );
    assert!(quoted.contains("fn kept"), "a brace in a string is not a brace");
    assert!(quoted.contains("fn tail"));

    // Both spellings of a compile-time assertion are exempt, and only those.
    let item = strip_compile_time_assertions("const _: () = assert!(1 == 1);\nfn kept() {}\n");
    assert!(!item.contains("assert!("), "a const item assertion is stripped");
    assert!(item.contains("fn kept"));

    let inline = strip_compile_time_assertions("fn f<const N: usize>() { const { assert!(N > 0) }; }\n");
    assert!(!inline.contains("assert!("), "an inline const block is stripped");

    let runtime = strip_compile_time_assertions("fn f() { assert!(x); }\n");
    assert!(
        runtime.contains("assert!("),
        "a runtime assertion is not a const item and must survive to be caught"
    );

    assert_eq!(strip_comments("// panic!\nlet x = 1;"), "let x = 1;");

    // And the module-gate derivation, which is what makes the exemptions real.
    assert_eq!(
        gated_child_modules("#[cfg(test)]\nmod schema;\npub mod aead;\n"),
        vec!["schema".to_owned()],
        "a gated declaration is found and an ungated one is not"
    );
    assert!(
        gated_child_modules("mod schema;\n").is_empty(),
        "removing the gate removes the exemption"
    );
    // A `#[cfg(test)]` struct field ends at a comma and has neither a body nor
    // a semicolon. Before QYR-0071 the scan ran past the struct and consumed
    // the rest of the file.
    let field = strip_test_only_items(
        "struct S {\n    a: u8,\n    #[cfg(test)]\n    gated: usize,\n    b: u8,\n}\nfn after() {}\n",
    );
    assert!(!field.contains("gated"), "a gated field is stripped");
    assert!(
        field.contains("fn after"),
        "stripping a gated field ate what followed it, which is QYR-0071"
    );
    assert!(field.contains("b: u8"), "it ate the rest of the struct too");

    // And the overrun check itself, tested directly rather than by breaking the
    // stripper and hoping something notices.
    assert!(analysis_reached_the_end("one\ntwo\n", "one\ntwo\n"));
    assert!(
        !analysis_reached_the_end("one\ntwo\n", "one\n"),
        "an analysis missing the file's last line must be reported"
    );
    assert!(
        analysis_reached_the_end("one\ntwo\n\n\n", "one\ntwo\n"),
        "trailing blank lines are not the last line"
    );

    assert_eq!(module_directory("lib.rs"), "");
    assert_eq!(module_directory("aead/mod.rs"), "aead");
    assert_eq!(module_directory("identity.rs"), "identity");
}
