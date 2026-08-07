//! The DPAPI entry points, declared by hand.
//!
//! ADR-0024 §1 chose this over `windows-sys` to keep eleven crates out of an
//! audited graph for two function declarations, and stated the cost plainly:
//! a hand transcription can get an ABI or a struct layout wrong, and that is
//! undefined behaviour rather than a compile error. The mitigation it promised
//! is `a_data_blob_that_lies_does_not_round_trip`, which runs on a real Windows
//! runner — a `DATA_BLOB` whose fields are wrong does not survive a
//! protect/unprotect.
//!
//! Signatures from [`CryptProtectData`][cpd] and [`CryptUnprotectData`][cud],
//! consulted 2026-08-07.
//!
//! [cpd]: https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata
//! [cud]: https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptunprotectdata

use zeroize::Zeroize;

/// `DATA_BLOB`, the only struct crossing this boundary.
///
/// `cbData` is a `DWORD` and `pbData` a `BYTE *`. Two fields, in that order,
/// C layout — nothing about it has changed since Windows 2000.
#[repr(C)]
pub(crate) struct DataBlob {
    pub(crate) cb_data: u32,
    pub(crate) pb_data: *mut u8,
}

/// Non-interactive: required because Qyro's store cannot present a UI. Without
/// it, an operation that wants one fails with `ERROR_PASSWORD_RESTRICTION`.
pub(crate) const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

// `CRYPTPROTECT_LOCAL_MACHINE` is deliberately absent rather than defined and
// unused. ADR-0024 §2 refuses it on the archived documentation's own words —
// with it "no real protection is provided" and any process on the machine can
// unprotect the data. A constant nobody defines is a constant nobody passes by
// accident.

// `Crypt32.lib` is **not** linked by default, and nothing in a `cargo check`
// notices: type-checking does not link, so the omission survived a clean
// cross-compile check on Linux and surfaced only as LNK2019 on a real Windows
// linker. The reference names the library in its requirements table — "Library:
// Crypt32.lib" — and that line is load-bearing rather than informational.
//
// `LocalFree` and `GetLastError` are kernel32, which the MSVC target links
// anyway; they are declared in a separate block so this attribute says exactly
// which two symbols need it.
#[link(name = "Crypt32")]
unsafe extern "system" {
    /// Windows: `Crypt32.dll`.
    pub(crate) fn CryptProtectData(
        p_data_in: *const DataBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *mut core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;

    pub(crate) fn CryptUnprotectData(
        p_data_in: *const DataBlob,
        pp_sz_data_descr: *mut *mut u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *mut core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;
}

unsafe extern "system" {
    /// Kernel32. The reference is explicit that a caller frees `pbData` with it.
    pub(crate) fn LocalFree(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;

    pub(crate) fn GetLastError() -> u32;
}

/// Copies a DPAPI output out, wipes it, and frees it.
///
/// The wipe is not optional and not decoration. The reference says to free
/// `pbData` with `LocalFree`; it says nothing about clearing it first, and a
/// buffer holding a seed that is freed without clearing is sensitive material in
/// freed memory. QYR-0018 closed exactly this defect once already, on the AEAD
/// plaintext path.
///
/// # Safety
///
/// `blob` must be a `DATA_BLOB` that DPAPI filled and that nobody has freed.
pub(crate) unsafe fn take_and_free(blob: &mut DataBlob) -> Vec<u8> {
    if blob.pb_data.is_null() || blob.cb_data == 0 {
        return Vec::new();
    }
    let len = blob.cb_data as usize;
    // SAFETY: DPAPI reports `cb_data` bytes at `pb_data` on success, and the
    // caller has not freed it.
    let slice = unsafe { core::slice::from_raw_parts_mut(blob.pb_data, len) };
    let copied = slice.to_vec();
    slice.zeroize();
    // SAFETY: `pb_data` came from DPAPI, which documents LocalFree as the way
    // to release it, and it is released exactly once here.
    unsafe {
        LocalFree(blob.pb_data.cast::<core::ffi::c_void>());
    }
    blob.pb_data = core::ptr::null_mut();
    blob.cb_data = 0;
    copied
}
