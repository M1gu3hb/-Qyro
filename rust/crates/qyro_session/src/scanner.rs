//! El ojo, envuelto para que lo alcance la frontera C.
//!
//! Especificación: ADR-0048.
//!
//! # Por qué existe este módulo en vez de exponer `qyro_eye` directamente
//!
//! `qyro_ffi` sólo puede nombrar `qyro_core` y `qyro_session`, y la guarda
//! `the_ffi_names_exactly_two_crates` lo comprueba. Reexportar `qyro_eye` desde
//! aquí tampoco vale: `qyro_session_re_exports_nothing_it_does_not_own` lo
//! rechazaría, y con razón — todo lo que esta fachada republica se vuelve
//! nombrable desde la frontera.
//!
//! Así que el ojo se **envuelve**, que es exactamente la forma que `browse` usa
//! con `qyro_net`. Lo que cruza son enteros y bytes en un búfer prestado, como
//! todo lo demás de esta frontera.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::scan_state::ScanState;

/// Un escaneo en marcha.
///
/// Vive tantos frames como haga falta. Es el estado que la cámara alimenta y del
/// que sale el archivo.
pub struct Scanner {
    eye: qyro_eye::Eye,
    /// El último código de emparejamiento leído, si hubo alguno.
    ///
    /// QYR-0381. Se conserva aunque después pasen carteles por delante: entre
    /// que se reconoce y que alguien toca la pantalla pasan decenas de frames, y
    /// perderlo ahí sería perderlo justo cuando ya se tenía.
    pairing: Option<String>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    /// Un escaneo que todavía no ha visto nada.
    #[must_use]
    pub fn new() -> Self {
        Self {
            eye: qyro_eye::Eye::new(),
            pairing: None,
        }
    }

    /// Mira un plano de luma de 8 bits.
    ///
    /// `luma` son `width * height` bytes, una fila tras otra. Es el plano 0 de un
    /// `ImageProxy` en `YUV_420_888` **ya sin relleno**: quitarlo es trabajo de
    /// quien lo saca de la cámara, porque sólo él sabe el `rowStride`.
    pub fn look(&mut self, luma: &[u8], width: usize, height: usize) -> ScanState {
        match self.eye.look(luma, width, height) {
            qyro_eye::Look::Nothing => ScanState::Nothing,
            qyro_eye::Look::Repeat => ScanState::Repeat,
            qyro_eye::Look::Progress { solved, total } => ScanState::Progress {
                solved: u32::try_from(solved).unwrap_or(u32::MAX),
                total: u32::try_from(total).unwrap_or(u32::MAX),
            },
            qyro_eye::Look::Complete => ScanState::Complete,
            // **Aquí es donde se decide qué era ese QR** (ADR-0032 enmienda 8).
            //
            // El ojo entrega bytes y no sabe qué son. Esta fachada sí puede
            // nombrar a `qyro_net`, así que pregunta con **el analizador de
            // verdad** en vez de mirar un prefijo: una cadena que casi lo parece
            // -- huella corta, mayúsculas, prefijo ajeno -- se rechaza por las
            // mismas reglas que rechazan al resto del producto.
            //
            // Y lo que no es un código sale como `Nothing`, que es lo que era
            // antes de esto: un cartel de la pared no es un suceso.
            qyro_eye::Look::Foreign => match self.eye.foreign() {
                Some(bytes) => match core::str::from_utf8(bytes) {
                    Ok(text) if crate::parse_pairing(text).is_ok() => {
                        self.pairing = Some(text.to_owned());
                        ScanState::Pairing
                    }
                    _ => ScanState::Nothing,
                },
                None => ScanState::Nothing,
            },
        }
    }

    /// El código de emparejamiento leído, entero, si se leyó alguno.
    ///
    /// **Entero, con su huella**, y no sólo la dirección: la huella es la mitad
    /// que hace que escanear valga más que teclear (ADR-0035 §2.1). Quien lo
    /// recibe la compara con la del apretón y se niega si no coincide.
    #[must_use]
    pub fn pairing(&self) -> Option<&str> {
        self.pairing.as_deref()
    }

    /// Cuántos frames se han mirado y cuántos traían un código.
    ///
    /// Las dos, porque «he mirado 300 y he leído 2» y «he mirado 300 y he leído
    /// 280» son la misma barra de progreso y dos situaciones opuestas: la
    /// primera dice que hay que acercar el teléfono.
    #[must_use]
    pub fn tally(&self) -> (u64, u64) {
        self.eye.tally()
    }

    /// El archivo, cuando está entero. Nunca uno a medias.
    #[must_use]
    pub fn finish(&self) -> Option<Vec<u8>> {
        self.eye.finish()
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "una prueba que no puede fallar en voz alta no es una prueba"
    )]

    use super::Scanner;
    use crate::scan_state::ScanState;

    /// Dibuja un payload como lo dibuja `qyro qr` y lo devuelve ya en píxeles.
    ///
    /// La misma rasterización que `qyro_eye::round_trip`, y por la misma razón:
    /// lo único que queda fuera es la cámara, y queda fuera **dicho**.
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

    #[test]
    fn un_codigo_de_emparejamiento_leido_por_la_camara_sale_entero() {
        // **QYR-0381.** Es el QR que `qyro qr` dibuja bajo la frase «Point the
        // other device's camera at this», y hasta ahora la cámara lo leía y lo
        // tiraba. Sale **entero** —dirección y huella— porque la huella es la
        // mitad que hace que escanear valga algo (ADR-0035 §2.1).
        const CODE: &str = "QYRO1|192.168.1.7:49517|00112233445566778899aabbccddeeff";
        let (luma, width, height) = draw_as_luma(CODE.as_bytes(), 4);

        let mut scanner = Scanner::new();
        assert_eq!(scanner.look(&luma, width, height), ScanState::Pairing);
        assert_eq!(scanner.pairing(), Some(CODE));

        // Y no ha empezado ninguna transferencia de archivo por el camino.
        assert_eq!(scanner.finish(), None);
    }

    #[test]
    fn un_qr_de_la_pared_no_es_un_codigo_y_no_se_guarda() {
        // El control, y es el que impide que esto acepte cualquier cosa. Una
        // habitación con carteles no puede producir un emparejamiento.
        for foreign in [
            "https://example.invalid/no-soy-qyro",
            // Casi uno: prefijo bueno, huella corta.
            "QYRO1|192.168.1.7:49517|00112233445566778899aabbccddee",
            // Casi uno: prefijo malo.
            "NOTQYRO|192.168.1.7:49517|00112233445566778899aabbccddeeff",
            // Casi uno: mayúsculas en la huella, que ADR-0035 §2 no acepta.
            "QYRO1|192.168.1.7:49517|00112233445566778899AABBCCDDEEFF",
        ] {
            let (luma, width, height) = draw_as_luma(foreign.as_bytes(), 4);
            let mut scanner = Scanner::new();
            assert_eq!(
                scanner.look(&luma, width, height),
                ScanState::Nothing,
                "{foreign:?} se acepto como codigo de emparejamiento"
            );
            assert_eq!(scanner.pairing(), None, "{foreign:?} dejo un codigo");
        }
    }

    #[test]
    fn la_huella_del_codigo_leido_es_la_que_se_va_a_comparar() {
        // **La contraprueba de QYR-0381, y es el punto entero de la ficha.**
        //
        // Escanear no establece confianza: fija una expectativa (ADR-0035 §2.1).
        // Lo que esta prueba fija es que la expectativa **sobrevive el viaje por
        // la cámara** — que la huella que sale del QR es la que se comparará con
        // la del apretón, carácter a carácter.
        //
        // Y su contraparte: cambiar **un** carácter de esa huella tiene que
        // dejar de coincidir. Sin esta mitad, lo de arriba lo pasaría una
        // comparación que siempre dice que sí.
        const HUELLA: &str = "00112233445566778899aabbccddeeff";
        let code = format!("QYRO1|192.168.1.7:49517|{HUELLA}");
        let (luma, width, height) = draw_as_luma(code.as_bytes(), 4);

        let mut scanner = Scanner::new();
        assert_eq!(scanner.look(&luma, width, height), ScanState::Pairing);
        let read = scanner.pairing().expect("se leyo el codigo");

        let expectation = crate::pairing_fingerprint(read).expect("la huella sale del codigo");
        assert_eq!(expectation, HUELLA, "la huella no sobrevivio a la camara");

        // Un aparato cuya huella autenticada sea ésta: coincide.
        assert!(matches(&expectation, HUELLA));

        // Y uno cuya huella difiera en un solo caracter: no.
        let impostor = format!("{}0", &HUELLA[..HUELLA.len() - 1]);
        assert_ne!(impostor, HUELLA, "el impostor es la misma cadena");
        assert!(
            !matches(&expectation, &impostor),
            "una huella distinta en un caracter se acepto como la misma, asi que \
             escanear no ata la sesion a ninguna clave"
        );
    }

    /// La misma normalización que hacen las dos caras al comparar huellas.
    ///
    /// Escrita aquí a propósito y no importada: si la de producción cambiara a
    /// algo que acepta de más, esta prueba seguiría diciendo la verdad sobre lo
    /// que *debería* pasar, y las dos dejarían de coincidir en voz alta.
    fn matches(actual: &str, wanted: &str) -> bool {
        fn normalise(text: &str) -> String {
            text.chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect()
        }
        !wanted.is_empty() && normalise(actual) == normalise(wanted)
    }

    #[test]
    fn el_codigo_leido_no_se_pisa_con_un_cartel_que_pase_despues() {
        // La cámara sigue mirando después de leer el código: a 30 fps, entre que
        // se reconoce y que alguien toca la pantalla pasan decenas de frames, y
        // cualquiera puede traer un cartel. Perder el código ahí sería perderlo
        // justo cuando ya se tenía.
        const CODE: &str = "QYRO1|10.0.0.4:49517|ffeeddccbbaa99887766554433221100";
        let (good, width, height) = draw_as_luma(CODE.as_bytes(), 4);
        let mut scanner = Scanner::new();
        assert_eq!(scanner.look(&good, width, height), ScanState::Pairing);

        let (poster, pw, ph) = draw_as_luma(b"https://example.invalid/cartel", 4);
        assert_eq!(scanner.look(&poster, pw, ph), ScanState::Nothing);
        assert_eq!(
            scanner.pairing(),
            Some(CODE),
            "un cartel posterior borro el codigo ya leido"
        );
    }
}
