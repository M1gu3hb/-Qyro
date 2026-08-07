//! The wrapper and the file store built on it.

use std::path::{Path, PathBuf};

use qyro_crypto::DeviceIdentity;
use qyro_identity_store::{IdentityStore, SecretWrapper, StoreError, open_identity, seal_identity};
use zeroize::Zeroizing;

use crate::ffi::{
    CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData, DataBlob, GetLastError,
    take_and_free,
};

/// The `wrap` byte this backend writes. DPAPI, user scope.
const WRAP_DPAPI_USER: u8 = 1;

/// Builds a `DATA_BLOB` borrowing `bytes`.
///
/// DPAPI only reads the input blobs, so borrowing is sound and avoids a second
/// copy of a seed.
fn borrowed(bytes: &[u8]) -> DataBlob {
    DataBlob {
        cb_data: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
        pb_data: bytes.as_ptr().cast_mut(),
    }
}

/// DPAPI in user scope, with Qyro's entropy.
pub struct DpapiWrapper;

impl SecretWrapper for DpapiWrapper {
    fn wrap(&self, secret: &[u8], entropy: &[u8]) -> Result<Vec<u8>, StoreError> {
        // Refuse rather than silently truncate: `borrowed` clamps to u32::MAX,
        // and a clamped length would hand DPAPI fewer bytes than the caller
        // meant.
        if u32::try_from(secret.len()).is_err() || u32::try_from(entropy.len()).is_err() {
            return Err(StoreError::WrappedTooLarge {
                found: secret.len(),
            });
        }
        let input = borrowed(secret);
        let extra = borrowed(entropy);
        let mut out = DataBlob {
            cb_data: 0,
            pb_data: core::ptr::null_mut(),
        };
        // SAFETY: `input` and `extra` borrow live slices for the duration of the
        // call; `out` is a valid writable blob; the two null arguments are the
        // reserved and prompt parameters, which ADR-0024 §2 fixes at NULL.
        let ok = unsafe {
            CryptProtectData(
                &raw const input,
                core::ptr::null(),
                &raw const extra,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &raw mut out,
            )
        };
        if ok == 0 {
            // SAFETY: no call intervenes between the failure and this read.
            return Err(StoreError::Unwrap {
                code: unsafe { GetLastError() },
            });
        }
        // SAFETY: DPAPI succeeded, so `out` owns a buffer it expects LocalFree
        // to release, and nothing has freed it yet.
        Ok(unsafe { take_and_free(&mut out) })
    }

    fn unwrap(&self, wrapped: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, StoreError> {
        let input = borrowed(wrapped);
        let extra = borrowed(entropy);
        let mut out = DataBlob {
            cb_data: 0,
            pb_data: core::ptr::null_mut(),
        };
        // SAFETY: as in `wrap`. The description out-parameter is NULL because
        // nothing is written into `szDataDescr` when sealing.
        let ok = unsafe {
            CryptUnprotectData(
                &raw const input,
                core::ptr::null_mut(),
                &raw const extra,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &raw mut out,
            )
        };
        if ok == 0 {
            // Every tampering case arrives here: DPAPI authenticates its own
            // output, so a flipped bit anywhere in the wrapped body fails the
            // MAC rather than decrypting to garbage.
            // SAFETY: no call intervenes between the failure and this read.
            return Err(StoreError::Unwrap {
                code: unsafe { GetLastError() },
            });
        }
        // SAFETY: as in `wrap`.
        Ok(Zeroizing::new(unsafe { take_and_free(&mut out) }))
    }

    fn wrap_id(&self) -> u8 {
        WRAP_DPAPI_USER
    }
}

/// A device identity stored as one file under `%LOCALAPPDATA%`.
///
/// `LOCALAPPDATA` and not `APPDATA`: a roaming profile can decrypt DPAPI data
/// from another machine, and if the file roamed too, two machines would present
/// the same device identity. ADR-0024 §2 records that this reduces the problem
/// without closing it.
pub struct WindowsIdentityStore {
    path: PathBuf,
}

impl WindowsIdentityStore {
    /// The store at the default location under `%LOCALAPPDATA%`.
    ///
    /// # Errors
    ///
    /// [`StoreError::Io`] when `LOCALAPPDATA` is not set, which is a broken
    /// environment rather than an empty store — and the two must not be
    /// confused, or a device mints a fresh identity because a variable was
    /// missing.
    pub fn at_default_location() -> Result<Self, StoreError> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or(StoreError::Io { code: -1 })?;
        Ok(Self::at(Path::new(&base).join("Qyro").join("identity.bin")))
    }

    /// The store at an explicit path. Used by the harness and the tests.
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Where this store keeps its blob.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn io(error: &std::io::Error) -> StoreError {
        StoreError::Io {
            code: error.raw_os_error().unwrap_or(-1),
        }
    }

    fn read_blob(&self) -> Result<Vec<u8>, StoreError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(bytes),
            // The one case that is not a failure: there is simply nothing here.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(StoreError::IdentityAbsent)
            }
            Err(error) => Err(Self::io(&error)),
        }
    }
}

impl IdentityStore for WindowsIdentityStore {
    /// Refuses to replace an identity that is already there.
    ///
    /// Overwriting one silently is data loss: the fingerprint a peer trusted
    /// would change with nothing reporting it. Replacing on purpose is
    /// [`Self::rotate`], which is a different word for a different intent.
    fn create(&self, identity: &DeviceIdentity) -> Result<(), StoreError> {
        match self.read_blob() {
            Err(StoreError::IdentityAbsent) => {}
            Ok(_) => return Err(StoreError::AlreadyExists),
            // Unreadable is not absent. Refusing here is deliberate: writing
            // over bytes we could not parse would destroy the evidence of
            // whatever went wrong.
            Err(other) => return Err(other),
        }
        let blob = seal_identity(identity, &DpapiWrapper)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Self::io(&e))?;
        }
        std::fs::write(&self.path, &blob).map_err(|e| Self::io(&e))
    }

    fn load(&self) -> Result<DeviceIdentity, StoreError> {
        let blob = self.read_blob()?;
        open_identity(&blob, &DpapiWrapper)
    }

    /// Idempotent: deleting an empty store is success, not an error.
    ///
    /// The caller asked for there to be no identity, and there is none. A typed
    /// error here would make every caller write the same `match` to ignore it.
    fn delete(&self) -> Result<(), StoreError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Self::io(&error)),
        }
    }

    /// Replaces the stored identity, leaving exactly one.
    ///
    /// Writes before removing, so the window where the store holds nothing is
    /// the write itself rather than a gap between two operations. It is **not**
    /// atomic and this does not pretend otherwise: a crash mid-write can leave a
    /// partial file, which `load` reports as unreadable rather than absent — so
    /// the failure is loud, and no identity is silently minted. Making it atomic
    /// needs a temporary file and a replace, which is transfer-filesystem work
    /// and out of this sprint's scope.
    fn rotate(&self) -> Result<DeviceIdentity, StoreError> {
        let replacement = DeviceIdentity::generate().map_err(|_| StoreError::Io { code: -2 })?;
        let blob = seal_identity(&replacement, &DpapiWrapper)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Self::io(&e))?;
        }
        std::fs::write(&self.path, &blob).map_err(|e| Self::io(&e))?;
        Ok(replacement)
    }
}
