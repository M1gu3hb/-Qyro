//! El ojo: píxeles de luma entran, un archivo sale.
//!
//! Especificación: ADR-0048, medida en `R10`.
//!
//! # Lo que este crate no sabe
//!
//! **No sabe qué es una cámara.** No conoce CameraX, ni JNI, ni Android, ni un
//! `ImageProxy`. Recibe un plano de luma de 8 bits con su anchura y su altura, y
//! eso es todo lo que pide.
//!
//! Es la decisión de ADR-0048 §5 y tiene una consecuencia concreta: **la prueba
//! de la fase 15 y la de la fase 24 son la misma prueba.** El arnés que
//! rasteriza lo que dibuja `qyro beam` produce exactamente la clase de píxeles
//! que CameraX entregará, así que la cadena entera se ejercita **sin cámara** —
//! y la única parte que no se puede ejercitar aquí queda aislada en el glue de
//! Kotlin, donde se ve.
//!
//! # Lo que sí sabe
//!
//! Que un frame puede no traer código, que el mismo código aparecerá muchas
//! veces seguidas —a 30 fps de cámara contra 5 de pantalla, cada QR se ve unas
//! seis veces— y que **nada de eso es un error**.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

mod eye;
mod geometry;
mod look;

#[cfg(test)]
mod guards;

#[cfg(test)]
mod round_trip;

#[cfg(test)]
mod tests;

pub use eye::Eye;
pub use geometry::{
    PIXELS_PER_MODULE_FLOOR, QR_SHARE_OF_FRAME, is_above_floor, modules_of_version,
    pixels_per_module,
};
pub use look::Look;
