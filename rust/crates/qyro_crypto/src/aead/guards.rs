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
