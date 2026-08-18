//! La superficie C del ojo: píxeles entran, un archivo sale.
//!
//! Especificación: ADR-0048, y el puente que la fase 24B monta —Kotlin saca el
//! plano Y, Dart lo pasa por aquí— sin una línea de `unsafe` nueva fuera de esta
//! frontera, que ya la tenía.
//!
//! **Nada cruza que no sea un entero o bytes en un búfer prestado**, como el
//! resto de esta frontera (ADR-0032). El escaneo vive detrás de un `u64`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing
)]

use std::sync::{Mutex, OnceLock};

use qyro_session::Scanner;

use crate::abi::guard;
use crate::handle::HandleTable;

/// Un argumento que no se puede usar.
const QYRO_ERR_BAD_ARGUMENT: i32 = -2;

/// Un parámetro de salida nulo.
const QYRO_ERR_NULL_OUT: i32 = -3;

/// Todavía no hay archivo: faltan bloques.
///
/// **Un código propio y no `BAD_ARGUMENT`.** «Aún no» es el estado normal de un
/// escaneo durante casi todo su tiempo de vida, y confundirlo con «me llamaste
/// mal» haría que una pantalla enseñara un error mientras todo va bien.
const QYRO_ERR_NOT_READY: i32 = -15;

type Table = HandleTable<Scanner>;

/// La tabla del proceso.
///
/// Un `Mutex` y no un thread-local, por lo mismo que la de sesiones: Dart puede
/// llamar desde cualquier isolate, y un thread-local le daría a cada uno su
/// propia tabla en silencio.
fn table() -> &'static Mutex<Table> {
    static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(HandleTable::new()))
}

/// Abre un escaneo y escribe su identificador en `out_handle`.
///
/// Devuelve 0, o un código negativo. **El identificador sale por parámetro y el
/// estado por el retorno**, que es la forma que ya usa el resto de esta frontera:
/// un valor de retorno que mezcla identificador y error obliga a reservar un
/// identificador para «falló», y ADR-0032 §4 ya reservó el 0 una vez.
///
/// # Safety
///
/// `out_handle` tiene que direccionar un `u64` escribible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_scanner_open(out_handle: *mut u64) -> i32 {
    guard(|| {
        if out_handle.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(handle) = table.insert(Scanner::new()) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        // SAFETY: quien llama promete un `u64` escribible; el caso nulo queda
        // arriba.
        unsafe { out_handle.write(handle) };
        0
    })
}

/// Mira un plano de luma de 8 bits.
///
/// `luma` son exactamente `width * height` bytes, **sin relleno**: quitarlo es
/// trabajo de quien lo saca de la cámara, porque sólo él conoce el `rowStride`.
///
/// Devuelve el código de `ScanState` —0 nada, 1 repetido, 2 progreso, 3
/// completo— o un negativo si el argumento no sirve. **Un frame sin código no es
/// un error**: es 0, y es el caso más común de todos.
///
/// # Safety
///
/// `luma` tiene que direccionar `width * height` bytes legibles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_scanner_look(
    handle: u64,
    luma: *const u8,
    width: usize,
    height: usize,
) -> i32 {
    guard(|| {
        let Some(len) = width.checked_mul(height) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        if len == 0 || luma.is_null() {
            return QYRO_ERR_BAD_ARGUMENT;
        }
        // SAFETY: quien llama promete `width * height` bytes legibles en `luma`;
        // el caso nulo y el vacío quedan arriba.
        let pixels = unsafe { std::slice::from_raw_parts(luma, len) };

        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        match table.get_mut(handle) {
            Ok(scanner) => scanner.look(pixels, width, height).code(),
            Err(_) => QYRO_ERR_BAD_ARGUMENT,
        }
    })
}

/// Cuántos frames se han mirado y cuántos traían un código.
///
/// **Las dos por la misma llamada, y es deliberado.** «300 mirados, 2 leídos» y
/// «300 mirados, 280 leídos» son la misma barra de progreso y dos situaciones
/// opuestas — la primera dice que hay que acercar el teléfono. Dos símbolos
/// distintos dejarían dibujar una sin la otra.
///
/// # Safety
///
/// Los dos punteros tienen que direccionar un `u64` escribible cada uno.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_scanner_tally(
    handle: u64,
    out_seen: *mut u64,
    out_read: *mut u64,
) -> i32 {
    guard(|| {
        if out_seen.is_null() || out_read.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(scanner) = table.get_mut(handle) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let (seen, read) = scanner.tally();
        // SAFETY: los dos punteros son escribibles por contrato y el caso nulo
        // queda arriba.
        unsafe {
            out_seen.write(seen);
            out_read.write(read);
        }
        0
    })
}

/// Cuántos bytes tiene el archivo recibido.
///
/// Devuelve 0 y escribe la longitud cuando está entero; `QYRO_ERR_NOT_READY`
/// mientras falte algo. **Se pregunta antes de pedirlo** para que quien llama
/// reserve el tamaño exacto con `qyro_buffer_alloc`.
///
/// # Safety
///
/// `out_len` tiene que direccionar un `usize` escribible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_scanner_result_len(handle: u64, out_len: *mut usize) -> i32 {
    guard(|| {
        if out_len.is_null() {
            return QYRO_ERR_NULL_OUT;
        }
        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(scanner) = table.get_mut(handle) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Some(bytes) = scanner.finish() else {
            return QYRO_ERR_NOT_READY;
        };
        // SAFETY: escribible por contrato, y el caso nulo queda arriba.
        unsafe { out_len.write(bytes.len()) };
        0
    })
}

/// Copia el archivo en el búfer prestado.
///
/// **Nunca escribe más de `cap`.** Si no cabe devuelve `QYRO_ERR_BAD_ARGUMENT`
/// sin tocar nada: un archivo truncado en silencio falla el hash y nada explica
/// por qué.
///
/// # Safety
///
/// `out` tiene que direccionar `cap` bytes escribibles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qyro_scanner_result(handle: u64, out: *mut u8, cap: usize) -> i32 {
    guard(|| {
        if cap == 0 || out.is_null() {
            return QYRO_ERR_BAD_ARGUMENT;
        }
        let Ok(mut table) = table().lock() else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Ok(scanner) = table.get_mut(handle) else {
            return QYRO_ERR_BAD_ARGUMENT;
        };
        let Some(bytes) = scanner.finish() else {
            return QYRO_ERR_NOT_READY;
        };
        if bytes.len() > cap {
            return QYRO_ERR_BAD_ARGUMENT;
        }
        // SAFETY: quien llama promete `cap` bytes escribibles, y la longitud se
        // comprobó contra `cap` justo arriba.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
        0
    })
}

/// Cierra el escaneo. Un identificador que no existe es un no-op.
#[unsafe(no_mangle)]
pub extern "C" fn qyro_scanner_close(handle: u64) {
    guard(|| {
        if let Ok(mut table) = table().lock() {
            let _ = table.remove(handle);
        }
    });
}
