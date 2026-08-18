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
        }
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
