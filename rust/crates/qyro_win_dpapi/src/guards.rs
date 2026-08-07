// The unsafe surface of this crate, enumerated.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the analysis reads files and must fail loudly when it cannot"
)]

/// Every function in this crate permitted to contain an `unsafe` block.
///
/// **By containing function, not by count.** A count lets one block be swapped
/// for another without the number moving, and the swap is the change worth
/// catching. ADR-0024 §1 stakes the whole hand-written-`extern` argument on this
/// surface staying enumerable in a sentence.
///
/// Written while the list was empty and before a single block existed, so that
/// the first ones turned it red. That ordering is the point: a guard added after
/// the code it guards has never demonstrated it can fail.
///
/// Three, and each does one thing:
/// - `ffi.rs::take_and_free` copies a DPAPI output out, wipes it, frees it;
/// - `store.rs::wrap` calls `CryptProtectData`;
/// - `store.rs::unwrap` calls `CryptUnprotectData`.
///
/// The `GetLastError` reads sit inside the last two, in the failure branch, so
/// they are not separate entries.
const FUNCTIONS_ALLOWED_UNSAFE: [&str; 3] = [
    "ffi.rs::take_and_free",
    "store.rs::unwrap",
    "store.rs::wrap",
];

/// Reads this crate's production sources.
fn production_source() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for name in ["lib.rs", "ffi.rs", "store.rs"] {
        let path = dir.join(name);
        if let Ok(text) = std::fs::read_to_string(&path) {
            out.push((name.to_owned(), text));
        }
    }
    assert!(!out.is_empty(), "no production source found under {dir:?}");
    out
}

/// Finds the function each `unsafe {` block sits inside.
///
/// Textual, and deliberately conservative: it walks backwards from the block to
/// the nearest preceding `fn` declaration. A block that cannot be attributed to
/// a function is reported as `<unattributed>` and fails, rather than being
/// skipped — a guard that silently ignores what it cannot parse reports success
/// about a thing it never checked.
fn functions_containing_unsafe_blocks() -> Vec<String> {
    let mut found = Vec::new();
    for (file, source) in production_source() {
        for (offset, _) in source.match_indices("unsafe {") {
            let before = &source[..offset];
            let name = before
                .rfind("fn ")
                .and_then(|at| before.get(at + 3..))
                .and_then(|rest| rest.split(['(', '<', ' ', '\n']).next())
                .filter(|name| !name.is_empty())
                .unwrap_or("<unattributed>");
            found.push(format!("{file}::{name}"));
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn the_unsafe_blocks_are_the_ones_we_listed() {
    let found = functions_containing_unsafe_blocks();
    let allowed: Vec<&str> = FUNCTIONS_ALLOWED_UNSAFE.to_vec();

    let unlisted: Vec<&String> = found
        .iter()
        .filter(|f| !allowed.contains(&f.as_str()))
        .collect();
    assert!(
        unlisted.is_empty(),
        "these functions contain an `unsafe` block and are not listed: \
         {unlisted:?}\n\
         Every one is a widening of the surface ADR-0024 §1 promised would stay \
         enumerable in a sentence. Add it here and say why in the ADR."
    );

    for name in &allowed {
        assert!(
            found.iter().any(|f| f == name),
            "{name} is listed as containing an `unsafe` block and does not. A \
             list that outlives its reason stops meaning anything."
        );
    }
}
