//! Stable native boundary for Qyro.
//!
//! The returned protocol-version memory is static, immutable, and always owned
//! by this library. Callers must never free or mutate it.

fn protocol_version_bytes() -> &'static [u8] {
    qyro_core::protocol_version().as_bytes()
}

/// Returns a borrowed pointer to the UTF-8 protocol-version bytes.
///
/// Read exactly the number of bytes returned by qyro_protocol_version_len.
/// The pointer remains valid for the process lifetime; ownership is not moved.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_protocol_version_ptr() -> *const u8 {
    protocol_version_bytes().as_ptr()
}

/// Returns the byte length of the value from qyro_protocol_version_ptr.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_protocol_version_len() -> usize {
    protocol_version_bytes().len()
}
