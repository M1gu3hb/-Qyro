//! The shared structural minimum, for the crate that is a binary.
//!
//! See `rust/guards/source_guard.rs`. The meta-guard in `qyro_identity_store`
//! demanded this within a minute of the crate existing, which is the guard
//! doing its job: a new crate with no analysis is a new place for a panic to
//! live unnoticed.
//!
//! # Why a binary needs this as much as a library
//!
//! More, actually. A panic in `qyro_ffi` becomes a return code at the C
//! frontier; a panic here **is the whole program dying in front of somebody
//! who was told to type a command**, on a machine where they cannot install a
//! debugger to find out why.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the shared analysis serves several crates, reads files, and must \
              fail loudly when it cannot"
)]

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../guards/source_guard.rs"
));

/// Every file compiled into a release build of this crate.
const PRODUCTION_FILES: [&str; 5] = ["main.rs", "flows.rs", "optical.rs", "serial.rs", "term.rs"];

#[test]
fn no_production_path_can_panic() {
    assert_no_production_path_can_panic(&PRODUCTION_FILES);
}

#[test]
fn every_production_file_is_listed() {
    assert_the_production_list_matches_the_source(&PRODUCTION_FILES);
}

#[test]
fn the_analysis_reaches_the_end_of_every_production_file() {
    for file in PRODUCTION_FILES {
        assert_analysis_reached_the_end(file, &production_source(file));
    }
}

#[test]
fn no_assertion_compares_a_call_to_itself() {
    assert_no_assertion_compares_a_call_to_itself();
}

#[test]
fn no_rust_source_carries_a_raw_nul_byte() {
    for file in PRODUCTION_FILES {
        let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert!(
            !bytes.contains(&0),
            "{file} carries a raw NUL byte, which makes grep treat it as binary \
             and skip it entirely (QYR-0327). Write the escape instead."
        );
    }
}

/// El decodificador de QR **sí** viaja en el producto, y su aviso sigue argumentado.
///
/// **Esta guarda decía lo contrario y pasaba mirando el sitio equivocado**
/// (QYR-0399). Se llamaba `the_qr_decoder_never_becomes_a_shipped_dependency` y
/// leía **sólo** `qyro_cli/Cargo.toml`, buscando `rqrr` en la mitad normal. Ahí
/// no está, así que pasaba — mientras `rqrr` llegaba igual, por
/// `qyro_eye`, que este crate nombra directamente **y** vuelve a llegar por
/// `qyro_session`. El decodificador lleva fases dentro del binario.
///
/// Y la decisión no fue un descuido: `qyro qr` **comprueba que su propio código
/// se lee** antes de decirle a nadie que lo escanee, y ése es el llamante de
/// producción de `qyro_eye` en esta cara (`flows.rs`). ADR-0048 lo eligió.
///
/// Así que lo que hay que guardar no es la ausencia del decodificador: es que
/// **su aviso siga siendo una alarma con argumento y no una silenciada.**
/// `.cargo/audit.toml` lo dice en su primera línea — «un ignore sin argumento es
/// una alarma silenciada, y una alarma silenciada es peor que ninguna porque se
/// lee como *limpio*» — y esta guarda es quien lo comprueba.
#[test]
fn the_shipped_qr_decoder_keeps_its_advisory_argued() {
    fn normal_dependencies(manifest: &str) -> String {
        let declared: String = manifest
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<&str>>()
            .join("\n");
        declared
            .split("[dev-dependencies]")
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    let read = |path: &str| -> String {
        let full = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..").to_owned() + "/" + path;
        std::fs::read_to_string(&full).unwrap_or_else(|error| panic!("{path}: {error}"))
    };

    // **El camino real, en dos saltos declarados.** Preguntarlo así y no por el
    // manifiesto de este crate es lo que arregla el defecto: `rqrr` no aparece
    // aquí y llega igual.
    let cli = normal_dependencies(&read("rust/crates/qyro_cli/Cargo.toml"));
    assert!(
        cli.contains("qyro_eye"),
        "qyro_cli ya no nombra qyro_eye. Si el decodificador dejo de viajar, \
         esta guarda sobra y el ignore de .cargo/audit.toml tambien: borralos \
         los dos en el mismo commit."
    );
    let eye = normal_dependencies(&read("rust/crates/qyro_eye/Cargo.toml"));
    assert!(
        eye.contains("rqrr"),
        "qyro_eye ya no depende de rqrr en su mitad normal, asi que el argumento \
         de .cargo/audit.toml -- que el decodificador viaja -- dejo de ser cierto"
    );

    // Y el aviso, con su argumento y su condición de caducidad escritas.
    let audit = read(".cargo/audit.toml");
    assert!(
        audit.contains("RUSTSEC-2026-0253"),
        "el aviso del decodificador que SI viaja desaparecio de .cargo/audit.toml \
         sin que este guardian se enterara"
    );
    assert!(
        audit.contains("Borrala cuando") || audit.contains("Bórrala cuando"),
        "el ignore de RUSTSEC-2026-0253 perdio su condicion de caducidad, asi \
         que es una alarma silenciada -- que es lo que la primera linea de ese \
         archivo prohibe"
    );

    // El control: sin él, un `read` que devolviera cadenas vacías haría pasar
    // las dos primeras afirmaciones al revés y ésta al derecho.
    assert!(
        !cli.contains("rqrr"),
        "rqrr aparece ahora como dependencia DIRECTA de qyro_cli. No es un \
         fallo, pero esta guarda mide el camino de dos saltos: reescribela para \
         el camino nuevo en vez de dejarla midiendo otro."
    );
}
