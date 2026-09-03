//! El símbolo 35 de ADR-0032 (enmienda 8), ejercitado por la frontera C.
//!
//! **QYR-0381.** `qyro qr` dibuja un código de emparejamiento y escribe debajo
//! «Point the other device's camera at this». Esto comprueba que lo que la
//! cámara lee llega entero al otro lado de la frontera — con su huella, que es
//! la mitad que hace que escanear valga más que teclear.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "una prueba que no puede fallar en voz alta no es una prueba"
)]

use qyro_ffi::{qyro_scanner_close, qyro_scanner_look, qyro_scanner_open, qyro_scanner_pairing};

const QYRO_OK: i32 = 0;
const QYRO_ERR_BAD_ARGUMENT: i32 = -2;
const QYRO_ERR_NULL_OUT: i32 = -3;
const QYRO_ERR_NOT_READY: i32 = -15;

/// El código que `qyro qr` dibujaría.
const CODE: &str = "QYRO1|192.168.1.7:49517|00112233445566778899aabbccddeeff";

/// Dibuja un payload como lo dibuja `qyro qr` y lo devuelve ya en píxeles.
fn draw_as_luma(payload: &[u8], scale: usize) -> (Vec<u8>, usize, usize) {
    use qrcode::{EcLevel, QrCode};

    let code = QrCode::with_error_correction_level(payload, EcLevel::L).expect("cabe en un QR");
    let modules = code.width();
    const QUIET: usize = 4;
    let side = (modules + QUIET * 2) * scale;

    let mut luma = vec![255_u8; side * side];
    let colors = code.to_colors();
    for row in 0..modules {
        for column in 0..modules {
            if colors[row * modules + column] != qrcode::Color::Dark {
                continue;
            }
            for y in 0..scale {
                for x in 0..scale {
                    let py = (row + QUIET) * scale + y;
                    let px = (column + QUIET) * scale + x;
                    if let Some(pixel) = luma.get_mut(py * side + px) {
                        *pixel = 0;
                    }
                }
            }
        }
    }
    (luma, side, side)
}

fn open() -> u64 {
    let mut handle = 0_u64;
    assert_eq!(unsafe { qyro_scanner_open(&raw mut handle) }, QYRO_OK);
    handle
}

fn show(handle: u64, payload: &[u8]) -> i32 {
    let (luma, width, height) = draw_as_luma(payload, 4);
    unsafe { qyro_scanner_look(handle, luma.as_ptr(), width, height) }
}

#[test]
fn un_codigo_leido_cruza_entero_y_con_su_huella() {
    let handle = open();

    // Antes de mirar nada no hay código, y eso no es un error de llamada.
    let mut needed = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, std::ptr::null_mut(), 0, &raw mut needed) },
        QYRO_ERR_NOT_READY,
        "un escaner recien abierto dijo tener un codigo"
    );

    // `ScanState::Pairing` es 4 (ADR-0032 enmienda 8).
    assert_eq!(show(handle, CODE.as_bytes()), 4);

    // Preguntar con capacidad cero no es un éxito: es la forma documentada de
    // preguntar cuánto reservar.
    let mut needed = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, std::ptr::null_mut(), 0, &raw mut needed) },
        QYRO_ERR_BAD_ARGUMENT
    );
    assert_eq!(needed, CODE.len());

    // Un byte corto: nada escrito, y la longitud verdadera.
    let mut tight = vec![0xAA_u8; needed - 1];
    let mut reported = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, tight.as_mut_ptr(), tight.len(), &raw mut reported) },
        QYRO_ERR_BAD_ARGUMENT
    );
    assert_eq!(reported, needed);
    assert!(
        tight.iter().all(|byte| *byte == 0xAA),
        "medio codigo escrito es peor que ninguno"
    );

    // Y entero.
    let mut buffer = vec![0_u8; needed];
    let mut wrote = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, buffer.as_mut_ptr(), buffer.len(), &raw mut wrote) },
        QYRO_OK
    );
    assert_eq!(String::from_utf8(buffer[..wrote].to_vec()).unwrap(), CODE);

    qyro_scanner_close(handle);
}

#[test]
fn un_cartel_de_la_pared_no_produce_un_codigo() {
    // El control. Sin él, lo de arriba pasaría con un símbolo que devolviera
    // cualquier cosa que la cámara viera.
    let handle = open();
    assert_eq!(
        show(handle, b"https://example.invalid/no-soy-qyro"),
        0,
        "un cartel salio como algo distinto de `Nothing`"
    );

    let mut needed = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, std::ptr::null_mut(), 0, &raw mut needed) },
        QYRO_ERR_NOT_READY,
        "un cartel de la pared se guardo como codigo de emparejamiento"
    );
    qyro_scanner_close(handle);
}

#[test]
fn los_argumentos_imposibles_se_niegan_y_no_escriben() {
    let handle = open();
    assert_eq!(
        unsafe { qyro_scanner_pairing(handle, std::ptr::null_mut(), 0, std::ptr::null_mut()) },
        QYRO_ERR_NULL_OUT
    );
    let mut len = 0_usize;
    assert_eq!(
        unsafe { qyro_scanner_pairing(u64::MAX, std::ptr::null_mut(), 0, &raw mut len) },
        QYRO_ERR_BAD_ARGUMENT,
        "un identificador que no existe no fue rechazado"
    );
    qyro_scanner_close(handle);
}
