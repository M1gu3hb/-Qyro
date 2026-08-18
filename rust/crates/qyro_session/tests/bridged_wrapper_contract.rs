//! The bridged wrapper, against a double that behaves like a hostile platform.
//!
//! ADR-0037. What is being checked is not "does AES work" — the platform does
//! that — but that this side survives a far side which lies about lengths,
//! refuses, or returns another domain's blob.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use qyro_identity_store::{SecretWrapper, entropy_for};
use qyro_session::{BRIDGED_WRAP_ID, BridgedWrapper, DOMAIN_MISMATCH, MAX_WRAPPED_LEN};

/// What the double should do.
///
/// Carried in the wrapper's own `context` and **not** in a global: these tests
/// run in parallel, and a shared mutable behaviour is a test that fails
/// depending on which other test happened to be running. `context` is exactly
/// the opaque value the bridge promises never to look inside, so this is also
/// the contract being exercised.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behaviour {
    /// Reverse the bytes. Not encryption, and not meant to be: what this test
    /// is about is the protocol between the two sides.
    Honest,
    /// Refuse with a platform code.
    Refuse(i32),
    /// Report a length larger than the ceiling.
    Preposterous,
    /// Report zero bytes needed and succeed.
    Empty,
    /// Claim to have written more than the buffer holds.
    Overclaim,
}

impl Behaviour {
    const fn to_context(self) -> usize {
        match self {
            Self::Honest => 0,
            Self::Preposterous => 1,
            Self::Empty => 2,
            Self::Overclaim => 3,
            // Offset so a code of 0 stays distinguishable from `Honest`.
            Self::Refuse(code) => 16_usize.wrapping_add(code as usize),
        }
    }

    const fn from_context(context: usize) -> Self {
        match context {
            0 => Self::Honest,
            1 => Self::Preposterous,
            2 => Self::Empty,
            3 => Self::Overclaim,
            other => Self::Refuse(other.wrapping_sub(16) as i32),
        }
    }
}

/// The far side. Reverses the bytes, or misbehaves as its context says.
extern "C" fn transform(
    context: usize,
    input: *const u8,
    input_len: usize,
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> i32 {
    let bytes = if input.is_null() {
        Vec::new()
    } else {
        // SAFETY: the bridge promises `input_len` readable bytes at `input`,
        // and this double is the caller that contract is written for.
        unsafe { std::slice::from_raw_parts(input, input_len) }.to_vec()
    };

    match Behaviour::from_context(context) {
        Behaviour::Refuse(code) => {
            unsafe { out_len.write(bytes.len()) };
            code
        }
        Behaviour::Preposterous => {
            unsafe { out_len.write(MAX_WRAPPED_LEN + 1) };
            0
        }
        Behaviour::Empty => {
            unsafe { out_len.write(0) };
            0
        }
        Behaviour::Overclaim => {
            unsafe { out_len.write(out_cap.saturating_add(1)) };
            0
        }
        Behaviour::Honest => {
            unsafe { out_len.write(bytes.len()) };
            if bytes.len() > out_cap || out.is_null() {
                // Reporting the length with no room is the ask half of the
                // protocol, and it is not an error.
                return 0;
            }
            let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
            unsafe { std::ptr::copy_nonoverlapping(reversed.as_ptr(), out, reversed.len()) };
            0
        }
    }
}

fn wrapper_that(behaviour: Behaviour) -> BridgedWrapper {
    BridgedWrapper::new(transform, transform, behaviour.to_context())
}

fn wrapper() -> BridgedWrapper {
    wrapper_that(Behaviour::Honest)
}

#[test]
fn a_secret_survives_the_round_trip_through_the_bridge() {
    let entropy = entropy_for(1, BRIDGED_WRAP_ID);
    let secret = b"a device identity seed, thirty-two bytes long!!!!";

    let wrapped = wrapper().wrap(secret, &entropy).expect("wrapping refused");
    // The far side saw the domain *and* the secret, which is what makes the
    // separation below possible at all.
    assert!(wrapped.len() > secret.len(), "the domain did not travel");

    let opened = wrapper()
        .unwrap(&wrapped, &entropy)
        .expect("unwrapping refused");
    assert_eq!(&opened[..], &secret[..]);
}

#[test]
fn a_blob_from_another_entropy_domain_is_refused_and_not_returned() {
    // The whole reason the domain is prepended. An identity blob opened as a
    // peer store is exactly what this stops, and the refusal has to be a
    // refusal — a truncated body handed back would be worse than an error.
    let identity_domain = entropy_for(1, BRIDGED_WRAP_ID);
    let peers_domain = entropy_for(2, BRIDGED_WRAP_ID);
    assert_ne!(
        identity_domain, peers_domain,
        "the two domains are equal, so this test cannot tell them apart"
    );

    let wrapped = wrapper().wrap(b"seed", &identity_domain).unwrap();

    // The control: the right domain opens it.
    assert!(wrapper().unwrap(&wrapped, &identity_domain).is_ok());

    let refused = wrapper().unwrap(&wrapped, &peers_domain);
    match refused {
        Err(qyro_identity_store::StoreError::Unwrap { code }) => {
            assert_eq!(code, DOMAIN_MISMATCH, "the refusal came from the platform");
        }
        other => panic!("the wrong domain produced {other:?}"),
    }
}

#[test]
fn a_far_side_that_lies_about_lengths_is_not_believed() {
    let entropy = entropy_for(1, BRIDGED_WRAP_ID);

    for behaviour in [
        Behaviour::Preposterous,
        Behaviour::Empty,
        Behaviour::Overclaim,
        Behaviour::Refuse(-7),
        Behaviour::Refuse(5),
    ] {
        assert!(
            wrapper_that(behaviour).wrap(b"seed", &entropy).is_err(),
            "{behaviour:?} was believed"
        );
    }

    // The control: the same bridge with an honest far side works, so the five
    // refusals are about the behaviour and not about a wrapper that refuses
    // everything.
    assert!(wrapper().wrap(b"seed", &entropy).is_ok());
}

#[test]
fn the_platform_code_survives_as_the_number_it_was() {
    // `StoreError::Unwrap` carries "the platform's own code" and a report is
    // supposed to be able to say which failure it was. A clamp would fold every
    // negative into one number.
    let entropy = entropy_for(1, BRIDGED_WRAP_ID);
    let minus_seven = wrapper_that(Behaviour::Refuse(-7)).wrap(b"seed", &entropy);
    let minus_eight = wrapper_that(Behaviour::Refuse(-8)).wrap(b"seed", &entropy);

    assert!(minus_seven.is_err() && minus_eight.is_err());
    assert_ne!(
        format!("{minus_seven:?}"),
        format!("{minus_eight:?}"),
        "two different platform codes reported the same error, so a bug report \
         cannot say which one happened"
    );
}

#[test]
fn the_wrap_byte_is_not_the_one_another_platform_writes() {
    // A blob wrapped by DPAPI must not open through this bridge, and the header
    // byte is what turns that into a refusal instead of a garbled read. DPAPI's
    // is 1; this must not be 1, and must not be 0 either, which is what an
    // uninitialised byte looks like.
    assert_eq!(BRIDGED_WRAP_ID, 3);
    assert_ne!(BRIDGED_WRAP_ID, 0);
    assert_ne!(BRIDGED_WRAP_ID, 1);
    assert_eq!(wrapper().wrap_id(), BRIDGED_WRAP_ID);
}
