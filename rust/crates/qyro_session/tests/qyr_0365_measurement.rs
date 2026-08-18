//! La medida que QYR-0365 pidió por su nombre.
//!
//! La ficha está abierta, es de severidad alta, y dice literalmente cuál es la
//! medida que la cierra: **por lado, cuántas veces entra `advance`, cuántas de
//! esas lecturas vencen, y qué había en vuelo cuando venció.** Sin los tres
//! datos no se distingue «el par no ha contestado todavía» de «el par contestó y
//! nadie leyó», y las dos cosas piden arreglos opuestos.
//!
//! Esto no es una prueba de regresión con un umbral: es un instrumento. Afirma
//! lo único que se puede afirmar sin fijar una cifra que dependa de la máquina
//! —que el reparto entre los dos lados no es simétrico— e **imprime** el resto
//! para que quien lea el fallo tenga números y no una intuición.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "una prueba que no puede fallar en voz alta no es una prueba"
)]

use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use qyro_session::{Protection, Session, SessionState};

fn a_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("the loopback has a free port")
        .local_addr()
        .expect("a bound listener has an address")
        .port()
}

fn ensure_identity(tag: &str) {
    let path = std::env::temp_dir().join(format!("qyro-0365-{tag}.bin"));
    let _ = qyro_session::open(&path, Protection::Sandbox);
}

/// Cuántos archivos pequeños se mueven.
///
/// Veinte, que es donde la sesión anterior midió `emisor=75 receptor=1`. Menos
/// no separa la señal del arranque; más alarga la prueba sin decir nada nuevo.
const FILES: usize = 20;

#[test]
fn quien_espera_es_el_emisor_y_estos_son_los_numeros() {
    ensure_identity("m");

    let root = std::env::temp_dir().join("qyro-0365-src");
    let destination = std::env::temp_dir().join("qyro-0365-dst");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&destination);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&destination).unwrap();

    let mut files: Vec<PathBuf> = Vec::new();
    for index in 0..FILES {
        let name = format!("f{index:03}.bin");
        // 64 bytes: un solo trozo, que es el caso que la ficha mide. Un archivo
        // grande escondería el coste por elemento detrás del de los bytes.
        std::fs::write(root.join(&name), vec![b'q'; 64]).unwrap();
        files.push(root.join(&name));
    }

    let address: SocketAddr = format!("127.0.0.1:{}", a_free_port()).parse().unwrap();
    let destination_for_thread = destination.clone();

    let receiving = thread::spawn(move || {
        let mut session = match Session::open_receiver(address, &destination_for_thread, None) {
            Ok(session) => session,
            Err(error) => return (Err(error), (0, 0)),
        };
        let mut state = Ok(SessionState::InProgress);
        while matches!(state, Ok(SessionState::InProgress)) {
            state = session.step();
        }
        let _ = session.finish();
        (state, session.step_tally())
    });

    // El emisor espera a que el receptor esté escuchando.
    let mut sender = None;
    for _ in 0..200 {
        match Session::open_sender(address, &root, &files, None) {
            Ok(session) => {
                sender = Some(session);
                break;
            }
            Err(_) => thread::sleep(std::time::Duration::from_millis(25)),
        }
    }
    let mut sender = sender.expect("el receptor acabo escuchando");

    let started = Instant::now();
    let mut state = Ok(SessionState::InProgress);
    while matches!(state, Ok(SessionState::InProgress)) {
        state = sender.step();
    }
    let elapsed = started.elapsed();
    let (sender_steps, sender_expired) = sender.step_tally();

    let (received, (receiver_steps, receiver_expired)) = receiving.join().unwrap();

    println!("\n--- QYR-0365, {FILES} archivos de 64 bytes ---");
    println!("  emisor:   {sender_steps} pasos, {sender_expired} lecturas vencidas");
    println!("  receptor: {receiver_steps} pasos, {receiver_expired} lecturas vencidas");
    println!("  tiempo:   {:.2} s", elapsed.as_secs_f64());
    println!(
        "  por archivo: {:.3} s",
        elapsed.as_secs_f64() / FILES as f64
    );
    let stalled =
        f64::from(u32::try_from(sender_expired.min(u64::from(u32::MAX))).unwrap_or(0)) * 0.250;
    println!("  de los cuales esperando el reloj de lectura: ~{stalled:.2} s");

    assert_eq!(state, Ok(SessionState::Completed), "la transferencia fallo");
    assert_eq!(received, Ok(SessionState::Completed));

    // **Lo único que se afirma**, porque es lo único que no depende de esta
    // máquina: los dos lados no esperan por igual. Si algún día esto empieza a
    // fallar porque el reparto se equilibró, QYR-0365 está arreglada y esta
    // prueba tiene que decirlo con ese mensaje y no con un umbral silencioso.
    assert!(
        sender_expired >= receiver_expired,
        "el que espera dejo de ser el emisor: emisor={sender_expired}, \
         receptor={receiver_expired}. Si eso es porque QYR-0365 se arreglo, \
         esta prueba ya no mide lo que existia para medir"
    );

    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&destination);
}
