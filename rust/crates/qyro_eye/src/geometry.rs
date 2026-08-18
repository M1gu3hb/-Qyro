//! La aritmética de `R10` §8 T1, en código y con prueba.
//!
//! **Está aquí porque la medida que manda `R10` no se pudo hacer.** Esa medida
//! —píxeles por módulo en el aparato real— necesita un aparato, y no hay
//! ninguno; ADR-0048 §4 deja el hueco en blanco y lo dice. Lo que sí se puede
//! hacer es la aritmética que decide la palanca, y dejarla comprobada en vez de
//! escrita en prosa: una constante que alguien vaya a mover algún día merece que
//! los números que la justifican fallen si se mueven mal.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

/// Qué parte del alto del frame ocupa el código, de 0 a 1.
///
/// 0,85 es lo que `R10` §8 T1 supone al dar sus cifras, y se nombra en vez de
/// incrustarse porque **es la suposición más frágil de todo el cálculo**: si en
/// el aparato el código ocupa la mitad del alto en vez del 85 %, los píxeles por
/// módulo caen con él y ninguna versión de QR salva eso.
pub const QR_SHARE_OF_FRAME: f64 = 0.85;

/// Píxeles por módulo por debajo de los cuales `rqrr` deja de leer.
///
/// `R10` §8 T1: ~3 es el suelo absoluto y 4–5 la banda fiable. **Suelo, no
/// objetivo**: un valor que lo roza no es un valor que funcione, es uno que
/// funciona hasta el primer desenfoque.
pub const PIXELS_PER_MODULE_FLOOR: f64 = 3.0;

/// Módulos por lado de una versión de QR, sin la zona de silencio.
///
/// v1 son 21 y cada versión añade 4. Fuera de 1..=40 no hay versión, y devolver
/// `None` es lo honesto: un cálculo sobre una versión inexistente daría un
/// número creíble y sin significado.
#[must_use]
pub const fn modules_of_version(version: u8) -> Option<u32> {
    if version == 0 || version > 40 {
        return None;
    }
    Some(21 + 4 * (version as u32 - 1))
}

/// Píxeles por módulo que da un frame de `frame_height` para esa versión.
///
/// Incluye los **cuatro módulos de zona de silencio por lado** que el QR exige
/// (ocho en total): olvidarlos infla el resultado un 6 % en v27, que es
/// justamente el margen que no hay.
#[must_use]
pub fn pixels_per_module(version: u8, frame_height: u32) -> Option<f64> {
    let modules = modules_of_version(version)?;
    let span = f64::from(modules + 8);
    Some((f64::from(frame_height) * QR_SHARE_OF_FRAME) / span)
}

/// Si esa combinación tiene alguna posibilidad de leerse.
///
/// **Un `true` aquí no promete que se lea**: promete que no está por debajo del
/// suelo. La diferencia importa porque el suelo es exactamente donde ADR-0044
/// dejó la v27 a 640×480.
#[must_use]
pub fn is_above_floor(version: u8, frame_height: u32) -> bool {
    pixels_per_module(version, frame_height).is_some_and(|ppm| ppm >= PIXELS_PER_MODULE_FLOOR)
}
