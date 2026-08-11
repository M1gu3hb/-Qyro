// Source analysis shared by six structural-guard modules.
//
// **Not a crate.** This file is pulled into `qyro_crypto`, `qyro_protocol`,
// `qyro_manifest`, `qyro_identity_store`, `qyro_fs` and `qyro_transfer` with
// `include!`, from each crate's `src/guards.rs`:
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

/// Legitimate identical comparisons, named narrowly and argued individually.
///
/// Tuple fields are crate, crate-relative file, whitespace-normalised side and
/// the written reason. There is deliberately no crate-wide or file-wide allow:
/// a second identical assertion must make the guard fail on its own merits.
const IDENTICAL_ASSERTIONS_WITH_A_WRITTEN_ARGUMENT: [(&str, &str, &str, &str); 0] = [];

/// Returns the byte after a literal or comment that starts at `index`.
///
/// This is not a Rust parser. It only keeps assertion delimiters and commas
/// inside strings/comments from being mistaken for syntax, which is the narrow
/// grammar this guard needs.
fn non_code_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let length = bytes.len();

    if bytes.get(index..index + 2) == Some(b"//") {
        return Some(
            bytes[index + 2..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(length, |offset| index + 2 + offset + 1),
        );
    }
    if bytes.get(index..index + 2) == Some(b"/*") {
        let mut depth = 1usize;
        let mut cursor = index + 2;
        while cursor < length {
            if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                depth += 1;
                cursor += 2;
            } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            } else {
                cursor += 1;
            }
        }
        return Some(length);
    }

    // Raw strings: r"...", r#"..."#, br"..." and br#"..."#.
    let raw_start = if bytes.get(index) == Some(&b'r') {
        Some(index + 1)
    } else if bytes.get(index..index + 2) == Some(b"br") {
        Some(index + 2)
    } else {
        None
    };
    if let Some(mut quote) = raw_start {
        let mut hashes = 0usize;
        while bytes.get(quote) == Some(&b'#') {
            hashes += 1;
            quote += 1;
        }
        if bytes.get(quote) == Some(&b'"') {
            let mut cursor = quote + 1;
            while cursor < length {
                if bytes.get(cursor) == Some(&b'"')
                    && bytes.get(cursor + 1..cursor + 1 + hashes)
                        .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
                {
                    return Some(cursor + 1 + hashes);
                }
                cursor += 1;
            }
            return Some(length);
        }
    }

    let quote = if bytes.get(index) == Some(&b'"') {
        Some(index)
    } else if bytes.get(index..index + 2) == Some(b"b\"") {
        Some(index + 1)
    } else {
        None
    };
    if let Some(quote) = quote {
        let mut cursor = quote + 1;
        while cursor < length {
            match bytes[cursor] {
                b'\\' => cursor += 2,
                b'"' => return Some(cursor + 1),
                _ => cursor += 1,
            }
        }
        return Some(length);
    }

    // A lifetime has no apostrophe immediately after its identifier; a
    // character literal does. Requiring the close at the exact scalar boundary
    // stops two lifetimes on one line from hiding everything between them.
    if bytes.get(index) == Some(&b'\'') {
        if bytes.get(index + 1) == Some(&b'\\') {
            let mut cursor = index + 2;
            while cursor < length && bytes[cursor] != b'\n' {
                match bytes[cursor] {
                    b'\\' => cursor += 2,
                    b'\'' => return Some(cursor + 1),
                    _ => cursor += 1,
                }
            }
        } else if let Some(character) = source[index + 1..].chars().next() {
            let close = index + 1 + character.len_utf8();
            if bytes.get(close) == Some(&b'\'') {
                return Some(close + 1);
            }
        }
    }
    None
}

/// Closing parenthesis for the one at `open`, ignoring nested Rust delimiters.
fn closing_parenthesis(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut round = 1usize;
    let mut cursor = open + 1;
    while cursor < bytes.len() {
        if let Some(end) = non_code_end(source, cursor) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'(' => round += 1,
            b')' => {
                round -= 1;
                if round == 0 {
                    return Some(cursor);
                }
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

/// Splits at top-level commas while preserving nested calls and literals.
fn assertion_arguments(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut arguments = Vec::new();
    let mut start = 0usize;
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if let Some(end) = non_code_end(source, cursor) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => curly += 1,
            b'}' => curly = curly.saturating_sub(1),
            b',' if round == 0 && square == 0 && curly == 0 => {
                arguments.push(&source[start..cursor]);
                start = cursor + 1;
            }
            _ => {}
        }
        cursor += 1;
    }
    arguments.push(&source[start..]);
    arguments
}

fn without_wrapping_parentheses(mut expression: &str) -> &str {
    loop {
        let trimmed = expression.trim();
        if !trimmed.starts_with('(')
            || closing_parenthesis(trimmed, 0) != Some(trimmed.len() - 1)
        {
            return trimmed;
        }
        expression = &trimmed[1..trimmed.len() - 1];
    }
}

/// The two operands of a top-level `==` or `!=` expression.
fn comparison_sides(expression: &str) -> Option<(&str, &str)> {
    let expression = without_wrapping_parentheses(expression);
    let bytes = expression.as_bytes();
    let mut round = 0usize;
    let mut square = 0usize;
    let mut curly = 0usize;
    let mut cursor = 0usize;

    while cursor + 1 < bytes.len() {
        if let Some(end) = non_code_end(expression, cursor) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'(' => round += 1,
            b')' => round = round.saturating_sub(1),
            b'[' => square += 1,
            b']' => square = square.saturating_sub(1),
            b'{' => curly += 1,
            b'}' => curly = curly.saturating_sub(1),
            b'=' | b'!'
                if bytes[cursor + 1] == b'='
                    && round == 0
                    && square == 0
                    && curly == 0 =>
            {
                return Some((&expression[..cursor], &expression[cursor + 2..]));
            }
            _ => {}
        }
        cursor += 1;
    }
    None
}

fn normalise_assertion_side(side: &str) -> String {
    side.chars().filter(|character| !character.is_whitespace()).collect()
}

/// The macro name and opening parenthesis at `index`, if one starts there.
fn assertion_macro_at(source: &str, index: usize) -> Option<(&'static str, usize)> {
    const MACROS: [&str; 3] = ["assert_eq!", "assert_ne!", "assert!"];
    let bytes = source.as_bytes();
    if index > 0 && (bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_') {
        return None;
    }
    for name in MACROS {
        if bytes.get(index..index + name.len()) != Some(name.as_bytes()) {
            continue;
        }
        let mut open = index + name.len();
        while bytes.get(open).is_some_and(u8::is_ascii_whitespace) {
            open += 1;
        }
        if bytes.get(open) == Some(&b'(') {
            return Some((name, open));
        }
    }
    None
}

/// Returns `(line, repeated_side)` for identical assertion comparisons.
fn tautological_assertions(source: &str) -> Vec<(usize, String)> {
    let bytes = source.as_bytes();
    let mut findings = Vec::new();
    let mut cursor = 0usize;

    while cursor < bytes.len() {
        if let Some(end) = non_code_end(source, cursor) {
            cursor = end;
            continue;
        }
        let Some((name, open)) = assertion_macro_at(source, cursor) else {
            cursor += 1;
            continue;
        };
        let Some(close) = closing_parenthesis(source, open) else {
            break;
        };
        let body = &source[open + 1..close];
        let arguments = assertion_arguments(body);
        let sides = if name == "assert!" {
            arguments.first().and_then(|argument| comparison_sides(argument))
        } else {
            arguments
                .first()
                .zip(arguments.get(1))
                .map(|(left, right)| (*left, *right))
        };
        if let Some((left, right)) = sides {
            let left = normalise_assertion_side(left);
            let right = normalise_assertion_side(right);
            if !left.is_empty() && left == right {
                let line = source[..cursor].bytes().filter(|byte| *byte == b'\n').count() + 1;
                findings.push((line, left));
            }
        }
        cursor = close + 1;
    }
    findings
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

/// Fails if a test assertion compares textually identical operands.
///
/// The scan intentionally recognises only the two shapes that have repeatedly
/// hidden missing evidence in this repository: `assert!(X == X)` (or `!=`) and
/// `assert_eq!(X, X)`/`assert_ne!(X, X)`. It normalises whitespace, not Rust
/// semantics. Expanding this into a general expression equivalence checker
/// would make a small, auditable guard guess about the language.
#[test]
fn assert_no_assertion_compares_a_call_to_itself() {
    let crate_name = env!("CARGO_PKG_NAME");
    let manifest_root = env!("CARGO_MANIFEST_DIR");
    let src_root = format!("{manifest_root}/src");
    let mut src_files = Vec::new();
    collect_sources(&src_root, "", &mut src_files);
    let gated = gated_files();

    let mut test_sources: Vec<(String, String)> = Vec::new();
    for file in src_files {
        let source = fs::read_to_string(format!("{src_root}/{file}"))
            .unwrap_or_else(|error| panic!("src/{file}: {error}"));
        if gated.contains(&file) || GATE_MARKERS.iter().any(|marker| source.contains(marker)) {
            test_sources.push((format!("src/{file}"), source));
        }
    }

    let integration_root = format!("{manifest_root}/tests");
    if std::path::Path::new(&integration_root).is_dir() {
        let mut integration_files = Vec::new();
        collect_sources(&integration_root, "", &mut integration_files);
        for file in integration_files {
            let source = fs::read_to_string(format!("{integration_root}/{file}"))
                .unwrap_or_else(|error| panic!("tests/{file}: {error}"));
            test_sources.push((format!("tests/{file}"), source));
        }
    }

    let crate_exemptions: Vec<(usize, &str, &str, &str)> =
        IDENTICAL_ASSERTIONS_WITH_A_WRITTEN_ARGUMENT
            .iter()
            .enumerate()
            .filter(|(_, (exempt_crate, _, _, _))| *exempt_crate == crate_name)
            .map(|(index, (_, file, side, reason))| (index, *file, *side, *reason))
            .collect();
    for (_, file, side, reason) in &crate_exemptions {
        assert!(
            !file.is_empty() && !side.is_empty() && !reason.trim().is_empty(),
            "an exemption for an identical assertion must name its file and \
             normalised operand and carry a written argument"
        );
    }

    let mut matched_exemptions = BTreeSet::new();
    let mut failures = Vec::new();
    for (file, source) in test_sources {
        for (line, side) in tautological_assertions(&source) {
            let exemption = crate_exemptions.iter().find(
                |(index, exempt_file, exempt_side, _)| {
                    !matched_exemptions.contains(index)
                        && *exempt_file == file
                        && *exempt_side == side
                },
            );
            if let Some((index, _, _, _)) = exemption {
                matched_exemptions.insert(*index);
            } else {
                failures.push(format!("{file}:{line}: `{side}` is compared with itself"));
            }
        }
    }

    for (index, file, side, _) in crate_exemptions {
        assert!(
            matched_exemptions.contains(&index),
            "the exemption for {file} operand `{side}` no longer matches an \
             assertion; remove the stale exemption"
        );
    }

    assert!(
        failures.is_empty(),
        "test assertions must compare independently derived values. Identical \
         text after whitespace normalisation is the five-times-repeated \
         anti-pattern this guard closes:\n{}",
        failures.join("\n")
    );
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

    assert_eq!(
        tautological_assertions("assert!(read() == read());"),
        vec![(1, "read()".to_owned())],
        "assert!(X == X) is the original anti-pattern"
    );
    assert_eq!(
        tautological_assertions("assert_eq!(frame.header(), frame . header());"),
        vec![(1, "frame.header()".to_owned())],
        "assert_eq!(X, X) is identical after whitespace normalization"
    );
    assert_eq!(
        tautological_assertions(
            "assert_ne!(digest(bytes, Domain::A), digest(bytes, Domain::A), \"must differ\");"
        ),
        vec![(1, "digest(bytes,Domain::A)".to_owned())],
        "assert_ne! is subject to the same two-independent-sides contract"
    );
    assert!(
        tautological_assertions("assert_ne!(left(), right());").is_empty(),
        "different calls are evidence rather than a tautology"
    );
    assert!(
        tautological_assertions(
            "// assert_eq!(hidden(), hidden())\nlet prose = \"assert!(also_hidden() == also_hidden())\";"
        )
        .is_empty(),
        "comments and string contents are not assertions"
    );
    assert_eq!(
        tautological_assertions(
            "fn f<'a>() { assert_eq!(read(), read()); let _: &'a str = \"x\"; }"
        ),
        vec![(1, "read()".to_owned())],
        "two lifetimes on one line must not hide the assertion between them"
    );
}
