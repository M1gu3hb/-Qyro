//! En qué quedó un frame.
//!
//! En su propio módulo porque la guarda de variantes exige que algo distinto de
//! la declaración construya cada una — y aquí importa: las cuatro son estados
//! que una pantalla va a dibujar mientras alguien sostiene un teléfono, y una
//! que nadie construyera sería una pantalla que nunca aparece.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// En qué quedó un frame. **Ninguna de las cinco es un error.**
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanState {
    /// No había código legible. El caso más común de todos.
    Nothing,
    /// Había código y ya se conocía. El segundo más común: a 30 fps de cámara
    /// contra 5 de pantalla, cada QR se ve unas seis veces.
    Repeat,
    /// Código nuevo, con cuántos bloques hay y cuántos faltan.
    Progress { solved: u32, total: u32 },
    /// Con éste ya está.
    Complete,
    /// Lo leído no era un archivo: era un **código de emparejamiento**.
    ///
    /// **QYR-0381.** Una rama distinta de la pantalla y no un grado de progreso:
    /// las cuatro de arriba dicen «sigue mirando, falta tanto», y ésta dice
    /// «para, ya tienes lo que hacía falta». Confundirlas dejaría al escáner
    /// buscando bloques de un archivo que nadie está mandando.
    ///
    /// El código se saca con [`crate::Scanner::pairing`]. **No establece
    /// confianza**: es la expectativa que hay que comparar con la huella del
    /// apretón, y quien la compara es la pantalla de Enviar (ADR-0035 §2.1).
    Pairing,
}

impl ScanState {
    /// El código que cruza la frontera C.
    ///
    /// Un entero y no un puntero: ADR-0032 — por la frontera de este producto no
    /// cruza ningún tipo.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Nothing => 0,
            Self::Repeat => 1,
            Self::Progress { .. } => 2,
            Self::Complete => 3,
            Self::Pairing => 4,
        }
    }
}
