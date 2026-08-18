//! Tragar frames hasta que el archivo esté entero.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use crate::look::Look;
use qyro_fountain::{Decoder, Shape, decode_frame};

/// El ojo. Píxeles entran, un archivo sale.
pub struct Eye {
    decoder: Option<Decoder>,
    shape: Option<Shape>,
    seen_frames: u64,
    read_frames: u64,
}

impl Default for Eye {
    fn default() -> Self {
        Self::new()
    }
}

impl Eye {
    /// Un ojo que todavía no sabe qué va a recibir.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            decoder: None,
            shape: None,
            seen_frames: 0,
            read_frames: 0,
        }
    }

    /// Cuántos frames han pasado por aquí, y cuántos traían un código.
    ///
    /// **Las dos, no una.** «He mirado 300 frames y he leído 2» y «he mirado 300
    /// y he leído 280» son la misma pantalla de progreso y dos situaciones
    /// opuestas: la primera dice que la cámara no está enfocando y la segunda
    /// que va bien. Sin las dos cifras la interfaz no puede distinguirlas, y
    /// quien sostiene el teléfono se queda sin saber si acercarlo.
    #[must_use]
    pub const fn tally(&self) -> (u64, u64) {
        (self.seen_frames, self.read_frames)
    }

    /// La forma del archivo que se está recibiendo, si ya se ha visto un frame.
    #[must_use]
    pub const fn shape(&self) -> Option<Shape> {
        self.shape
    }

    /// Mira un plano de luma de 8 bits.
    ///
    /// `luma` es una fila tras otra, `width` × `height` bytes, donde 0 es negro.
    /// Es exactamente el plano 0 de un `ImageProxy` en `YUV_420_888`, que es lo
    /// que CameraX entrega — **y también lo que produce rasterizar lo que dibuja
    /// `qyro beam`**, que es lo que hace que esto se pueda probar sin cámara.
    ///
    /// Un buffer que no mide `width * height` se trata como [`Look::Nothing`] en
    /// vez de rechazarse: viene de otro proceso a través de JNI, y un frame mal
    /// medido es un frame malo, no una razón para tirar la transferencia.
    pub fn look(&mut self, luma: &[u8], width: usize, height: usize) -> Look {
        self.seen_frames = self.seen_frames.saturating_add(1);

        if width == 0 || height == 0 || luma.len() < width.saturating_mul(height) {
            return Look::Nothing;
        }

        let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(width, height, |x, y| {
            luma.get(y.saturating_mul(width).saturating_add(x))
                .copied()
                .unwrap_or(255)
        });

        let grids = prepared.detect_grids();
        let Some(grid) = grids.first() else {
            return Look::Nothing;
        };

        let mut bytes = Vec::new();
        if grid.decode_to(&mut bytes).is_err() {
            // Un código detectado y no decodificado es un código movido o medio
            // tapado. Pasa constantemente y el siguiente frame lo arregla.
            return Look::Nothing;
        }
        self.read_frames = self.read_frames.saturating_add(1);

        let Ok(frame) = decode_frame(&bytes) else {
            // Un QR que no es de Qyro. En una habitación con carteles, esto es
            // lo normal, no un ataque.
            return Look::Nothing;
        };

        let decoder = match &mut self.decoder {
            Some(decoder) => decoder,
            None => {
                self.shape = Some(frame.shape);
                self.decoder.insert(Decoder::new(frame.shape))
            }
        };

        if !decoder.accept(&frame) {
            return Look::Repeat;
        }

        if decoder.is_complete() {
            Look::Complete
        } else {
            Look::Progress {
                solved: decoder.solved_count(),
                total: frame.shape.blocks(),
            }
        }
    }

    /// El archivo, cuando está entero.
    ///
    /// `None` mientras falte algo — nunca un archivo a medias, que es lo peor
    /// que se puede devolver porque falla el hash y nada explica por qué.
    #[must_use]
    pub fn finish(&self) -> Option<Vec<u8>> {
        self.decoder.as_ref().and_then(Decoder::finish)
    }
}
