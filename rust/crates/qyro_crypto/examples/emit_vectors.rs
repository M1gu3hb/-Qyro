//! Emits the identity test vectors. TEST ONLY — NEVER PRODUCTION.
//!
//! cargo run -p qyro_crypto --features test-vectors --example emit_vectors

use qyro_crypto::{DeviceIdentity, SignatureDomain};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    // RFC 8032 section 7.1 TEST 1 secret key: public, well-known, test-only.
    let seed: [u8; 32] = [
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ];
    let identity = DeviceIdentity::from_test_seed(&seed);
    let public = identity.public_identity();

    println!("SEED {}", hex(&seed));
    println!("PUBKEY {}", hex(public.as_bytes()));
    println!("FINGERPRINT {}", public.fingerprint().to_hex());
    println!("GROUPED {}", public.fingerprint().to_grouped_hex());

    for (name, domain) in [
        ("TestVector", SignatureDomain::TestVector),
        ("DeviceClaim", SignatureDomain::DeviceClaim),
    ] {
        for message in [b"".as_slice(), b"qyro".as_slice()] {
            let signature = identity.sign(domain, message);
            println!(
                "SIG {name} {} {} {}",
                domain.to_wire(),
                hex(message),
                signature.to_hex()
            );
        }
    }
}
