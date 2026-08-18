//! A secret wrapper that lives on the other side of the C boundary.
//!
//! Specification: `docs/adr/ADR-0037-android-keystore-bridge.md`.
//!
//! # Why the wrapper crosses instead of the JVM
//!
//! There is **no Keystore API in the NDK**, so Rust cannot reach it. The plan
//! for this phase said `jni-sys` and it was pre-authorised; ADR-0037 takes the
//! third option neither side of that argument had considered — **do not do JNI
//! at all**. Kotlin implements two functions, they cross as pointers, and what
//! travels between them is bytes.
//!
//! Zero new dependencies, zero new `unsafe` outside `qyro_ffi`, and the
//! `Context` that Keystore demands stays on the side that owns one.
//!
//! # What does not cross
//!
//! **Dart never sees a secret.** These pointers are called *by Rust*, not by
//! Dart: Dart registers them once and is never handed a plaintext. The value
//! that comes back is already wrapped by the platform.

use qyro_identity_store::{SecretWrapper, StoreError};
use zeroize::Zeroizing;

/// The wrap byte a bridged wrapper writes. Distinct from DPAPI's.
///
/// A blob wrapped on one platform must not open as one from another, and the
/// header byte is what makes that a refusal rather than a garbled read.
pub const BRIDGED_WRAP_ID: u8 = 3;

/// What the platform side must provide.
///
/// Two functions and an opaque context. The context is whatever the far side
/// needs to find itself — a JNI global reference, a channel id — and this crate
/// never looks inside it.
///
/// # Safety of the contract, stated once
///
/// Both functions receive `(context, input, input_len, out, out_cap, out_len)`
/// and return `0` on success. They must write at most `out_cap` bytes, must
/// always set `out_len` to the length they needed, and must not retain the input
/// pointer after returning. `qyro_ffi` is where that contract is checked against
/// a real caller; here it is a type.
pub type WrapFn = extern "C" fn(
    context: usize,
    input: *const u8,
    input_len: usize,
    out: *mut u8,
    out_cap: usize,
    out_len: *mut usize,
) -> i32;

/// A [`SecretWrapper`] backed by two foreign functions.
///
/// Holds no secret of its own: it is a pair of pointers and a number.
pub struct BridgedWrapper {
    wrap: WrapFn,
    unwrap: WrapFn,
    context: usize,
    /// The largest wrapped blob this will accept, in bytes.
    ///
    /// A ceiling and not a growing loop: the far side reports the length it
    /// needs, and a caller that reported a preposterous one would otherwise get
    /// an allocation of that size out of this process. The identity blob is
    /// hundreds of bytes and the known-peer store is bounded by its own format,
    /// so 64 KiB is generous by three orders of magnitude.
    ceiling: usize,
}

/// The ceiling above, named so a test can assert on it rather than on a literal.
pub const MAX_WRAPPED_LEN: usize = 64 * 1024;

impl BridgedWrapper {
    #[must_use]
    pub const fn new(wrap: WrapFn, unwrap: WrapFn, context: usize) -> Self {
        Self {
            wrap,
            unwrap,
            context,
            ceiling: MAX_WRAPPED_LEN,
        }
    }

    /// Calls one of the two functions with the ask-then-fill protocol.
    fn call(&self, function: WrapFn, input: &[u8]) -> Result<Vec<u8>, StoreError> {
        let mut needed: usize = 0;
        // First call asks. A null buffer with zero capacity cannot be written
        // to, so a far side that ignored the capacity still cannot corrupt this
        // one — it can only report a length.
        let asked = function(
            self.context,
            input.as_ptr(),
            input.len(),
            std::ptr::null_mut(),
            0,
            &raw mut needed,
        );
        if needed == 0 {
            // Nothing to return and no error is not a valid answer: an empty
            // wrapped blob would unwrap to nothing and look like success.
            return Err(StoreError::Unwrap {
                code: platform_code(asked),
            });
        }
        if needed > self.ceiling {
            return Err(StoreError::Unwrap {
                code: platform_code(asked),
            });
        }

        let mut out = vec![0_u8; needed];
        let mut wrote: usize = 0;
        let code = function(
            self.context,
            input.as_ptr(),
            input.len(),
            out.as_mut_ptr(),
            out.len(),
            &raw mut wrote,
        );
        if code != 0 {
            return Err(StoreError::Unwrap {
                code: platform_code(code),
            });
        }
        if wrote > out.len() {
            // The far side claims to have written past the end of what it was
            // given. Nothing here can undo that, but it must not be believed:
            // truncating to the buffer would hand back bytes the caller did not
            // produce.
            return Err(StoreError::Unwrap {
                code: platform_code(code),
            });
        }
        out.truncate(wrote);
        Ok(out)
    }
}

impl SecretWrapper for BridgedWrapper {
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        // The entropy domain is prepended rather than passed alongside, because
        // the far side is a platform API with room for one input. Prepending it
        // means a blob wrapped under one domain cannot be unwrapped under
        // another without the length check below failing.
        let mut joined = Vec::with_capacity(entropy.len() + secret.len() + 2);
        joined.extend_from_slice(&domain_header(entropy.len())?);
        joined.extend_from_slice(entropy);
        joined.extend_from_slice(secret);
        self.call(self.wrap, &joined)
    }

    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        let opened = Zeroizing::new(self.call(self.unwrap, wrapped)?);
        let header = domain_header(entropy.len())?;
        let prefix_len = header.len().saturating_add(entropy.len());
        let Some(body) = opened.get(prefix_len..) else {
            return Err(StoreError::Unwrap {
                code: DOMAIN_MISMATCH,
            });
        };
        let Some(found) = opened.get(header.len()..prefix_len) else {
            return Err(StoreError::Unwrap {
                code: DOMAIN_MISMATCH,
            });
        };
        if opened.get(..header.len()) != Some(&header[..]) || found != entropy {
            // Wrapped under a different domain. Refused rather than returned:
            // an identity blob opened as a peer store is exactly what the domain
            // separation exists to stop.
            return Err(StoreError::Unwrap {
                code: DOMAIN_MISMATCH,
            });
        }
        Ok(Zeroizing::new(body.to_vec()))
    }

    fn wrap_id(&self) -> u8 {
        BRIDGED_WRAP_ID
    }
}

/// The code this crate reports when the domain, not the platform, refused.
///
/// Distinct from anything a platform returns, and named, because
/// `StoreError::Unwrap` carries "the platform's own code" and this one is not
/// the platform's: it is this bridge saying the blob belongs to another entropy
/// domain.
pub const DOMAIN_MISMATCH: u32 = u32::MAX;

/// A signed platform code as the unsigned one `StoreError` carries.
///
/// Two's complement rather than a saturating clamp: a platform that returns -1
/// and one that returns 0xFFFF_FFFE must stay distinguishable in a report, and
/// clamping would fold every negative into one number.
const fn platform_code(code: i32) -> u32 {
    code as u32
}

/// Two bytes of length for the entropy domain, big-endian.
///
/// Fixed width so the split is exact: a delimiter would have to be escaped, and
/// an entropy domain is arbitrary bytes.
fn domain_header(len: usize) -> Result<[u8; 2], StoreError> {
    let Ok(len) = u16::try_from(len) else {
        return Err(StoreError::Unwrap {
            code: DOMAIN_MISMATCH,
        });
    };
    Ok(len.to_be_bytes())
}
