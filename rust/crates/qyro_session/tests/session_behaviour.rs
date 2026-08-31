//! What `qyro_session` actually does, exercised rather than read.
//!
//! Phase 01 shipped this crate with six tests, all of them in `guards.rs`, and
//! all six read the production files as *text*: no panicking construct, every
//! file listed, every error variant constructed somewhere. Structural guards are
//! worth having and they caught a real defect — the `AlreadyFailed` variant that
//! nothing could produce — but not one of them opens a socket, and 444 lines of
//! `session.rs` had no test that made the code run (QYR-0309).
//!
//! So this file is the behaviour half. It drives a real sender and a real
//! receiver, in two threads, over a real loopback socket, through the crate's
//! **public** API and nothing else — which is also the surface ADR-0032 §2
//! bounds, so exercising it here is exercising the boundary itself.
//!
//! # Why a reserved port and not port 0
//!
//! The obvious shape is to bind the receiver on port 0 and ask it which port it
//! got. `Session::local_addr` now answers that correctly — it used to hand back
//! `peer_addr`, the *far* end (QYR-0314) — but it still cannot be asked in time:
//! `open_receiver` blocks in `accept` before returning, so by the moment there
//! is a session to ask, a peer has already connected. So the port is reserved
//! the conventional way: bind an ephemeral socket, read the number, drop it. The
//! sender then retries the dial, because the receiver may not have bound yet,
//! and a bounded retry is honest where a sleep is a guess.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use qyro_session::{MAX_FILES_PER_TRANSFER, Progress, Session, SessionError, SessionState};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A directory that removes itself, so a failed assertion cannot leave a file
/// that makes the next run pass.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "qyro-session-{tag}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Writes `len` deterministic bytes without holding them all in memory.
///
/// Deterministic on purpose: the comparison at the end has to be able to say
/// *which* byte differs, and a random file makes a failure unreproducible.
/// Opens a process-wide identity, once, so a session can be built at all.
///
/// **This is the fix of ADR-0040, seen from a test.** Before it, every
/// constructor generated a throwaway keypair and none of these tests needed
/// anything; now a session without an identity is `IdentityUnreadable`, and
/// that refusal is the property. Calling it at the top of each test rather than
/// hiding it in a helper is deliberate: a reader should see that a session
/// requires an identity, because that is the thing that was missing.
///
/// `Protection::Sandbox`, not `Platform`: these run on Linux in CI, where there
/// is no platform wrapper and `Platform` correctly refuses.
fn ensure_identity() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!("qyro-behaviour-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a temporary directory for the test identity");
        qyro_session::open(
            &dir.join("identity.qyro"),
            qyro_session::Protection::Sandbox,
        )
        .expect("opening a test identity");
    });
}

fn write_pattern(path: &Path, len: u64) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let mut file = fs::File::create(path).unwrap();
    let mut written = 0_u64;
    let mut block = [0_u8; 4096];
    while written < len {
        let take = usize::try_from((len - written).min(block.len() as u64)).unwrap();
        for (index, slot) in block.iter_mut().enumerate().take(take) {
            // A byte that depends on its own offset: a transfer that shifted or
            // duplicated a chunk changes the value, not just the length.
            *slot = ((written + index as u64) % 251) as u8;
        }
        file.write_all(&block[..take]).unwrap();
        written += take as u64;
    }
    file.flush().unwrap();
}

fn read_all(path: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

/// A port nothing is listening on, released immediately.
///
/// The window between the drop and the receiver's bind is a race in principle.
/// It is the conventional technique and it is bounded; the alternative — a
/// hard-coded port — collides with whatever else the machine is running, which
/// is a worse race with a worse failure message.
fn a_free_port() -> u16 {
    let probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = probe.local_addr().unwrap().port();
    assert_ne!(port, 0, "port 0 is the request, never the answer");
    port
}

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

/// Dials until the receiver has bound, or gives up with the real error.
///
/// Bounded rather than slept: a sleep long enough to be safe is long enough to
/// be slow, and a sleep short enough to be fast is a flake waiting for a loaded
/// machine.
fn open_sender_when_ready(
    address: SocketAddr,
    root: &Path,
    files: &[PathBuf],
) -> Result<Session, SessionError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match Session::open_sender(address, root, files, None) {
            Err(SessionError::PeerUnreachable) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            other => return other,
        }
    }
}

/// Steps a session to its ending, and reports the last progress seen.
///
/// Returns the terminal state and every distinct `done` value observed, so a
/// caller can assert on the shape of the progression and not only on its end.
fn drive(session: &mut Session) -> (Result<SessionState, SessionError>, Vec<Progress>) {
    let mut seen = vec![session.progress()];
    loop {
        match session.step() {
            Ok(SessionState::InProgress) => {
                let now = session.progress();
                if seen.last() != Some(&now) {
                    seen.push(now);
                }
            }
            other => {
                let now = session.progress();
                if seen.last() != Some(&now) {
                    seen.push(now);
                }
                return (other, seen);
            }
        }
    }
}

/// One file, one root, one transfer, both ends driven to their ending.
struct Moved {
    sent: Result<SessionState, SessionError>,
    received: Result<SessionState, SessionError>,
    materialised: Result<u32, SessionError>,
    sender_progress: Vec<Progress>,
    receiver_progress: Vec<Progress>,
}

fn move_files(root: &Path, files: &[PathBuf], destination: &Path) -> Moved {
    let address = loopback(a_free_port());
    let destination = destination.to_path_buf();

    let receiving = thread::spawn(move || {
        let mut session = match Session::open_receiver(address, &destination, None) {
            Ok(session) => session,
            Err(error) => return (Err(error), Err(error), Vec::new()),
        };
        let (state, progress) = drive(&mut session);
        let materialised = session.finish();
        (state, materialised, progress)
    });

    let mut sender = open_sender_when_ready(address, root, files);
    let (sent, sender_progress) = match sender.as_mut() {
        Ok(session) => drive(session),
        Err(error) => (Err(*error), Vec::new()),
    };

    let (received, materialised, receiver_progress) = receiving.join().unwrap();
    Moved {
        sent,
        received,
        materialised,
        sender_progress,
        receiver_progress,
    }
}

/// Two chunk windows and a bit.
///
/// The engine moves 64 KiB chunks behind a window of 16, so a file has to be
/// larger than 1 MiB before the window, the go-back-N and the flow control are
/// exercised at all. A file that fits in one window would pass a transfer that
/// never refills.
const CROSSES_THE_WINDOW: u64 = 2 * 1024 * 1024 + 7;

// ------------------------------------------------------- arguments, before the wire

#[test]
fn cancelar_se_lo_dice_al_otro_lado() {
    // **«Alguien lo paró» y «se cayó la conexión» son dos frases distintas**, y
    // la fase 25 §5 existe porque confundirlas es el 80 % de los «no funciona».
    //
    // `Session::cancel()` sólo ponía una bandera local: el paso siguiente
    // fallaba aquí y **el par nunca se enteraba**. `request_cancel()` —que emite
    // el frame— existía desde la fase 04 y no la llamaba nada de producción: la
    // undécima capacidad muerta de este proyecto.
    //
    // **El primer intento de esta prueba pasó por el motivo equivocado**: medía
    // tiempos, y al morir el hilo receptor se cerraba el socket, así que el
    // emisor terminaba rápido por un error de conexión y la prueba lo aplaudía.
    // Por eso aquí el receptor **cancela y se queda quieto con el socket
    // abierto**: sin eso no se distingue lo que se quiere distinguir.
    ensure_identity();
    let root = Scratch::new("cancelsrc");
    let destination = Scratch::new("canceldst");
    let file = root.path("algo.bin");
    std::fs::write(&file, vec![b'z'; 4 * 1024 * 1024]).expect("se escribe");

    let address = loopback(a_free_port());
    let target = destination.dir.clone();

    let receiving = thread::spawn(move || {
        let Ok(mut session) = Session::open_receiver(address, &target, None) else {
            return;
        };
        // Handshake, manifiesto, y algo de contenido en marcha.
        let _ = session.step();
        let _ = session.step();
        session.cancel();
        // El paso que lleva la cancelación al cable.
        let _ = session.step();
        // **Quieto, con el socket vivo.** Si el hilo terminara aquí, el emisor
        // vería un socket cerrado y esta prueba no mediría nada.
        thread::sleep(std::time::Duration::from_secs(5));
        drop(session);
    });

    let mut sender = open_sender_when_ready(address, &root.dir, &[file]);
    let mut outcome = Ok(SessionState::InProgress);
    let started = std::time::Instant::now();
    if let Ok(session) = sender.as_mut() {
        while matches!(outcome, Ok(SessionState::InProgress))
            && started.elapsed() < std::time::Duration::from_secs(4)
        {
            outcome = session.step();
        }
    }
    let _ = receiving.join();

    // **Lo que se afirma:** el emisor se entera de que lo cancelaron, y se
    // entera **mientras el socket sigue abierto** — o sea, por el protocolo y no
    // porque el otro proceso se muriera.
    assert!(
        !matches!(outcome, Ok(SessionState::InProgress)),
        "el emisor seguia en marcha cuatro segundos despues de que el receptor          cancelara: la cancelacion no llego al cable"
    );
    assert!(
        !matches!(outcome, Err(SessionError::PeerUnreachable)),
        "el emisor leyo la cancelacion como «el otro aparato no responde», que          es el nombre equivocado: {outcome:?}"
    );

    // **Y con qué nombre se entera**, que es de lo que trata la §5. Se afirma el
    // que sale de verdad para que un cambio de nombre no pase inadvertido: si
    // esto falla, alguien movió el vocabulario y hay que mirar qué lee la
    // persona, no cambiar el número.
    println!("el emisor termino con: {outcome:?}");
    assert!(
        matches!(
            outcome,
            Err(SessionError::Cancelled) | Err(SessionError::TransferRefused)
        ),
        "la cancelacion llego con un nombre inesperado: {outcome:?}"
    );
}

#[test]
fn las_carpetas_no_gastan_el_presupuesto_de_descriptores() {
    // **ADR-0047 §3 dice que el límite es por descriptores**, no por gusto: en
    // Android el selector devuelve descriptores y una selección de miles no es
    // una transferencia lenta, es un proceso agotado.
    //
    // Una carpeta **no abre ningún descriptor**: se crea en el destino y ya. Con
    // ADR-0050 enmienda 1 las carpetas viajan, así que contarlas contra ese
    // presupuesto es contar lo que no cuesta — y un árbol de 200 archivos con 60
    // carpetas se rechazaría por un motivo que no le aplica.
    ensure_identity();
    let root = Scratch::new("presupuesto");

    let mut entries: Vec<PathBuf> = Vec::new();
    for index in 0..MAX_FILES_PER_TRANSFER {
        let name = format!("f{index:04}.bin");
        std::fs::write(root.path(&name), b"x").expect("se escribe");
        entries.push(root.path(&name));
    }
    // Justo en el techo de archivos, y además sesenta carpetas.
    for index in 0..60 {
        let name = format!("d{index:03}");
        std::fs::create_dir_all(root.path(&name)).expect("se crea");
        entries.push(root.path(&name));
    }

    // Nada escucha en esta dirección, así que un `TooManyFiles` demuestra que la
    // negativa ocurrió **antes** de marcar — que es donde ADR-0047 §3 la quiere.
    let outcome = Session::open_sender(loopback(a_free_port()), &root.dir, &entries, None);
    assert!(
        !matches!(outcome, Err(SessionError::TooManyFiles { .. })),
        "sesenta carpetas gastaron un presupuesto que existe por los          descriptores, y una carpeta no abre ninguno"
    );

    // El control: un archivo de más **sí** se rechaza, y por su nombre. Sin
    // esto, quitar el límite entero pasaría la afirmación de arriba.
    let mut demasiados = entries.clone();
    std::fs::write(root.path("uno_mas.bin"), b"x").expect("se escribe");
    demasiados.push(root.path("uno_mas.bin"));
    let outcome = Session::open_sender(loopback(a_free_port()), &root.dir, &demasiados, None);
    assert!(
        matches!(outcome, Err(SessionError::TooManyFiles { .. })),
        "el archivo 257 no se rechazo: {outcome:?}"
    );
}

#[test]
fn an_empty_file_list_is_refused_before_anything_is_dialled() {
    ensure_identity();
    let root = Scratch::new("empty");

    // Nothing is listening on this address. A `BadArgument` therefore proves the
    // refusal happened before the dial: had the check been missing or moved
    // after it, this would be `PeerUnreachable` instead, and the two are
    // distinguishable exactly because nothing is listening.
    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[], None);

    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

#[test]
fn a_file_outside_the_root_is_refused_rather_than_renamed_to_its_last_component() {
    ensure_identity();
    let root = Scratch::new("root");
    let elsewhere = Scratch::new("elsewhere");
    let stray = elsewhere.path("photo.jpg");
    write_pattern(&stray, 32);

    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[stray], None);

    // The quiet failure this guards against is naming the file `photo.jpg` and
    // sending it anyway: the receiver would get a name the sender never chose.
    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

#[test]
fn a_parent_directory_in_the_remainder_is_refused() {
    ensure_identity();
    let root = Scratch::new("parent");
    let nested = root.path("inner");
    fs::create_dir_all(&nested).unwrap();
    write_pattern(&root.path("secret.bin"), 32);

    // `inner/../secret.bin` strips against the root successfully and still is
    // not a plain descendant. qyro_manifest refuses it too; refusing here keeps
    // the error the caller's.
    let escaping = nested.join("..").join("secret.bin");
    let refused = Session::open_sender(loopback(a_free_port()), &root.dir, &[escaping], None);

    assert_eq!(refused.unwrap_err(), SessionError::BadArgument);
}

// ------------------------------------------------------------ the transfer itself

#[test]
fn a_file_crosses_two_sessions_on_the_loopback_and_arrives_byte_for_byte() {
    ensure_identity();
    let source = Scratch::new("send");
    let destination = Scratch::new("recv");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );
    assert_eq!(moved.received, Ok(SessionState::Completed));
    assert_eq!(moved.materialised, Ok(1));

    let arrived = destination.path("payload.bin");
    assert!(arrived.exists(), "the receiver materialised nothing");
    let expected = read_all(&original);
    let actual = read_all(&arrived);
    assert_eq!(
        actual.len(),
        expected.len(),
        "the arrival is a different length from the original"
    );
    assert_eq!(
        actual, expected,
        "the arrival differs from the original byte for byte"
    );

    // And no partial survives its own success.
    assert!(
        !destination.path("payload.bin.qyro-part").exists(),
        "a .qyro-part outlived a completed transfer"
    );
}

#[test]
fn a_corrupted_arrival_would_be_visible_to_this_comparison() {
    ensure_identity();
    // The counter-test for the one above (R2 §1.7). A byte-for-byte comparison
    // that cannot see a changed byte is not evidence that none changed, and a
    // comparison of a file with itself is exactly how this repository has
    // produced five green tests that checked nothing.
    let source = Scratch::new("corrupt-src");
    let destination = Scratch::new("corrupt-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );

    let arrived = destination.path("payload.bin");
    let mut tampered = read_all(&arrived);
    let midpoint = tampered.len() / 2;
    tampered[midpoint] ^= 0x01;

    let expected = read_all(&original);
    assert_eq!(
        tampered.len(),
        expected.len(),
        "flipping a bit must not change the length, or this test proves the wrong thing"
    );
    assert_ne!(
        tampered, expected,
        "a single flipped bit was invisible to the comparison the test above relies on"
    );
}

#[test]
fn two_files_under_a_common_root_arrive_under_their_own_relative_names() {
    ensure_identity();
    // The doc-comment on `open_sender` claims this explicitly: naming by file
    // name alone would send `a.txt` twice and make the receiver arbitrate a
    // collision the sender created. Nothing exercised the claim.
    let source = Scratch::new("names-src");
    let destination = Scratch::new("names-dst");
    let first = source.path("docs/a.txt");
    let second = source.path("notes/a.txt");
    write_pattern(&first, 4096);
    write_pattern(&second, 8192);

    let moved = move_files(
        &source.dir,
        &[first.clone(), second.clone()],
        &destination.dir,
    );

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );
    assert_eq!(moved.materialised, Ok(2));

    let arrived_first = destination.path("docs/a.txt");
    let arrived_second = destination.path("notes/a.txt");
    assert!(arrived_first.exists(), "docs/a.txt did not arrive");
    assert!(arrived_second.exists(), "notes/a.txt did not arrive");

    // Two different sizes on purpose. Had the two names collapsed into one, the
    // survivor would carry one of the two bodies and the other would be absent
    // — a size check distinguishes that from a genuine pair, where an existence
    // check alone would not if both wrote the same bytes.
    assert_eq!(read_all(&arrived_first), read_all(&first));
    assert_eq!(read_all(&arrived_second), read_all(&second));
    assert_ne!(
        read_all(&arrived_first).len(),
        read_all(&arrived_second).len(),
        "the two arrivals are indistinguishable, so this test cannot see a collision"
    );
}

#[test]
fn progress_reaches_the_total_and_never_goes_backwards() {
    ensure_identity();
    let source = Scratch::new("progress-src");
    let destination = Scratch::new("progress-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised {:?}",
        moved.received,
        moved.materialised
    );

    let seen = moved.sender_progress;
    assert!(
        seen.len() > 2,
        "only {} progress samples for a {CROSSES_THE_WINDOW}-byte transfer; a counter \
         that reported its final value once would satisfy a weaker assertion",
        seen.len()
    );
    for pair in seen.windows(2) {
        assert!(
            pair[1].done >= pair[0].done,
            "progress went backwards: {} then {}",
            pair[0].done,
            pair[1].done
        );
    }
    let last = seen.last().unwrap();
    assert_eq!(
        last.total, CROSSES_THE_WINDOW,
        "the total the session reports is not the size of what was asked for"
    );
    assert_eq!(
        last.done, last.total,
        "the transfer completed without progress reaching the total"
    );
    // Trap 4 of the phase document: a transfer test that never checks there was
    // a transfer can be measuring the empty set.
    assert!(last.total > 0, "the total is zero, so nothing was moved");

    // The receiving end learns its total from the manifest rather than from the
    // caller, and that is worth defending on its own: it is the only number a
    // receiver has before any content arrives.
    let received = moved.receiver_progress;
    let receiver_total = received
        .iter()
        .map(|sample| sample.total)
        .max()
        .expect("the receiver was driven at least once");
    assert_eq!(
        receiver_total, CROSSES_THE_WINDOW,
        "the receiver never learned the size the manifest declares"
    );
    assert_eq!(
        received.first().map(|sample| sample.total),
        Some(0),
        "the receiver started out already knowing the total, so this test cannot \
         tell learning it from being handed it"
    );

    // El hueco que esta prueba dejo escrito -«deliberately no assertion on the
    // receiver's done»- lo cierra ahora `el_receptor_cuenta_lo_que_lleva_recibido`.
    // Se deja dicho aqui para que nadie lo vuelva a abrir creyendo que sigue sin
    // poder afirmarse.
}

// --------------------------------------------------- the descriptor-backed sender

/// Opens a sender over handles, retrying the dial exactly like the path version.
///
/// `Vec<(String, File)>` and not `&[..]`, because the session takes ownership of
/// every handle: a retry therefore needs *fresh* handles, which is why this
/// reopens from `sources` on each attempt rather than cloning a list it cannot
/// clone.
fn open_sender_files_when_ready(
    address: SocketAddr,
    sources: &[(String, PathBuf)],
) -> Result<Session, SessionError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut handles = Vec::with_capacity(sources.len());
        for (name, path) in sources {
            handles.push((name.clone(), fs::File::open(path).unwrap()));
        }
        match Session::open_sender_files(address, handles, None) {
            Err(SessionError::PeerUnreachable) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5));
            }
            other => return other,
        }
    }
}

/// The descriptor-backed twin of [`move_files`].
fn move_open_files(sources: &[(String, PathBuf)], destination: &Path) -> Moved {
    let address = loopback(a_free_port());
    let destination = destination.to_path_buf();

    let receiving = thread::spawn(move || {
        let mut session = match Session::open_receiver(address, &destination, None) {
            Ok(session) => session,
            Err(error) => return (Err(error), Err(error), Vec::new()),
        };
        let (state, progress) = drive(&mut session);
        let materialised = session.finish();
        (state, materialised, progress)
    });

    let mut sender = open_sender_files_when_ready(address, sources);
    let (sent, sender_progress) = match sender.as_mut() {
        Ok(session) => drive(session),
        Err(error) => (Err(*error), Vec::new()),
    };

    let (received, materialised, receiver_progress) = receiving.join().unwrap();
    Moved {
        sent,
        received,
        materialised,
        sender_progress,
        receiver_progress,
    }
}

#[test]
fn a_file_opened_by_descriptor_reads_identically_to_one_opened_by_path() {
    ensure_identity();
    // ADR-0034's central claim: Android hands out a descriptor and Windows a
    // path, and the *same bytes* come out either way. Two independent transfers
    // of one source into two destinations, compared against the original and
    // against each other — a comparison of one arrival with itself would be the
    // five-times-repeated anti-pattern of R1 §5.
    let source = Scratch::new("fd-vs-path-src");
    let by_path = Scratch::new("fd-vs-path-a");
    let by_handle = Scratch::new("fd-vs-path-b");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let path_moved = move_files(&source.dir, &[original.clone()], &by_path.dir);
    assert_eq!(
        path_moved.sent,
        Ok(SessionState::Completed),
        "the path-driven transfer did not complete; the receiver ended {:?}",
        path_moved.received
    );

    let handle_moved = move_open_files(
        &[("payload.bin".to_owned(), original.clone())],
        &by_handle.dir,
    );
    assert_eq!(
        handle_moved.sent,
        Ok(SessionState::Completed),
        "the descriptor-driven transfer did not complete; the receiver ended \
         {:?} and materialised {:?}",
        handle_moved.received,
        handle_moved.materialised
    );
    assert_eq!(handle_moved.materialised, Ok(1));

    let arrived_by_path = read_all(&by_path.path("payload.bin"));
    let arrived_by_handle = read_all(&by_handle.path("payload.bin"));
    let expected = read_all(&original);

    // The length first, so a failure says whether the bytes differ or the size
    // does — and so a zero-length pair cannot satisfy the equality below.
    assert_eq!(
        arrived_by_handle.len(),
        expected.len(),
        "the descriptor-driven arrival is a different length from the original"
    );
    assert_eq!(
        arrived_by_handle.len() as u64,
        CROSSES_THE_WINDOW,
        "the arrival is not the size this test wrote, so it is comparing \
         something else"
    );
    assert_eq!(arrived_by_handle, expected);
    assert_eq!(
        arrived_by_path, arrived_by_handle,
        "the two paths through the engine disagree about the bytes"
    );
}

#[test]
fn a_transfer_driven_by_descriptor_arrives_byte_identical() {
    ensure_identity();
    // Two files, two different sizes, two names that only the picker knows: a
    // descriptor carries no name of its own, so the names travelling correctly
    // is a property of this API and of nothing else.
    let source = Scratch::new("fd-two-src");
    let destination = Scratch::new("fd-two-dst");
    let first = source.path("first.bin");
    let second = source.path("second.bin");
    write_pattern(&first, 4096);
    write_pattern(&second, 12_288);

    let moved = move_open_files(
        &[
            ("holiday.jpg".to_owned(), first.clone()),
            ("notes/deep.txt".to_owned(), second.clone()),
        ],
        &destination.dir,
    );

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "the sender did not complete; the receiver ended {:?} and materialised \
         {:?}",
        moved.received,
        moved.materialised
    );
    assert_eq!(moved.materialised, Ok(2));

    // The names the picker chose, not the names on the sender's disk. An
    // implementation that fell back to the file's own name would put
    // `first.bin` here and fail.
    let arrived_first = destination.path("holiday.jpg");
    let arrived_second = destination.path("notes/deep.txt");
    assert!(
        arrived_first.exists(),
        "holiday.jpg did not arrive; the descriptor was named after its source"
    );
    assert!(arrived_second.exists(), "notes/deep.txt did not arrive");
    assert!(
        !destination.path("first.bin").exists(),
        "the source's own file name reached the receiver, so the name the picker \
         chose is not what travelled"
    );

    assert_eq!(read_all(&arrived_first), read_all(&first));
    assert_eq!(read_all(&arrived_second), read_all(&second));
    // Two different sizes on purpose: if the two items had collapsed into one,
    // an existence-and-equality check on identical bodies would not notice.
    assert_ne!(
        read_all(&arrived_first).len(),
        read_all(&arrived_second).len(),
        "the two arrivals are indistinguishable, so this test cannot see a \
         collision between them"
    );
}

#[test]
fn a_revoked_descriptor_mid_transfer_is_a_typed_error_not_a_hang() {
    ensure_identity();
    // ADR-0034 §3 argues that revoking a `content://` permission cannot close a
    // descriptor that is already open, and that *if* a read failed anyway the
    // existing path carries it: `FileSource::read_at` has no error channel, so
    // a failure reads as a short read, the digest does not match, and the
    // session ends `Rejected`.
    //
    // The argument was written and never run. This runs it, modelling the
    // revocation as the observable thing it would look like — the handle stops
    // producing the bytes the manifest already recorded — by truncating the file
    // underneath after the manifest is built. What matters is that the session
    // *ends*: a sender that waits forever for bytes that will never come is a
    // hang, and a hang in the engine is what phase 05's UI would inherit.
    let source = Scratch::new("revoked-src");
    let destination = Scratch::new("revoked-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();
    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let (state, _) = drive(&mut session);
            let materialised = session.finish();
            (state, materialised)
        })
    });

    let mut sender =
        open_sender_files_when_ready(address, &[("payload.bin".to_owned(), original.clone())])
            .unwrap();

    // The manifest already carries the true size and digest. Now the bytes go.
    fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&original)
        .unwrap();
    assert_eq!(
        fs::metadata(&original).unwrap().len(),
        0,
        "the file was not truncated, so this test never revokes anything"
    );

    let (report, outcome) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let (state, _) = drive(&mut sender);
        let _ = report.send(state);
        drop(sender);
    });

    let ending = outcome
        .recv_timeout(Duration::from_secs(60))
        .unwrap_or_else(|_| {
            panic!(
                "the sender never finished. A descriptor that stops producing \
                 bytes must end the session with a typed error, not wait for \
                 them for ever"
            )
        });

    // Either ending is correct and they are genuinely different outcomes: the
    // engine may notice the short item itself, or the receiver may refuse the
    // digest. What must not happen is `Completed`, which would mean a file the
    // sender could no longer read was delivered as good.
    assert_ne!(
        ending,
        Ok(SessionState::Completed),
        "a file that became unreadable mid-transfer was delivered as complete"
    );

    let received = receiving.join().unwrap();
    if let Ok((_, Ok(materialised))) = received {
        assert_eq!(
            materialised, 0,
            "the receiver materialised an item whose bytes never arrived"
        );
    }
    assert!(
        !destination.path("payload.bin").exists(),
        "a truncated transfer left a file the receiver would show as delivered"
    );
}

// ------------------------------------------------------------- the receiver's «no»

#[test]
fn a_receiver_that_refuses_stops_the_sender_and_leaves_nothing_behind() {
    ensure_identity();
    // QYR-0089 and QYR-0088 together, because neither is worth anything alone:
    // a refusal the sender never learns about is a hang, and a refusal that
    // leaves half a file on disk is a lie about what the destination contains.
    //
    // Until this existed the only «no» a receiver could express was `Cancel`,
    // which says «stop what we agreed to do» — a different sentence from «I do
    // not want this», and the receive screen needs the second one.
    let source = Scratch::new("reject-src");
    let destination = Scratch::new("reject-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        let mut session = Session::open_receiver(address, &destination_dir, None)?;
        // One step to take the offer and the manifest, then refuse. Refusing
        // before reading anything would prove a different, easier thing.
        let _ = session.step();
        session.reject(qyro_session::RejectReason::NoRoom)?;
        Ok::<_, SessionError>(())
    });

    let mut sender = open_sender_when_ready(address, &source.dir, &[original.clone()]);
    let (sent, _) = drive(sender.as_mut().expect("the sender opened"));

    receiving.join().unwrap().expect("the receiver refused");

    // 1. The sender **stopped**, and did not complete.
    assert_eq!(
        sent,
        Ok(SessionState::Rejected),
        "a refused transfer ended {sent:?}; a sender that keeps stepping against \
         a peer that already said no is the hang this closes"
    );
    // 2. It learned **why**, and the reason is the one that was sent — not a
    //    default. `NoRoom` was chosen precisely because it is not the first
    //    variant, so a reason that came from `Default` fails here.
    let sender = sender.expect("the sender opened");
    assert_eq!(
        sender.rejection(),
        Some(qyro_session::RejectReason::NoRoom),
        "the sender did not learn the reason it was given"
    );
    assert_ne!(
        sender.rejection(),
        Some(qyro_session::RejectReason::Declined)
    );

    // 3. The destination has **no new file**, checked by listing it rather than
    //    by asking about one name: a `.qyro-part` left behind has a different
    //    name, and a test that only looked for `payload.bin` would not see it.
    let left: Vec<String> = fs::read_dir(&destination.dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        left.is_empty(),
        "the destination is not empty after a refusal: {left:?}"
    );
}

#[test]
fn a_leftover_partial_would_be_visible_to_that_directory_listing() {
    ensure_identity();
    // R2 §1.7 for the assertion above. «The directory is empty» passes for free
    // if the listing cannot see a file, so this puts one there and requires the
    // same listing to report it.
    let destination = Scratch::new("listing-control");
    fs::write(destination.path("payload.bin.qyro-part"), b"half a file").unwrap();

    let left: Vec<String> = fs::read_dir(&destination.dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    assert_eq!(
        left,
        vec!["payload.bin.qyro-part".to_owned()],
        "the listing cannot see a partial, so the emptiness asserted above means \
         nothing"
    );
}

// ------------------------------------------------------------------- the trust half

/// Opens a sender against a fresh receiver and hands back the live session.
///
/// The receiver thread is left parked in `open_receiver`; the caller drops the
/// sender when done and the thread ends with it. Nothing is transferred: what
/// this test family is about happens **after** the handshake and **before** a
/// manifest crosses, which is exactly the window ADR-0035 §3 decides.
fn a_handshaken_sender(source: &Scratch) -> Session {
    let original = source.path("payload.bin");
    if !original.exists() {
        write_pattern(&original, 4096);
    }
    let address = loopback(a_free_port());
    let destination = Scratch::new("trust-dst");
    let destination_dir = destination.dir.clone();

    thread::spawn(move || {
        // A fresh identity per receiver, because `open_receiver` generates one.
        // That is what makes «the key changed» reachable at all here.
        let _ = Session::open_receiver(address, &destination_dir, None);
        // Held so the directory outlives the handshake.
        drop(destination);
    });

    open_sender_when_ready(address, &source.dir, &[original]).unwrap()
}

#[test]
fn a_known_peer_whose_key_changed_is_refused_by_name() {
    ensure_identity();
    // The case that matters. In SSH this is a shouted warning; ADR-0035 §3 says
    // it is one here too, and that it must never soften into `New`.
    //
    // **Rewritten in phase 11, and the reason is the finding.** This test used
    // to build its second identity out of the defect: "a second receiver,
    // therefore a second identity", with an `assert_ne!` on the two
    // fingerprints. That was true only because every session minted a throwaway
    // keypair, so the test passed *because the product was broken* — and once
    // ADR-0040 gave the process one identity, the `assert_ne!` failed.
    //
    // The property is unchanged and worth keeping. What changed is that the
    // second identity is now constructed **on purpose**, which is what the test
    // meant all along, instead of being borrowed from a bug.
    let mut book = qyro_session::TrustBook::new();

    let laptop = qyro_crypto::DeviceIdentity::generate().expect("an identity");
    let impostor = qyro_crypto::DeviceIdentity::generate().expect("a second identity");
    assert_ne!(
        laptop.public_identity().fingerprint(),
        impostor.public_identity().fingerprint(),
        "two generated identities collided, so this test cannot tell a changed          key from an unchanged one"
    );

    book.remember("laptop", laptop.public_identity())
        .expect("remembering a peer");
    assert_eq!(
        book.verdict("laptop", laptop.public_identity()).unwrap(),
        qyro_session::PeerTrust::Known,
        "the peer that was just remembered is not recognised, so the verdict          below would mean nothing"
    );

    let verdict = book
        .verdict("laptop", impostor.public_identity())
        .expect("a verdict for a known name");
    assert_eq!(
        verdict,
        qyro_session::PeerTrust::Changed,
        "a peer whose key changed reported {verdict:?}"
    );
    // And specifically **not** `New`, which is the softening this guards
    // against: `New` asks a person, `Changed` refuses.
    assert_ne!(verdict, qyro_session::PeerTrust::New);
}

#[test]
fn forgetting_a_peer_makes_it_new_again_and_not_trusted() {
    ensure_identity();
    // The only way back from `Changed`, and it has to be an explicit act.
    let source = Scratch::new("forget-src");
    let mut book = qyro_session::TrustBook::new();

    let session = a_handshaken_sender(&source);
    session.remember_peer(&mut book, "phone").unwrap();
    assert_eq!(book.names(), vec!["phone".to_owned()]);
    assert_eq!(
        session.peer_trust(&book, "phone").unwrap(),
        qyro_session::PeerTrust::Known
    );

    assert!(
        book.forget("phone"),
        "forget said there was nothing to forget"
    );
    assert!(book.is_empty());
    assert_eq!(
        session.peer_trust(&book, "phone").unwrap(),
        qyro_session::PeerTrust::New,
        "a forgotten peer is not new again"
    );
    // Forgetting twice is not an error and is not a lie either.
    assert!(!book.forget("phone"));

    drop(session);
}

#[test]
fn the_fingerprint_the_session_shows_matches_the_one_the_store_recorded() {
    ensure_identity();
    // Two paths, not one call twice: the left side reads the identity the
    // handshake authenticated on this session, the right side reads the copy
    // the book stored under a name. They are the same value arrived at through
    // different objects, which is the only version of this assertion worth
    // making.
    let source = Scratch::new("fp-src");
    let mut book = qyro_session::TrustBook::new();
    let session = a_handshaken_sender(&source);

    session.remember_peer(&mut book, "desk").unwrap();
    let shown = session.peer_fingerprint();
    let recorded = book.fingerprint_of("desk").expect("the peer was recorded");

    assert_eq!(shown, recorded);
    // The format is the core's, and it is not empty or a placeholder: grouped
    // hex with separators, so a `String::new()` on either side fails here.
    assert!(shown.contains('-'), "{shown} is not the grouped form");
    assert!(
        shown.len() >= 32,
        "{shown} is too short to be a fingerprint"
    );
    // And a *different* identity's fingerprint differs, so the equality above
    // is not satisfied by every pair of strings this code can produce.
    //
    // Phase 11: this control used to open a second session and assert its
    // fingerprint differed — which was only ever true because each session
    // minted its own keypair. Both ends of a loopback session now share the one
    // process identity, so the second session is the *same* peer and the
    // assertion was measuring the defect, not the property. Built on purpose
    // instead.
    let stranger = qyro_crypto::DeviceIdentity::generate().expect("a second identity");
    let mut other_book = qyro_session::TrustBook::new();
    other_book
        .remember("stranger", stranger.public_identity())
        .expect("remembering a stranger");
    assert_ne!(
        other_book.fingerprint_of("stranger").expect("recorded"),
        shown
    );

    drop(session);
}

#[test]
fn a_name_the_peer_store_refuses_is_refused_here_too() {
    ensure_identity();
    // One validator, not two. The rules live in qyro_identity_store and this
    // crate asks them rather than restating them, so a rule can never hold in
    // one place and not the other.
    let source = Scratch::new("name-src");
    let mut book = qyro_session::TrustBook::new();
    let session = a_handshaken_sender(&source);

    for bad in ["", "with\u{0}a control", "with\u{7f}another"] {
        assert_eq!(
            session.remember_peer(&mut book, bad),
            Err(SessionError::BadArgument),
            "{bad:?} was accepted as a peer name"
        );
    }
    assert!(book.is_empty(), "a refused name still entered the book");
    // The control: a name with nothing wrong is accepted, so the refusals above
    // are about the names and not about the method refusing everything.
    assert!(session.remember_peer(&mut book, "kitchen tablet").is_ok());

    drop(session);
}

// ------------------------------------------------------------- the progress bridge

/// Sends `size` bytes with an observer attached, and counts both.
///
/// Returns every emission and **how many times `step` was called**. The two
/// numbers together are what makes the budget measurable: a counter that only
/// knew emissions could not tell a budget from an implementation that emits
/// once and stops.
fn move_observed(size: u64) -> (Vec<Progress>, usize, Result<SessionState, SessionError>) {
    let source = Scratch::new("budget-src");
    let destination = Scratch::new("budget-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, size);

    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();
    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let (state, _) = drive(&mut session);
            let _ = session.finish();
            state
        })
    });

    let seen = Arc::new(Mutex::new(Vec::new()));
    // Rebuilt on every retry rather than probed with a throwaway connection: the
    // receiver accepts exactly once, so a probe that succeeded would consume the
    // session under test.
    let recorder_for = |shared: &Arc<Mutex<Vec<Progress>>>| -> Box<dyn FnMut(Progress) + Send> {
        let recorder = Arc::clone(shared);
        Box::new(move |progress| {
            if let Ok(mut log) = recorder.lock() {
                log.push(progress);
            }
        })
    };

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sender = loop {
        let attempt = Session::open_sender(
            address,
            &source.dir,
            &[original.clone()],
            Some(recorder_for(&seen)),
        );
        match attempt {
            Err(SessionError::PeerUnreachable) if Instant::now() < deadline => {
                // A refused dial emitted nothing, so the log is still empty and
                // the retry starts clean.
                thread::sleep(Duration::from_millis(5));
            }
            other => break other,
        }
    };
    let outcome = match sender.as_mut() {
        Ok(session) => {
            let mut steps = 0_usize;
            let state = loop {
                steps += 1;
                match session.step() {
                    Ok(SessionState::InProgress) => {}
                    other => break other,
                }
            };
            (state, steps)
        }
        Err(error) => (Err(*error), 0),
    };
    let _ = receiving.join();
    let emissions = seen.lock().map(|log| log.clone()).unwrap_or_default();
    (emissions, outcome.1, outcome.0)
}

#[test]
fn a_session_without_an_observer_still_completes() {
    ensure_identity();
    // ADR-0033 §2: «no observer» must never be a second code path. Every other
    // test in this file passes None, so this one states the property by name
    // rather than leaving it implied.
    let source = Scratch::new("noobs-src");
    let destination = Scratch::new("noobs-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original.clone()], &destination.dir);

    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "a session with no observer did not complete; the receiver ended {:?}",
        moved.received
    );
    assert_eq!(
        read_all(&destination.path("payload.bin")),
        read_all(&original)
    );
}

#[test]
fn the_callback_budget_is_respected_for_a_known_file_size() {
    ensure_identity();
    let (small, _, small_state) = move_observed(512 * 1024);
    let (large, _, large_state) = move_observed(4 * 1024 * 1024);
    assert_eq!(small_state, Ok(SessionState::Completed));
    assert_eq!(large_state, Ok(SessionState::Completed));

    // The upper bound of ADR-0033 §4: 100 stepped emissions, plus the opening
    // one, plus the terminal one.
    const CEILING: usize = 102;
    assert!(
        large.len() <= CEILING,
        "{} emissions for 4 MiB, over the budget of {CEILING}",
        large.len()
    );

    // Two sizes and a strict inequality, which is the shape that tells a
    // measured counter from a constant (R1 §5.6). Below the 25 MiB elbow the
    // 256 KiB floor decides, so eight times the bytes must emit strictly more.
    // An implementation that always emitted 102 -- or always 2 -- satisfies the
    // ceiling above and fails here.
    assert!(
        large.len() > small.len(),
        "512 KiB emitted {} and 4 MiB emitted {}; a budget that does not grow \
         with the file below the floor is not measuring the file",
        small.len(),
        large.len()
    );

    // And it is a real progression, not a repeated number.
    let last = large
        .last()
        .expect("a completed transfer emitted something");
    assert_eq!(last.done, last.total, "the last emission is not the total");
    assert_eq!(
        large.first().map(|first| first.done),
        Some(0),
        "the opening emission did not start at zero"
    );
    for pair in large.windows(2) {
        assert!(
            pair[1].done >= pair[0].done,
            "an emission went backwards: {} then {}",
            pair[0].done,
            pair[1].done
        );
    }
}

#[test]
fn an_emission_per_chunk_would_be_visible_to_this_measurement() {
    ensure_identity();
    // R2 §1.7, and the reason the helper counts `step` calls as well as
    // emissions. The budget exists to stop the emission count tracking the
    // chunk count; the way to know this measurement could see that failure is
    // to compare it against a number that *does* track the chunks.
    //
    // `step` is the loop that moves chunks, so an implementation that emitted
    // once per step would make the two counts equal. They must not be.
    let (emissions, steps, state) = move_observed(4 * 1024 * 1024);
    assert_eq!(state, Ok(SessionState::Completed));

    assert!(
        steps > 0,
        "no step ran, so this measurement is comparing two zeros"
    );
    assert!(
        emissions.len() < steps,
        "{} emissions for {steps} steps. Equal counts is exactly what emitting \
         once per chunk looks like, so this measurement would report it",
        emissions.len()
    );
    // Not merely fewer -- decisively fewer. One emission short of per-step would
    // satisfy a bare `<` while being the failure this guards against.
    assert!(
        emissions.len() * 2 < steps,
        "{} emissions for {steps} steps is within a factor of two of one per \
         step, which is not a budget",
        emissions.len()
    );
}

// --------------------------------------------------------------- endings and state

#[test]
fn a_cancelled_session_reports_cancelled_and_keeps_reporting_it() {
    ensure_identity();
    let source = Scratch::new("cancel");
    let original = source.path("payload.bin");
    write_pattern(&original, 4096);
    let address = loopback(a_free_port());
    let destination = Scratch::new("cancel-dst");
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let _ = drive(&mut session);
        })
    });

    let mut sender = open_sender_when_ready(address, &source.dir, &[original]).unwrap();
    assert!(
        !sender.is_cancelled(),
        "a fresh session is already cancelled"
    );

    sender.cancel();
    assert!(sender.is_cancelled(), "cancel did not raise the flag");

    // ADR-0032 §5 freezes stickiness as *returning the same code*. A second Ok
    // would let a caller believe the session recovered.
    assert_eq!(sender.step(), Err(SessionError::Cancelled));
    assert_eq!(sender.step(), Err(SessionError::Cancelled));
    assert_eq!(sender.step(), Err(SessionError::Cancelled));

    drop(sender);
    let _ = receiving.join();
}

#[test]
fn a_receiver_reports_the_port_it_bound_and_not_the_one_the_peer_dialled_from() {
    ensure_identity();
    // Half of QYR-0314: `local_addr` returned `peer_addr` -- the *far* end --
    // and nothing noticed, because the C surface does not expose it and no test
    // called it.
    //
    // The two ports are distinguishable on purpose: a dialling socket gets an
    // ephemeral port the system picks, so an implementation that still handed
    // back the peer's address fails the last assertion rather than looking
    // plausible.
    let source = Scratch::new("addr-src");
    let destination = Scratch::new("addr-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, 4096);
    let port = a_free_port();
    let address = loopback(port);
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|session| {
            let local = session.local_addr();
            let peer = session.progress();
            let _ = peer;
            local
        })
    });

    let sender = open_sender_when_ready(address, &source.dir, &[original]).unwrap();
    let sender_local = sender.local_addr().unwrap();

    let reported = receiving.join().unwrap().unwrap().unwrap();

    assert_eq!(
        reported.port(),
        port,
        "the receiver reported {} where it bound {port}",
        reported.port()
    );
    assert_ne!(
        reported.port(),
        0,
        "port 0 is the request, never the answer"
    );
    assert_ne!(
        reported.port(),
        sender_local.port(),
        "the receiver reported the port the sender dialled from, which is what \
         handing back `peer_addr` looks like"
    );
    drop(sender);
}

#[test]
fn a_receiver_that_never_gets_a_peer_reports_the_peer_and_not_success() {
    ensure_identity();
    let destination = Scratch::new("no-peer");
    let address = loopback(a_free_port());

    let receiving = thread::spawn(move || Session::open_receiver(address, &destination.dir, None));

    // Connect and hang up without speaking. The receiver must not treat a
    // socket that opened and closed as an authenticated peer.
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(socket) = std::net::TcpStream::connect(address) {
            drop(socket);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let outcome = receiving.join().unwrap();
    assert!(
        outcome.is_err(),
        "a peer that said nothing produced an open session"
    );
    let error = outcome.err().unwrap();
    assert!(
        error == SessionError::PeerUnreachable || error == SessionError::NotAuthenticated,
        "a silent peer produced {error:?}, which is neither of the two endings that mean \
         the handshake did not happen"
    );
}

#[test]
fn finishing_a_sender_materialises_nothing_and_says_so() {
    ensure_identity();
    // `finish` is the receiver's operation. A sender answering anything but zero
    // would mean the count came from somewhere other than materialised items.
    let source = Scratch::new("finish-src");
    let destination = Scratch::new("finish-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, 4096);
    let address = loopback(a_free_port());
    let destination_dir = destination.dir.clone();

    let receiving = thread::spawn(move || {
        Session::open_receiver(address, &destination_dir, None).map(|mut session| {
            let (state, _) = drive(&mut session);
            let _ = session.finish();
            state
        })
    });

    let mut sender = open_sender_when_ready(address, &source.dir, &[original]).unwrap();
    let (sent, _) = drive(&mut sender);
    assert_eq!(sent, Ok(SessionState::Completed));

    assert_eq!(
        sender.finish(),
        Ok(0),
        "a sending session reported materialised items"
    );

    let _ = receiving.join();
}

/// La barra del receptor, que **estaba congelada en cero hasta el final**.
///
/// `Role::Receiving` asignaba `total` al llegar el manifiesto y **nunca tocaba
/// `done`**. Quien recibe veia 0 % durante toda la transferencia y un salto a
/// 100 % al acabar, que para un archivo grande es indistinguible de «esto se ha
/// colgado». Y no era invisible: la prueba de arriba lo dejo escrito y siguio
/// ahi.
///
/// Mira las **emisiones**, que es lo que la barra dibuja, y no un total final
/// que un contador roto tambien acertaria.
#[test]
fn el_receptor_cuenta_lo_que_lleva_recibido() {
    ensure_identity();
    let source = Scratch::new("recv-progress-src");
    let destination = Scratch::new("recv-progress-dst");
    let original = source.path("payload.bin");
    write_pattern(&original, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[original], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "el emisor no completo"
    );

    let seen = moved.receiver_progress;

    // El control, antes de creer nada: sin muestras suficientes esto no mediria
    // el progreso sino su ausencia.
    assert!(
        seen.len() > 2,
        "solo {} muestras en el receptor para {CROSSES_THE_WINDOW} bytes, asi que          esta prueba no esta midiendo lo que cree",
        seen.len()
    );

    let avanzando = seen
        .iter()
        .any(|sample| sample.done > 0 && sample.total > 0 && sample.done < sample.total);
    assert!(
        avanzando,
        "ninguna muestra del receptor tenia progreso intermedio: la barra se queda          en cero hasta el final, y quien recibe no distingue eso de un cuelgue.          Muestras: {seen:?}"
    );

    for pair in seen.windows(2) {
        assert!(
            pair[1].done >= pair[0].done,
            "el progreso del receptor retrocedio: {} y luego {}",
            pair[0].done,
            pair[1].done
        );
    }

    let last = seen.last().expect("al menos una muestra");
    assert_eq!(
        last.done, last.total,
        "el receptor termino sin que su cuenta llegara al total declarado"
    );
}

/// **Que archivo se esta moviendo**, que iba siempre a cero.
///
/// ADR-0050 §4.1 pide «archivo N de M». El campo `item` existe en la frontera C
/// desde la fase 02 y ninguno de los dos extremos lo asignaba jamas: QYR-0318 lo
/// documento como «siempre cero» en vez de arreglarlo, que es describir un
/// defecto con precision y dejarlo donde estaba.
#[test]
fn el_progreso_dice_por_que_archivo_va() {
    ensure_identity();
    let source = Scratch::new("item-src");
    let destination = Scratch::new("item-dst");
    let uno = source.path("uno.bin");
    let dos = source.path("dos.bin");
    write_pattern(&uno, CROSSES_THE_WINDOW);
    write_pattern(&dos, CROSSES_THE_WINDOW);

    let moved = move_files(&source.dir, &[uno, dos], &destination.dir);
    assert_eq!(
        moved.sent,
        Ok(SessionState::Completed),
        "el emisor no completo"
    );

    for (quien, seen) in [
        ("emisor", moved.sender_progress),
        ("receptor", moved.receiver_progress),
    ] {
        let mayor = seen.iter().map(|sample| sample.item).max().unwrap_or(0);
        assert_eq!(
            mayor, 2,
            "el {quien} nunca dijo ir por el segundo de dos archivos: el mayor              `item` que emitio fue {mayor}"
        );
        assert!(
            seen.iter().any(|sample| sample.item == 1),
            "el {quien} nunca dijo ir por el primero, asi que salta al ultimo y              no cuenta: {seen:?}"
        );
        for pair in seen.windows(2) {
            assert!(
                pair[1].item >= pair[0].item,
                "el {quien} retrocedio de archivo: {} y luego {}",
                pair[0].item,
                pair[1].item
            );
        }
    }
}

/// A port already in use has its own answer, and it is not «bad argument».
///
/// **ADR-0041 §3 decided this behaviour and the code did not have the vocabulary
/// for it.** The ADR says, in those words: *«Si el puerto está ocupado: se dice,
/// no se mueve. Qyro dice qué puerto está ocupado y ofrece elegir otro.»* What
/// `open_receiver` actually did was
/// `Listener::bind(bind).map_err(|_| SessionError::BadArgument)` — every reason a
/// bind can fail collapsed into the one variant whose message is «the address,
/// port or path was not usable». Nothing downstream could tell «that port is
/// taken» from «that path is wrong», so nothing downstream could offer another
/// port.
///
/// **This is not hypothetical on the machine this is for.** Windows reserves
/// port ranges for Hyper-V, WSL2 and Docker (`netsh interface ipv4 show
/// excludedportrange protocol=tcp`), and a bind inside one fails with
/// `WSAEACCES` — **10013**, «permission denied» — not with «address in use». A
/// person who installed Docker once, two years ago, gets a receiver that refuses
/// to start and a message about an argument they never passed.
///
/// So both kinds map here: `AddrInUse` and `PermissionDenied`. They are the same
/// fact to the person holding the machine — *this port is not yours today* — and
/// the answer to both is the same: pick another one.
#[test]
fn a_port_that_is_taken_says_so_instead_of_blaming_the_arguments() {
    let occupier = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port to occupy");
    let taken = occupier.local_addr().expect("the port it took");
    let scratch = Scratch::new("port-taken");
    ensure_identity();

    // `open_receiver` blocks in `accept` once it binds, so this call is only
    // ever safe in a test *because* the bind fails. That is also why the happy
    // path has no assertion here.
    let Err(refusal) = qyro_session::Session::open_receiver(taken, &scratch.dir, None) else {
        panic!("binding a port somebody else holds cannot succeed");
    };

    assert_eq!(
        refusal,
        qyro_session::SessionError::PortUnavailable,
        "a taken port came back as {refusal:?}, so nothing above this can offer \
         another port -- which is what ADR-0041 §3 asks for"
    );
}

/// The control: a bind that fails for a reason that **is** the caller's fault
/// still says so.
///
/// Without this, mapping every bind failure to `PortUnavailable` would satisfy
/// the test above and would tell a person to try another port when the address
/// they typed does not exist on this machine.
#[test]
fn but_an_address_this_machine_does_not_have_is_still_a_bad_argument() {
    let scratch = Scratch::new("address-absent");
    ensure_identity();
    // RFC 5737 documentation address. It is not assigned to any interface here,
    // so the bind fails with `AddrNotAvailable` — a different fact, and one no
    // amount of choosing another port fixes.
    let elsewhere: std::net::SocketAddr = "192.0.2.1:49517".parse().expect("a literal address");

    let Err(refusal) = qyro_session::Session::open_receiver(elsewhere, &scratch.dir, None) else {
        panic!("binding an address this machine does not hold cannot succeed");
    };

    assert_eq!(refusal, qyro_session::SessionError::BadArgument);
}

/// A receiver knows **what** it is being asked to accept before it is asked.
///
/// **ADR-0036 §1 and QYR-0364, measured instead of assumed.** QYR-0364 is
/// recorded as closed with the words «una pregunta sin objeto es una formalidad,
/// no una decisión»; running `qyro recv` against a real sender prints, verbatim:
///
/// ```text
///   someone connected. They say they are:
///     b76c0bb3-034672e9-4c9ab47b-632ddcc0
///   they have not said what they are sending yet.
///   accept from this device? [y/N]
/// ```
///
/// The question with no object was still there. Not because `offered_files` is
/// broken — it reads the manifest correctly — but because of **when** it is
/// called: `open_receiver` returns as soon as the handshake completes, and the
/// offer and the manifest arrive later.
///
/// **How much later, measured: two steps, not one.** After the first the
/// manifest is not there and `progress().total` is **0**; after the second both
/// are. That number is the whole finding, because the Dart side takes exactly
/// one `stepBlocking()` and then asks the person to decide, passing
/// `progress.total` along — so the dialog on the phone offered «0 bytes» and no
/// names at all.
///
/// [`Session::await_offer`] is where that number now lives, once, so neither
/// consumer has to know it. This test pins both halves: **nothing before it,
/// everything after it.** Without the first half the fix would be unnecessary;
/// without the second it would not work.
#[test]
fn what_is_offered_is_unknown_until_await_offer_and_known_after_it() {
    ensure_identity();
    let scratch = Scratch::new("offered-when");
    let root = scratch.path("origen");
    let destination = scratch.path("destino");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&destination).unwrap();

    let names = ["foto.jpg", "informe.pdf"];
    let files: Vec<PathBuf> = names
        .iter()
        .map(|name| {
            let path = root.join(name);
            write_pattern(&path, 4096);
            path
        })
        .collect();

    let address = loopback(a_free_port());
    let seen = std::sync::Arc::new(std::sync::Mutex::new((Vec::new(), Vec::new(), 0_u64)));
    let recorder = std::sync::Arc::clone(&seen);
    let destination_for_thread = destination.clone();

    let receiving = thread::spawn(move || {
        let mut session =
            Session::open_receiver(address, &destination_for_thread, None).expect("a receiver");

        // Exactly where the CLI used to ask, and where it used to get nothing.
        let before = session.offered_files();
        let _ = session.await_offer();
        let after = session.offered_files();
        let total = session.progress().total;
        *recorder.lock().unwrap() = (before, after, total);

        let _ = drive(&mut session);
        let _ = session.finish();
    });

    let mut sender = open_sender_when_ready(address, &root, &files).expect("a sender");
    let _ = drive(&mut sender);
    receiving.join().unwrap();

    let (before, after, total) = seen.lock().unwrap().clone();

    assert!(
        before.is_empty(),
        "the manifest was already there before await_offer, so «they have not \
         said what they are sending yet» had a different cause than this test \
         claims: {before:?}"
    );

    let offered: Vec<&str> = after.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        offered, names,
        "after await_offer the receiver still cannot say what it is being \
         offered, so neither consumer can ask a question with an object"
    );
    for (name, size) in &after {
        assert_eq!(*size, 4096, "{name} came back with the wrong size");
    }
    // The number the phone's dialog shows. It was 0, and 0 is not «unknown» to
    // somebody reading it -- it is «nothing», which is a different lie.
    assert_eq!(
        total, 8192,
        "progress().total is {total} at the moment of asking, so the dialog \
         offers a number that is not the size of what is arriving"
    );
}

/// A name that is already taken is refused, and **the refusal is a value**.
///
/// **QYR-0374, found by running it three times.** Sending the same file twice to
/// the same folder gives, on the second go: a progress bar that reaches 100 %,
/// and then `0 file(s) saved in .` — with no reason anywhere. The wire transfer
/// really did complete; what failed was the last step, `finish`, refusing to
/// overwrite a file that was already there.
///
/// **Refusing is right.** ADR-0027 §4: nothing overwrites, ever. What is wrong is
/// that both consumers threw the reason away — the CLI with `unwrap_or(0)`, the
/// Dart worker with a bare `on QyroSessionFailure {}` in a `finally` whose
/// comment argued that the ending had already said everything. It had not: the
/// ending is `Completed`, because the **transfer** completed. The refusal comes
/// from the filesystem, which the ending knows nothing about — so the phone
/// reported **«delivered»** with nothing on disk, which is the exact shape of
/// QYR-0357 that the comment above it claims to have closed.
///
/// This test pins the engine half: the refusal must arrive as
/// `StorageRefused`, distinguishable from «zero files were offered». Both are
/// «nothing was written» to a caller that only looks at the count.
#[test]
fn a_second_arrival_under_a_taken_name_refuses_with_a_reason_and_not_a_zero() {
    ensure_identity();
    let scratch = Scratch::new("collision");
    let root = scratch.path("origen");
    let destination = scratch.path("destino");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&destination).unwrap();

    let source = root.join("informe.pdf");
    write_pattern(&source, 8192);
    let files = vec![source.clone()];

    // First time: it lands.
    let first = move_files(&root, &files, &destination);
    assert_eq!(
        first.materialised,
        Ok(1),
        "the first arrival did not land, so the second proves nothing"
    );

    // Second time, same name, same folder.
    let second = move_files(&root, &files, &destination);

    assert_eq!(
        second.received,
        Ok(SessionState::Completed),
        "the transfer itself should complete: the bytes cross fine and it is \
         only the last step that refuses. If this changed, the message the \
         person sees changes with it"
    );
    assert_eq!(
        second.materialised,
        Err(SessionError::StorageRefused),
        "a name that is already taken came back as {:?}. A caller that only \
         reads the count cannot tell «refused because the name is taken» from \
         «nothing was offered», and both faces printed the second",
        second.materialised
    );

    // And nothing was damaged: the original is intact and no part file is left.
    let landed = read_all(&destination.join("informe.pdf"));
    assert_eq!(
        landed.len(),
        8192,
        "the first file was truncated or replaced"
    );
    let leftovers: Vec<_> = fs::read_dir(&destination)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".qyro-part"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a refused arrival left {leftovers:?} behind, so the folder fills up \
         with parts nobody can open"
    );
}

/// The fingerprint a pairing string carries can be read back, and it is right.
///
/// **QYR-0381.** `parse_pairing` returned the address and threw the fingerprint
/// away, under a doc comment that said the half it discarded «is an expectation
/// to check against the authenticated fingerprint». Nothing checked it, and
/// nothing could: no caller had any way to get it back — not the CLI, not
/// `qyro_pairing_parse`, which emits the address and nothing else.
///
/// So a code **typed by hand**, compared character by character by a person,
/// established **less** than typing a bare `ip:port` and adding `--expect`. The
/// expensive half of pairing happened and bought nothing.
///
/// ADR-0035 §2.1 is explicit about what it should buy: a fingerprint that does
/// not match the authenticated one refuses the session **without asking anybody**.
#[test]
fn a_pairing_string_hands_back_the_fingerprint_it_carries() {
    let code = "QYRO1|192.168.1.9:49517|ab12cd34ab12cd34ab12cd34ab12cd34";

    assert_eq!(
        qyro_session::pairing_fingerprint(code),
        Ok("ab12cd34ab12cd34ab12cd34ab12cd34".to_owned()),
        "the expectation the person typed cannot be read back, so nothing can \
         compare it with the fingerprint the handshake authenticated"
    );

    // The two never disagree about what is a pairing string. If they could, a
    // caller would get an address for a string whose fingerprint it cannot read,
    // and would then dial with no expectation at all — silently.
    assert!(qyro_session::parse_pairing(code).is_ok());
    for broken in [
        "QYRO1|192.168.1.9:49517|ab12",
        "QYRO1|192.168.1.9|ab12cd34ab12cd34ab12cd34ab12cd34",
        "NOTQYRO|192.168.1.9:49517|ab12cd34ab12cd34ab12cd34ab12cd34",
        "192.168.1.9:49517",
        "",
    ] {
        assert_eq!(
            qyro_session::pairing_fingerprint(broken).is_ok(),
            qyro_session::parse_pairing(broken).is_ok(),
            "the two parsers disagree about {broken:?}"
        );
    }

    // And the control: a different code gives a different expectation. A
    // function that returned a constant would satisfy everything above.
    assert_ne!(
        qyro_session::pairing_fingerprint(code),
        qyro_session::pairing_fingerprint(
            "QYRO1|192.168.1.9:49517|00112233445566778899aabbccddeeff"
        )
    );
}

/// A zero-byte file does not take the rest of the transfer down with it.
///
/// **Reported by the wire audit as `empty-item-never-materialises`, and worth a
/// test either way: an empty file is not exotic.** A `.gitkeep`, a lock file, a
/// log that has not been written to yet — people send folders, and folders have
/// them.
///
/// The claim under test is the sharp half: not «the empty one is missing», which
/// would be a small loss, but «`finish` gives up at it and **everything after it
/// in the manifest is abandoned too**», which loses files that crossed perfectly.
#[test]
fn an_empty_file_does_not_abandon_the_files_that_follow_it() {
    ensure_identity();
    let scratch = Scratch::new("empty-item");
    let root = scratch.path("origen");
    let destination = scratch.path("destino");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&destination).unwrap();

    // The empty one **first**, so that anything that gives up at it gives up
    // with the others still to come. Ordering the test around the failure is
    // the whole point: with the empty one last, a broken `finish` would look
    // fine.
    let empty = root.join("a-vacio.txt");
    fs::write(&empty, b"").unwrap();
    let after = root.join("b-lleno.bin");
    write_pattern(&after, 4096);
    let also = root.join("c-lleno.bin");
    write_pattern(&also, 8192);

    let moved = move_files(
        &root,
        &[empty.clone(), after.clone(), also.clone()],
        &destination,
    );

    assert_eq!(
        moved.received,
        Ok(SessionState::Completed),
        "the transfer did not complete: {:?}",
        moved.received
    );

    let landed = |name: &str| destination.join(name).exists();
    assert!(
        landed("b-lleno.bin") && landed("c-lleno.bin"),
        "the two files that follow an empty one did not arrive, so one empty \
         file took the rest of the transfer down with it. Destination holds: \
         {:?}. materialised={:?}",
        fs::read_dir(&destination)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect::<Vec<_>>(),
        moved.materialised
    );
    assert_eq!(read_all(&destination.join("b-lleno.bin")).len(), 4096);
    assert_eq!(read_all(&destination.join("c-lleno.bin")).len(), 8192);

    // And the empty one itself. Reported separately so a failure says which of
    // the two problems it is.
    assert!(
        landed("a-vacio.txt"),
        "the empty file did not arrive. On its own that is a small loss; the \
         assertion above is the one that matters"
    );
    assert_eq!(read_all(&destination.join("a-vacio.txt")).len(), 0);
}

/// How many descriptors a batch of two hundred files holds open **at once**.
///
/// **FASE-28 §4, pregunta 2, contestada con un número y no con una lectura.**
/// The question is not rhetorical: ADR-0047 §3 caps a transfer at 256 files
/// *because of* descriptors, and on Windows the CRT's default is 512 while
/// Android's `RLIMIT_NOFILE` is commonly 1024. «Two hundred files» and «two
/// hundred open descriptors» are one bad loop apart, and the difference is
/// invisible until a real batch runs on a real phone.
///
/// Measured on `/proc/self/fd`, which counts **both** ends plus the test
/// harness, because the sender and the receiver are two threads of this
/// process. That makes the number pessimistic — it can only over-count — which
/// is the safe direction for a ceiling.
#[cfg(target_os = "linux")]
#[test]
fn two_hundred_files_do_not_hold_two_hundred_descriptors() {
    fn open_descriptors() -> usize {
        fs::read_dir("/proc/self/fd")
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    ensure_identity();
    let scratch = Scratch::new("descriptors");
    let root = scratch.path("origen");
    let destination = scratch.path("destino");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&destination).unwrap();

    const FILES: usize = 200;
    let files: Vec<PathBuf> = (0..FILES)
        .map(|index| {
            let path = root.join(format!("f{index:03}.bin"));
            write_pattern(&path, 512);
            path
        })
        .collect();

    let baseline = open_descriptors();
    let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(baseline));
    let watching = std::sync::Arc::clone(&peak);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let halt = std::sync::Arc::clone(&stop);

    // A sampler rather than a hook: what is being measured is a property of the
    // running process, and asking the process is the only way to learn it that
    // cannot be satisfied by the code under test agreeing with itself.
    let sampler = thread::spawn(move || {
        while !halt.load(Ordering::Relaxed) {
            let now = open_descriptors();
            watching.fetch_max(now, Ordering::Relaxed);
            thread::sleep(std::time::Duration::from_millis(2));
        }
    });

    let moved = move_files(&root, &files, &destination);
    stop.store(true, Ordering::Relaxed);
    sampler.join().unwrap();

    assert_eq!(
        moved.materialised,
        Ok(FILES as u32),
        "the batch did not land"
    );

    let high = peak.load(Ordering::Relaxed);
    let extra = high.saturating_sub(baseline);
    eprintln!(
        "[measure] {FILES} files: {baseline} descriptors before, {high} at the \
         peak, {extra} extra"
    );

    // **Medido: 402 de mas antes de QYR-0391, 11 despues.** Dos descriptores
    // por archivo -- el que lee en el origen y la parte abierta en el destino
    // --, ninguno de los dos cerrado hasta el final de la transferencia.
    //
    // **Thirty-two, and the number is a ceiling with room, not a measurement.**
    // What is being refused is the shape `O(files)`: two hundred files holding
    // two hundred descriptors would blow through this by a factor of six, and
    // anything under a few dozen is the `O(1)` this is checking for. The margin
    // absorbs the sampler, the two sockets and whatever the harness holds.
    assert!(
        extra < 32,
        "{FILES} files held {extra} extra descriptors at once. That is the \
         shape of one-per-file, and ADR-0047 §3 caps a transfer at 256 files \
         precisely because descriptors are a hard per-process limit -- 512 on \
         the Windows CRT, commonly 1024 on Android"
    );
}
