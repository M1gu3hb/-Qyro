use qyro_ffi::{qyro_protocol_version_len, qyro_protocol_version_ptr};

#[test]
fn c_abi_exposes_the_protocol_version_without_ownership_transfer() {
    let pointer = qyro_protocol_version_ptr();
    let length = qyro_protocol_version_len();

    assert!(!pointer.is_null());

    // SAFETY: The ABI contract promises a non-null pointer to immutable static
    // bytes and returns their exact length. Ownership remains in the library.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };

    assert_eq!(bytes, b"QYRO/1");
}
