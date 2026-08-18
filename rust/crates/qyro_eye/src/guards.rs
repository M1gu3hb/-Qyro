//! Las guardas estructurales que lleva cada caja de este taller.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "el analisis compartido sirve a varias cajas, lee archivos, y tiene \
              que fallar en voz alta cuando no puede"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Todo archivo que entra en una compilación de release de esta caja.
const PRODUCTION_FILES: [&str; 4] = ["lib.rs", "eye.rs", "geometry.rs", "look.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

/// Toda variante de `Look` tiene que poder producirse.
///
/// Aquí importa más que de costumbre: las cuatro describen estados que una
/// interfaz va a dibujar, y una que nadie construya sería una pantalla que nunca
/// aparece con un `match` que aparenta cubrirla.
#[test]
fn every_look_has_a_construction_site() {
    assert_every_variant_has_a_construction_site(&PRODUCTION_FILES, "look.rs", "Look", 4, &[]);
}
