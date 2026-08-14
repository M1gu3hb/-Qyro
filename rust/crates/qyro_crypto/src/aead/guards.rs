//! Structural guards over the AEAD module's own production source.
//!
//! Two properties here cannot be expressed as ordinary tests. A third — that no
//! production path can end the process — moved to `crate::guards`, which now
//! makes the same assertion over every production file in the crate rather than
//! over the three in this directory.
//!
//! *Plaintext lives in a zeroizing container.* A test can check that a byte
//! slice compares equal; it cannot check that the allocation behind it is wiped
//! when it drops, because reading freed memory is undefined behaviour and an
//! allocator may reuse or unmap the page. What can be checked is the type, and
//! the type is what carries the guarantee.
//!
//! *No accessor hands the plaintext out unprotected.* `into_payload(self) ->
//! Vec<u8>` would silently drop the guarantee at the one call that matters.
//!
//! The analysis is deliberately narrow. It reads only this module's production
//! files, with `crate::guards` doing the stripping, and never guesses: anything
//! it cannot account for makes it fail rather than pass.

/// Reads one of this module's production files with everything non-production
/// removed.
///
/// The stripping lives in `crate::guards`, which owns the crate-wide anti-panic
/// guard. It used to be duplicated here, and a duplicated analysis is two
/// analyses that can disagree about what counts as production code.
///
/// `tests.rs`, `vectors.rs` and `corpus.rs` never appear below: they are
/// `#[cfg(test)]` in their entirety and may assert freely.
fn production_source(file: &str) -> String {
    crate::guards::production_source(&format!("aead/{file}"))
}

#[test]
fn every_committed_vector_arrives_with_the_bytes_that_were_committed() {
    // Found by running this suite on Windows for the first time. Git's default
    // there checks text files out with CRLF, so `include_str!` returned
    // different bytes than the ones that had been committed, and three tests
    // failed with diffs that looked like a stale vector rather than a
    // translated file.
    //
    // It matters beyond the tests. These files are the interoperability
    // contract: an implementation in Swift or Kotlin is meant to hash exactly
    // these bytes, and a file that arrives different on Windows is a different
    // file. `.gitattributes` pins `eol=lf`; this fails with a sentence that
    // says so if anyone removes it.
    for (name, contents) in [
        ("aead-v1.json", super::vectors::COMMITTED),
        (
            "handshake-v1.json",
            include_str!("../../../../../docs/security/test-vectors/handshake-v1.json"),
        ),
        (
            "rfc8439-chacha20poly1305.json",
            include_str!("../../../../../docs/security/test-vectors/rfc8439-chacha20poly1305.json"),
        ),
    ] {
        assert!(
            !contents.contains('\r'),
            "{name} was checked out with CRLF; the committed vectors are \
             byte-exact and .gitattributes must pin eol=lf"
        );
    }
}

#[test]
fn verified_plaintext_lives_in_a_zeroizing_container() {
    let source = production_source("mod.rs");

    assert!(
        source.contains("payload: Zeroizing<Vec<u8>>"),
        "AuthenticatedFrame must store its plaintext in a zeroizing container; \
         a plain Vec leaves verified plaintext in freed memory"
    );
    assert!(
        !source.contains("fn into_payload(self) -> Vec<u8>"),
        "into_payload would drop the zeroization guarantee at the one call that \
         matters; hand out the zeroizing container instead"
    );
    assert!(
        source.contains("fn into_zeroizing_payload"),
        "there must be a way to take ownership that keeps the guarantee"
    );
}

/// The accessor whose result must stay wrapped, wherever it is consumed.
const ZEROIZING_ACCESSOR: &str = "into_zeroizing_payload";

/// Chained calls that are permitted to follow it, each with a reason.
///
/// Empty on purpose today. This is a **deny-by-shape** list, not the
/// allow-list-disguised-as-a-deny-list that QYR-0053 describes: anything not
/// named here fails, so a method invented next year fails too, and adding an
/// entry costs an argument in this file rather than silence somewhere else.
const CHAINS_ALLOWED_AFTER_THE_ACCESSOR: [&str; 0] = [];

#[test]
fn no_consumer_unwraps_the_plaintext_out_of_its_wipe() {
    // QYR-0304, and the reason it needed reopening: the guard above forbids
    // `into_payload` **by name**, and was blind to `.to_vec()` on its
    // replacement -- in another crate, where nothing owned by qyro_crypto could
    // look. Undoing the fix left 592 tests green.
    //
    // So this reads the *consumers*, and checks a shape rather than a name:
    // whatever `into_zeroizing_payload` returns must be bound or returned, never
    // method-chained. `.to_vec()`, `.clone()`, `.to_owned()` and anything not
    // yet invented all fail the same way, because the rule is "no chain" and not
    // "not these three".
    let crates_root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let mut call_sites = 0_usize;
    let mut offences = Vec::new();

    for path in crate::guards::rust_files_under(crates_root) {
        // This crate owns the accessor and defines it; the rule is about who
        // *consumes* it.
        //
        // Matched on `/qyro_crypto/src/` and not on `/qyro_crypto/`, because
        // `crates_root` ends in `/..` and therefore every path under it carries
        // this crate's name as a component. The first draft skipped **every**
        // file that way and found zero call sites — caught by the
        // `call_sites > 0` assertion below, on its first run, which is the whole
        // reason that assertion exists.
        if path.replace('\\', "/").contains("/qyro_crypto/src/") {
            continue;
        }
        let source = crate::guards::production_source_at(&path);
        for (index, _) in source.match_indices(ZEROIZING_ACCESSOR) {
            let Some(rest) = source.get(index + ZEROIZING_ACCESSOR.len()..) else {
                continue;
            };
            // A definition or an import is not a call site.
            if !rest.starts_with("()") {
                continue;
            }
            call_sites += 1;
            let after = rest.get(2..).unwrap_or_default().trim_start();
            if !after.starts_with('.') {
                continue;
            }
            let chained: String = after
                .chars()
                .skip(1)
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if CHAINS_ALLOWED_AFTER_THE_ACCESSOR.contains(&chained.as_str()) {
                continue;
            }
            offences.push(format!("{path}: .{chained}()"));
        }
    }

    assert!(
        offences.is_empty(),
        "these call sites chain onto {ZEROIZING_ACCESSOR}, which is how verified \
         plaintext leaves the container that wipes it: {offences:?}"
    );

    // Without this the guard passes on a repository where the accessor is
    // renamed, or where every consumer disappeared -- which is precisely the
    // state it must not silently accept. It is the same idea as
    // `a_descriptor_leak_would_be_visible_to_this_measurement`: a measurement
    // that cannot fail is not evidence.
    assert!(
        call_sites > 0,
        "no consumer of {ZEROIZING_ACCESSOR} was found outside qyro_crypto, so \
         this analysis is not reading what it thinks it is"
    );
}

#[test]
fn the_temporary_buffers_are_zeroizing_too() {
    // The buffer inside `seal` starts as plaintext, and the buffer inside
    // `open` becomes plaintext the moment the tag verifies. Both outlive their
    // usefulness by one drop, and both are the obvious place for plaintext to
    // survive an early return.
    let source = production_source("mod.rs");
    let occurrences = source.matches("Zeroizing::new(").count();
    assert!(
        occurrences >= 3,
        "expected zeroizing temporaries in seal and open plus the derived key, \
         found {occurrences} constructions"
    );
}
