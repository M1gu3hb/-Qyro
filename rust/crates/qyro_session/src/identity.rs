//! The identity this process is, for as long as it runs.
//!
//! Specification: `docs/adr/ADR-0040-identity-persistence.md`.
//!
//! # What this replaces, and why it is the whole point
//!
//! Until phase 11 `Session` called `DeviceIdentity::generate()` in each of its
//! three constructors, so **every transfer minted a new keypair**. The
//! fingerprint a person was asked to compare out loud changed between one
//! transfer and the next, `TrustBook` could never hold a peer worth
//! recognising, and the application could not show its own pairing code because
//! it had no stable identity to build one from.
//!
//! The engine, the trust vocabulary, the storage format and the DPAPI backend
//! were all real and all tested. Nothing joined them. This module is the join.
//!
//! # Three rules, and none of them bends
//!
//! **Nothing generates unless the store is empty.** A blob that exists and will
//! not open is [`SessionError::IdentityUnreadable`] — never a reason to mint a
//! replacement. A device that quietly becomes a stranger to every peer that
//! trusted it is worse than a device that refuses to start a transfer.
//!
//! **Protection is asked for by name.** [`Protection::Platform`] does not fall
//! back to [`Protection::Sandbox`] when no wrapper is installed; it refuses.
//! ADR-0040 amendment 1.
//!
//! **The caller names the path.** Rust never guesses a directory. One code path
//! on every platform, the platform difference lives in Dart where
//! `defaultDestination()` already set that precedent, and a test can point at a
//! temporary directory — which is what makes the two-process test possible at
//! all.
//!
//! # Why there is no reset for tests
//!
//! `OnceLock` cannot be reset, and wrapping it in a `Mutex<Option<_>>` just so
//! tests could re-open would put a lock on the path every session takes for the
//! sake of the path no session takes. The process-global behaviour is the thing
//! under test, so it is exercised where it is real: `qyro_store_smoke
//! session-open`, run twice, in two processes.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use qyro_crypto::DeviceIdentity;
use qyro_identity_store::{SandboxWrapper, SecretWrapper, open_identity, seal_identity};

use crate::bridged_wrapper::{BridgedWrapper, WrapFn};
use crate::error::SessionError;

/// How the seed on disk is protected.
///
/// An enum and not a boolean, because the two are not opposites of one thing:
/// one names a platform mechanism and the other names its absence, and a build
/// that gained a third would want to say so rather than negate twice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protection {
    /// The platform's own wrapper: DPAPI on Windows, or whatever
    /// [`install_wrapper`] supplied.
    ///
    /// **Refuses when there is none.** Nobody receives less protection than
    /// they asked for because a wrapper happened to be missing.
    Platform,
    /// The filesystem sandbox, and nothing else.
    ///
    /// Android's stage A (ADR-0040 §7). The blob records it, so a later build
    /// with Keystore can refuse, migrate or warn rather than guess.
    Sandbox,
}

impl Protection {
    /// The wire value the FFI carries. ADR-0040 amendment 1.
    #[must_use]
    pub const fn from_code(code: u32) -> Option<Self> {
        match code {
            0 => Some(Self::Platform),
            1 => Some(Self::Sandbox),
            _ => None,
        }
    }
}

/// The wrapper the host installed, if it installed one.
static WRAPPER: OnceLock<BridgedWrapper> = OnceLock::new();

/// The identity this process uses, once opened.
static IDENTITY: OnceLock<DeviceIdentity> = OnceLock::new();

/// Where it was opened from, so a second call with another path is refused.
static OPENED_AT: OnceLock<PathBuf> = OnceLock::new();

/// Installs the platform wrapper for this process.
///
/// Must be called before [`open`]. Installing after an identity is already open
/// is refused: the blob on disk records which wrapper sealed it, and swapping
/// the wrapper underneath a live identity means the next write is unreadable by
/// the next read.
///
/// # Errors
///
/// [`SessionError::BadArgument`] if a wrapper is already installed, or if an
/// identity is already open.
pub fn install_wrapper(wrap: WrapFn, unwrap: WrapFn, context: usize) -> Result<(), SessionError> {
    if IDENTITY.get().is_some() {
        return Err(SessionError::BadArgument);
    }
    WRAPPER
        .set(BridgedWrapper::new(wrap, unwrap, context))
        .map_err(|_| SessionError::BadArgument)
}

/// Opens the identity stored at `path`, creating one if there is none.
///
/// Idempotent for the same path. A second call naming a **different** path is
/// [`SessionError::BadArgument`]: two identities in one process is not a state
/// this can be in, and silently keeping the first would make the second call
/// look like it worked.
///
/// # Errors
///
/// - [`SessionError::BadArgument`] — [`Protection::Platform`] with no wrapper
///   available, or a second call with a different path.
/// - [`SessionError::IdentityUnreadable`] — a blob exists and will not open, or
///   the file cannot be read for any reason other than not existing. **Nothing
///   is generated and the file is not touched.**
/// - [`SessionError::StorageRefused`] — a newly created identity could not be
///   written.
pub fn open(path: &Path, protection: Protection) -> Result<(), SessionError> {
    if let Some(previous) = OPENED_AT.get() {
        return if previous == path {
            Ok(())
        } else {
            Err(SessionError::BadArgument)
        };
    }

    let identity = match protection {
        Protection::Platform => open_with(path, &platform_wrapper()?)?,
        Protection::Sandbox => open_with(path, &SandboxWrapper)?,
    };

    // `set` losing a race means another thread opened the same path first, and
    // its identity is as valid as this one. Not an error.
    let _ = IDENTITY.set(identity);
    let _ = OPENED_AT.set(path.to_path_buf());
    Ok(())
}

/// This device's fingerprint, in the grouped form a person reads aloud.
///
/// # Errors
///
/// [`SessionError::IdentityUnreadable`] if [`open`] has not succeeded.
pub fn fingerprint() -> Result<String, SessionError> {
    Ok(crate::trust::fingerprint_text(current()?.public_identity()))
}

/// The identity for this process.
///
/// **Refuses rather than generating.** This is the function the three session
/// constructors call, and a fallback here would ship the original defect with
/// more code around it.
///
/// # Errors
///
/// [`SessionError::IdentityUnreadable`] if [`open`] has not succeeded.
pub(crate) fn current() -> Result<&'static DeviceIdentity, SessionError> {
    IDENTITY.get().ok_or(SessionError::IdentityUnreadable)
}

/// The wrapper for [`Protection::Platform`], or a refusal.
fn platform_wrapper() -> Result<PlatformWrapper, SessionError> {
    if let Some(installed) = WRAPPER.get() {
        return Ok(PlatformWrapper::Installed(installed));
    }
    #[cfg(windows)]
    {
        Ok(PlatformWrapper::Dpapi(qyro_win_dpapi::DpapiWrapper))
    }
    // No wrapper and no platform default: refuse, and do not quietly become the
    // sandbox. ADR-0040 amendment 1.
    #[cfg(not(windows))]
    {
        Err(SessionError::BadArgument)
    }
}

/// One type so `open_with` takes a single `&impl SecretWrapper`.
enum PlatformWrapper {
    Installed(&'static BridgedWrapper),
    #[cfg(windows)]
    Dpapi(qyro_win_dpapi::DpapiWrapper),
}

impl SecretWrapper for PlatformWrapper {
    fn wrap(
        &self,
        secret: &[u8],
        entropy: &[u8],
    ) -> Result<Vec<u8>, qyro_identity_store::StoreError> {
        match self {
            Self::Installed(inner) => inner.wrap(secret, entropy),
            #[cfg(windows)]
            Self::Dpapi(inner) => inner.wrap(secret, entropy),
        }
    }

    fn unwrap(
        &self,
        wrapped: &[u8],
        entropy: &[u8],
    ) -> Result<zeroize::Zeroizing<Vec<u8>>, qyro_identity_store::StoreError> {
        match self {
            Self::Installed(inner) => inner.unwrap(wrapped, entropy),
            #[cfg(windows)]
            Self::Dpapi(inner) => inner.unwrap(wrapped, entropy),
        }
    }

    fn wrap_id(&self) -> u8 {
        match self {
            Self::Installed(inner) => inner.wrap_id(),
            #[cfg(windows)]
            Self::Dpapi(inner) => inner.wrap_id(),
        }
    }
}

/// Loads the blob at `path`, or creates and stores one.
fn open_with(path: &Path, wrapper: &impl SecretWrapper) -> Result<DeviceIdentity, SessionError> {
    match std::fs::read(path) {
        Ok(bytes) => open_identity(&bytes, wrapper).map_err(|_| SessionError::IdentityUnreadable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => create(path, wrapper),
        // Permissions, a directory where a file should be, a device error: all
        // of these are "there is something and I cannot read it", which is not
        // the same as "there is nothing". Conflating them is how a device
        // replaces an identity a peer trusted.
        Err(_) => Err(SessionError::IdentityUnreadable),
    }
}

/// Generates an identity and writes it, atomically.
fn create(path: &Path, wrapper: &impl SecretWrapper) -> Result<DeviceIdentity, SessionError> {
    let identity = DeviceIdentity::generate().map_err(|_| SessionError::IdentityUnreadable)?;
    let blob = seal_identity(&identity, wrapper).map_err(|_| SessionError::IdentityUnreadable)?;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|_| SessionError::StorageRefused)?;
    }

    // Temp file then rename, in the same directory so the rename is atomic.
    // Without it a crash during first-run creation leaves a short blob that
    // `open_with` refuses **for ever** under the no-regeneration rule — the
    // rule that protects the identity would brick it instead.
    let temporary = path.with_extension("qyro-new");
    std::fs::write(&temporary, &blob).map_err(|_| SessionError::StorageRefused)?;
    std::fs::rename(&temporary, path).map_err(|_| {
        let _ = std::fs::remove_file(&temporary);
        SessionError::StorageRefused
    })?;

    Ok(identity)
}
