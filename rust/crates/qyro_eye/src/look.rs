//! Qué dejó un frame por el ojo.
//!
//! En su propio módulo, como todo tipo con variantes en este taller, y la razón
//! es la guarda: una variante sólo cuenta como real cuando algo **distinto de su
//! propia declaración** la construye. Declarar y construir en el mismo archivo
//! deja que una variante se escriba, se cubra en un `match`, y no se produzca
//! nunca — que aquí sería una pantalla que nadie llega a ver.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Qué dejó un frame.
///
/// Las tres son estados normales y **ninguna es un error**. A 30 fps de cámara
/// contra 5 de pantalla, de cada seis frames uno trae código nuevo, cuatro
/// traen el mismo de antes y varios no traen nada mientras la pantalla cambia.
/// Una interfaz que tratara cualquiera de los tres como fallo estaría enseñando
/// un error cinco veces por segundo durante una transferencia que va bien.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Look {
    /// No había código legible. El caso más común, y no significa nada malo.
    Nothing,
    /// Había código y ya se conocía. El segundo más común.
    Repeat,
    /// Código nuevo, con cuántos bloques faltan.
    Progress { solved: usize, total: usize },
    /// Con éste ya está: el archivo se puede sacar con [`crate::Eye::finish`].
    Complete,
}
