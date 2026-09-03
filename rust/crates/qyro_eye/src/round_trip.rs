//! La cadena entera menos la cámara.
//!
//! ADR-0048 §5: el ojo no sabe de dónde vienen los píxeles, y **eso es lo que
//! hace que esto se pueda probar**. Se dibuja lo que dibuja `qyro beam`, se
//! rasteriza como lo vería una cámara, y se le da al mismo `Eye` que CameraX
//! alimentará. Lo único que queda fuera es la cámara, y queda fuera **dicho**.
//!
//! # Lo que esto NO prueba, y hay que leerlo antes que los resultados
//!
//! Una cámara. Desenfoque, obturador rodante, moiré, brillo de la pantalla,
//! ángulo, autofoco que no engancha a 30 cm (`R10` §8 T3). Ahí es donde un canal
//! óptico falla de verdad. **No hay aparato y no se inventa.**

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "una prueba que no puede fallar en voz alta no es una prueba"
)]

use crate::{Eye, Look};

/// Dibuja un payload como lo dibuja `qyro beam` y lo devuelve ya en píxeles.
///
/// Escala `scale` píxeles por módulo, que es la magnitud que `R10` §8 T1 mide en
/// el aparato: pasarla como parámetro deja que la prueba **barra** el eje que no
/// se puede medir aquí, en vez de fijar uno y llamarlo verificado.
fn draw_as_luma(payload: &[u8], scale: usize) -> (Vec<u8>, usize, usize) {
    use qrcode::{EcLevel, QrCode};

    let code = QrCode::with_error_correction_level(payload, EcLevel::L).expect("cabe en un QR");
    let modules = code.width();
    const QUIET: usize = 4;
    let span = modules + QUIET * 2;
    let side = span * scale;

    // 255 es blanco: un QR necesita campo claro.
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

#[test]
fn un_archivo_entero_entra_por_el_ojo_con_frames_perdidos() {
    // La prueba que decide si la fase 24 vale algo. Cada frame se dibuja, se
    // rasteriza y se le da al ojo — **y uno de cada cuatro no llega**, porque un
    // canal que sólo funciona cuando no se pierde nada es el canal que el
    // fountain existe para evitar.
    let original: Vec<u8> = (0..1500).map(|i| ((i * 41 + 13) % 251) as u8).collect();
    let block_size: u16 = 220;
    let shape = qyro_fountain::Shape {
        payload_len: u32::try_from(original.len()).expect("pequeño"),
        block_size,
    };
    let blocks = qyro_fountain::split(&original, block_size);

    let mut eye = Eye::new();
    let mut seed = 1_u64;
    let mut complete = false;
    while !complete && seed < 400 {
        if seed % 4 != 0 {
            let frame = qyro_fountain::encode(&blocks, shape, seed);
            let wire = qyro_fountain::encode_frame(&frame);
            let (luma, width, height) = draw_as_luma(&wire, 4);
            if eye.look(&luma, width, height) == Look::Complete {
                complete = true;
            }
        }
        seed += 1;
    }

    let (seen, read) = eye.tally();
    assert!(
        complete,
        "el archivo no llego: {seen} mirados, {read} leidos"
    );
    assert_eq!(
        eye.finish().as_deref(),
        Some(original.as_slice()),
        "el archivo llego DISTINTO, que es peor que no llegar"
    );
    assert_eq!(eye.shape(), Some(shape));
}

#[test]
fn el_mismo_codigo_seis_veces_seguidas_es_lo_normal_y_no_progreso() {
    // A 30 fps de cámara contra 5 de pantalla, cada QR se ve unas seis veces. Un
    // ojo que contara eso como progreso mentiría en la barra.
    let shape = qyro_fountain::Shape {
        payload_len: 400,
        block_size: 200,
    };
    let payload: Vec<u8> = (0..400).map(|i| (i % 251) as u8).collect();
    let blocks = qyro_fountain::split(&payload, 200);
    let wire = qyro_fountain::encode_frame(&qyro_fountain::encode(&blocks, shape, 7));
    let (luma, width, height) = draw_as_luma(&wire, 4);

    let mut eye = Eye::new();
    let first = eye.look(&luma, width, height);
    assert!(
        matches!(first, Look::Progress { .. } | Look::Complete),
        "{first:?}"
    );
    for _ in 0..5 {
        assert_eq!(eye.look(&luma, width, height), Look::Repeat);
    }
    let (seen, read) = eye.tally();
    assert_eq!(
        (seen, read),
        (6, 6),
        "los seis se leyeron; solo uno era nuevo"
    );
}

#[test]
fn un_qr_que_no_es_de_qyro_no_entra() {
    // Una habitación con carteles. No es un ataque, es una pared.
    let (luma, width, height) = draw_as_luma(b"https://example.invalid/no-soy-qyro", 4);
    let mut eye = Eye::new();

    // **Esta prueba decía `Look::Nothing` hasta QYR-0381**, y la propiedad que
    // le importa no era ésa: era que un QR ajeno **no entre en la sesión**. Eso
    // se sigue afirmando abajo, y es lo que hay que proteger.
    //
    // Lo que cambia es que ahora se dice en voz alta que se leyó algo ajeno, en
    // vez de confundirlo con «no veo nada». El ojo no juzga qué es: sólo dice
    // que decodificó un QR que no es un frame suyo, y entrega los bytes para que
    // otro decida. Ese «otro» es `qyro_session::Scanner`.
    assert_eq!(eye.look(&luma, width, height), Look::Foreign);
    assert_eq!(
        eye.foreign(),
        Some(b"https://example.invalid/no-soy-qyro".as_slice()),
        "el ojo no entrego los bytes del QR ajeno"
    );

    assert_eq!(eye.shape(), None, "un QR ajeno fijo la forma de la sesion");
    assert_eq!(eye.finish(), None, "un QR ajeno produjo un archivo");
    // Y se contó como leído: el ojo SÍ vio un código, sólo que no era nuestro.
    // La distinción importa para la pantalla: «no veo nada» y «veo códigos que
    // no son de Qyro» piden acciones distintas.
    assert_eq!(eye.tally(), (1, 1));
}

#[test]
fn el_codigo_de_emparejamiento_que_dibuja_qyro_qr_se_lee() {
    // **QYR-0381, y es el QR que este producto pide escanear.** `qyro qr` lo
    // dibuja y escribe debajo «Point the other device's camera at this»; hasta
    // aquí, la cámara lo leía y lo tiraba en la misma rama que un cartel.
    //
    // El ojo sigue sin saber qué es: devuelve los bytes. Quien los reconoce es
    // la fachada, con el analizador de verdad.
    const CODE: &[u8] = b"QYRO1|192.168.1.7:49517|00112233445566778899aabbccddeeff";
    let (luma, width, height) = draw_as_luma(CODE, 4);
    let mut eye = Eye::new();

    assert_eq!(eye.look(&luma, width, height), Look::Foreign);
    assert_eq!(eye.foreign(), Some(CODE));
    assert_eq!(
        eye.shape(),
        None,
        "un codigo de emparejamiento abrio una sesion de archivo"
    );
}
