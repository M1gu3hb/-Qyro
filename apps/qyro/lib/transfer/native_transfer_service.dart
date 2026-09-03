// The transfer service, against the real engine.
//
// `QyroSession.stepBlocking` blocks without a bound (ADR-0032 §7), so the whole
// session — open, step to its ending, close — runs inside `Isolate.run`. Only
// three kinds of value cross back: progress triples, a terminal code, and text.
// Nothing that owns a pointer is ever sent, because a `Pointer` is an address in
// one isolate's view and means nothing in another's.

import 'dart:async';
import 'dart:ffi';
import 'dart:io';
import 'dart:isolate';

import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/ffi/qyro_identity_api.dart';
import 'package:qyro/ffi/qyro_session_api.dart';
import 'package:qyro/ffi/qyro_trust_api.dart';
import 'package:qyro/transfer/transfer_service.dart';

/// The worker's way of saying "the person here said no".
///
/// Distinct from every `QyroCode`, because an isolate returns one integer
/// and a refusal is not a transport failure. Negative and far from the
/// engine's own codes so a collision would be a compile-time-visible
/// accident rather than a silent remapping.
const int _receiveRefusedByMe = -1001;

/// The worker's way of saying "it finished and did not verify".
const int _receiveIntegrity = -1002;

/// La forma que tiene el trabajador de decir «contestó otro aparato».
///
/// **QYR-0392, y no lo produce el motor.** El apretón salió bien y la huella
/// autenticada no es la que prometía el código que alguien escaneó o tecleó. El
/// motor no puede decirlo porque no sabe qué código se leyó: la expectativa vive
/// donde se escaneó, no en la sesión.
///
/// Del mismo lado de la recta que los otros dos y por la misma razón: lejos de
/// los del motor, para que un choque fuera un accidente visible y no un
/// reetiquetado silencioso.
const int _sendNotTheExpectedDevice = -1003;

/// Where received files land.
///
/// ADR-0034 §4: the app's own directory on Android and `Downloads/Qyro` on
/// Windows, and **no storage permission on either**.
String defaultDestination() {
  if (Platform.isWindows) {
    final home = Platform.environment['USERPROFILE'] ?? '.';
    return '$home\\Downloads\\Qyro';
  }
  // **En Android esto NO es una respuesta usable, y el comentario que había aquí
  // decía lo contrario.** Decía «el lado Kotlin la pasa; hasta que lo haga, el
  // directorio de trabajo del proceso es la respuesta honesta». El directorio de
  // trabajo de un proceso de Android es **`/`**, así que la respuesta era
  // `/Qyro` — la raíz del sistema, que ninguna aplicación puede escribir. Y el
  // lado Kotlin nunca se escribió (QYR-0373).
  //
  // Quien llama en Android es `ReceiveScreen`, y desde QYR-0373 pregunta primero
  // a `androidDestination()`, que va por `dev.qyro/paths` y contesta
  // `/sdcard/Android/data/dev.qyro.app/files/Qyro` — escribible sin ningún
  // permiso, visible por USB, y borrada al desinstalar.
  //
  // Esta línea es lo que queda si ese canal no contesta, y no se ha cambiado por
  // algo «más seguro» a propósito: cualquier ruta que se pusiera aquí sería una
  // suposición sobre un aparato que este taller no ha tocado nunca, y una
  // suposición que falle al escribir el primer byte es peor que un fallo al
  // crear la carpeta, que al menos ocurre antes de que nada esté en el cable.
  return '${Directory.current.path}${Platform.pathSeparator}Qyro';
}

/// Where this device's identity blob lives.
///
/// ADR-0040 §4. Rust never guesses a directory: the caller names it, so there
/// is one code path on both platforms and a test can point at a temporary
/// directory — which is what makes the two-process check possible at all.
///
/// On Android the Kotlin side is what knows `getNoBackupFilesDir()`; until it
/// passes one in, the app's working directory is the honest answer rather than
/// a guessed path that would fail at write time. Same precedent as
/// [defaultDestination].
String defaultIdentityPath() {
  if (Platform.isWindows) {
    final local = Platform.environment['LOCALAPPDATA'] ??
        Platform.environment['USERPROFILE'] ??
        '.';
    return '$local${Platform.pathSeparator}Qyro'
        '${Platform.pathSeparator}identity.bin';
  }
  return '${Directory.current.path}${Platform.pathSeparator}identity.qyro';
}

/// The library path this process loads the engine from.
String? _libraryOverride() {
  final value = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
  return (value == null || value.isEmpty) ? null : value;
}

/// One progress sample, small enough to cross an isolate boundary.
final class _Sample {
  const _Sample(this.done, this.total);
  final int done;
  final int total;
}

final class NativeTransferService implements QyroTransferService {
  NativeTransferService({QyroSessionBindings? bindings})
      : _bindings = bindings ?? QyroSessionBindings.openDefault() {
    _trust = QyroTrustBindings.openDefault(_bindings);
    _identity = QyroIdentityBindings.open(_bindings);
  }

  final QyroSessionBindings _bindings;

  /// The loaded engine, for the one screen that needs the library itself.
  ///
  /// **QYR-0371.** `ScanScreen` takes a `DynamicLibrary` because the scanner
  /// resolves its own symbols, and nothing in production could hand it one: this
  /// class held the only loaded library and kept it private. The result was an
  /// optical channel complete on both sides with no way into it from the phone.
  ///
  /// Exposed rather than opened a second time. `DynamicLibrary.open` twice on
  /// the same path is two handles onto one image on Android, and the engine
  /// holds process-wide state — the identity `OnceLock` among it — so a second
  /// handle is a second chance to disagree about which one is current.
  DynamicLibrary get nativeLibrary => _bindings.library;
  late final QyroTrustBindings _trust;
  late final QyroIdentityBindings _identity;

  /// Opens this device's identity. **Must succeed before any transfer.**
  ///
  /// ADR-0040. Without it every session answers `identity_unreadable` rather
  /// than quietly generating a throwaway keypair, which is what it used to do
  /// and why the fingerprint on the peers screen changed between one transfer
  /// and the next.
  ///
  /// [QyroProtection.sandbox] off Windows because stage A has no Keystore
  /// bridge: the seed sits in the app's private directory with the per-UID
  /// sandbox as its only protection, and `THREAT_MODEL.md` says so in those
  /// words rather than in a footnote.
  void openIdentity({String? at}) {
    final path = at ?? defaultIdentityPath();
    _identity.open(
      path,
      Platform.isWindows ? QyroProtection.platform : QyroProtection.sandbox,
    );
  }

  /// This device's own fingerprint, or null before [openIdentity] succeeds.
  String? ownFingerprint() {
    try {
      final text = _identity.fingerprint();
      return text.isEmpty ? null : text;
    } on QyroSessionFailure {
      return null;
    }
  }

  /// What the peers screen shows. Names and fingerprints only.
  ///
  /// The engine's book is keyed by name and the fingerprint lives with the
  /// identity, so a name with no live session has no fingerprint to show yet;
  /// it is listed with an empty one rather than hidden, because a peer the
  /// device remembers and does not display is a peer nobody can forget.
  @override
  Future<List<QyroPeerEntry>> knownPeers() async => _trust
      .listPeers()
      .map(
        (name) => QyroPeerEntry(
          name: name,
          fingerprint: '',
          trust: QyroPeerTrust.known,
        ),
      )
      .toList(growable: false);

  @override
  Future<bool> forgetPeer(String name) async => _trust.forgetPeer(name);

  @override
  Future<String?> addressOfPairingString(String text) async =>
      _trust.addressOfPairingString(text);

  @override
  Future<String?> fingerprintOfPairingString(String text) async =>
      _trust.fingerprintOfPairingString(text);

  /// Si dos huellas son la misma, escritas como cada una se escriba.
  ///
  /// La del apretón lleva guiones y la de la cadena no (ADR-0035 §2), así que
  /// una comparación literal diría siempre que no — y decir siempre que no es
  /// tan inútil como decir siempre que sí. Es la misma normalización que hace
  /// la terminal en `flows.rs`, y lo es a propósito: dos caras que comparan
  /// distinto son dos productos.
  ///
  /// **Una expectativa vacía no coincide con nada.** Quien no tiene expectativa
  /// no llega aquí; quien llega con la cadena vacía tiene una y está rota.
  static bool fingerprintMatches(String actual, String wanted) {
    String normalise(String text) => text
        .toLowerCase()
        .split('')
        .where((c) => RegExp(r'[a-z0-9]').hasMatch(c))
        .join();
    final left = normalise(actual);
    final right = normalise(wanted);
    return right.isNotEmpty && left == right;
  }

  /// This device's pairing code, once it is receiving.
  ///
  /// **This returned `null` unconditionally until phase 11**, so the peers
  /// screen always showed "there is no code to show" and nobody could ever hand
  /// their code to anyone — the manual pairing path, the one that works on every
  /// network including one with client isolation, could not be used in either
  /// direction. The reason given was that a code needs an address *and* a
  /// fingerprint and both come from a live session; the fingerprint half was
  /// true only because the engine had no stable identity, and ADR-0040 fixed
  /// that.
  ///
  /// The address half still needs a listener, so this answers null until one
  /// exists and says which half is missing rather than showing a code that does
  /// not work (ADR-0035 §2).
  @override
  Future<String?> ownPairingString() async {
    final fingerprint = ownFingerprint();
    if (fingerprint == null) {
      return null;
    }
    final address = _listeningAddress;
    if (address == null) {
      return null;
    }
    return 'QYRO1|$address|${fingerprint.replaceAll('-', '')}';
  }

  /// Where this device is listening, while it is.
  ///
  /// **QYR-0322.** This field was read and never written -- two occurrences in
  /// the whole tree and neither an assignment -- so `ownPairingString()`
  /// answered null for every transfer the product ever attempted. It is
  /// assigned now, in [receive], **before** the blocking open, because the
  /// whole point is that the code exists while somebody is still typing it.
  String? _listeningAddress;

  /// Every address this device could be reached at.
  ///
  /// ADR-0041 §4. Loopback is excluded because a code naming it works only
  /// against oneself; IPv6 link-local is excluded because its zone-id is local
  /// to the node and does not travel (RFC 4007), so it would be a datum that
  /// means something else on the machine that types it. Virtual adapters --
  /// Hyper-V, VirtualBox, WSL, VPN -- are **not** excluded: filtering them
  /// needs a per-OS list of adapter names, exactly the kind of heuristic that
  /// ages badly and that this project has paid for twice. Every candidate is
  /// shown with its interface name and a person decides.
  @override
  Future<List<QyroListenAddress>> listenCandidates() async {
    final fingerprint = ownFingerprint();
    if (fingerprint == null) {
      return const <QyroListenAddress>[];
    }
    final compact = fingerprint.replaceAll('-', '');

    final interfaces = await NetworkInterface.list(
      includeLoopback: false,
      includeLinkLocal: false,
      type: InternetAddressType.IPv4,
    );

    final candidates = <QyroListenAddress>[];
    for (final interface in interfaces) {
      for (final address in interface.addresses) {
        // Belt and braces: `includeLoopback: false` is documented, and a code
        // naming 127.0.0.1 is the single most useless thing this could emit.
        if (address.isLoopback) continue;
        final where = '${address.address}:$qyroDefaultPort';
        candidates.add(
          QyroListenAddress(
            interfaceName: interface.name,
            address: where,
            pairingString: 'QYRO1|$where|$compact',
          ),
        );
      }
    }
    return List<QyroListenAddress>.unmodifiable(candidates);
  }

  @override
  Future<List<QyroPicked>> pickFiles() => pickerForPlatform().pickFiles();

  @override
  Stream<QyroTransferState> send({
    required String address,
    required List<QyroPicked> files,
    String? expectedFingerprint,
  }) async* {
    if (files.isEmpty) {
      yield const QyroFailed(kind: QyroFailureKind.cancelled);
      return;
    }
    yield const QyroConnecting();

    final paths = files.whereType<QyroPickedPath>().map((f) => f.path).toList();
    if (paths.length != files.length) {
      // A descriptor cannot cross an isolate: it is an integer in this
      // process's table and the isolate shares that table, but ownership does
      // not survive being sent. Android's path goes through the same session on
      // this isolate instead, which blocks the frame for the length of the
      // transfer and is the honest cost until phase 07 measures it.
      yield* _sendDescriptors(address, files, expectedFingerprint);
      return;
    }

    final root = _commonRoot(paths);
    final library = _libraryOverride();
    final port = ReceivePort();
    final samples = StreamController<_Sample>();
    port.listen((message) {
      if (message is List && message.length == 2) {
        samples.add(_Sample(message[0] as int, message[1] as int));
      }
    });

    // **`sendPort`, hoisted, and not `port`.** `Isolate.run` serialises whatever
    // the closure captures, and a `ReceivePort` is explicitly unsendable while a
    // `SendPort` is exactly what is meant to cross. Capturing `port` -- which is
    // what writing `port.sendPort` inside the closure does -- made every send
    // throw «object is unsendable» before a byte moved.
    //
    // **The GUI had therefore never sent a file**, since this was written. It
    // survived because the screens are tested against a fake service and the
    // two-process test exercises *receiving*; nothing ever ran this path against
    // a real peer. That is the same seam as QYR-0361 on the other face, found
    // the same way and on the same day (QYR-0362).
    final sendPort = port.sendPort;
    // Izada por la misma razón que `sendPort`: lo que el cierre captura se
    // serializa, y una cadena lo hace sin ruido. Leerla de `this` dentro del
    // cierre arrastraría el servicio entero al otro isolate.
    final expected = expectedFingerprint;
    final outcome = Isolate.run<int>(() {
      final bindings = library == null
          ? QyroSessionBindings.openDefault()
          : QyroSessionBindings.open(library);
      final session = QyroSession.send(
        bindings: bindings,
        to: address,
        root: root,
        files: paths,
        onProgress: (progress) =>
            sendPort.send(<int>[progress.done, progress.total]),
      );
      try {
        // **La expectativa se comprueba aquí, antes del primer paso**
        // (QYR-0392). El apretón ya terminó —`QyroSession.send` no vuelve
        // hasta que termina— así que la huella autenticada existe, y ningún
        // byte del archivo ha salido todavía.
        //
        // ADR-0035 §2.1: si no coincide **no se pregunta**. Quien escaneó ya
        // contestó esa pregunta, y volver a hacerla es como la gente aprende a
        // decir que sí.
        if (expected != null) {
          final peer =
              QyroTrustBindings.openDefault(bindings).peerFingerprint(session);
          if (!NativeTransferService.fingerprintMatches(peer, expected)) {
            return _sendNotTheExpectedDevice;
          }
        }
        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
        }
        return state == QyroSessionState.completed ? 0 : 1;
      } on QyroSessionFailure catch (failure) {
        return failure.code;
      } finally {
        session.dispose();
      }
    });

    yield* _drain(samples.stream, outcome, address);
    await samples.close();
    port.close();
  }

  /// Android's half: the descriptors belong to this isolate's table.
  Stream<QyroTransferState> _sendDescriptors(
    String address,
    List<QyroPicked> files,
    String? expectedFingerprint,
  ) async* {
    final descriptors = files.whereType<QyroPickedDescriptor>().toList();
    yield const QyroConnecting();
    try {
      final session = QyroSession.sendDescriptors(
        bindings: _bindings,
        to: address,
        descriptors: descriptors.map((f) => f.descriptor).toList(),
        names: descriptors.map((f) => f.name).toList(),
      );
      try {
        // La misma comprobación que el otro camino, y en el mismo momento: el
        // apretón terminó y no ha salido un byte (QYR-0392). **Éste es el
        // camino del teléfono**, que es donde se escanea.
        if (expectedFingerprint != null &&
            !fingerprintMatches(
              _trust.peerFingerprint(session),
              expectedFingerprint,
            )) {
          yield const QyroFailed(kind: QyroFailureKind.notTheExpectedDevice);
          return;
        }
        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
          final progress = session.progress();
          yield QyroMoving(
            done: progress.done,
            total: progress.total,
            fingerprint: _trust.peerFingerprint(session),
          );
          await Future<void>.delayed(Duration.zero);
        }
        if (state == QyroSessionState.completed) {
          yield QyroDelivered(
            fileCount: descriptors.length,
            destination: address,
          );
        } else {
          yield QyroFailed(
            kind: QyroFailureKind.refusedByPeer,
            reason: _trust.rejection(session),
          );
        }
      } finally {
        session.dispose();
      }
    } on QyroSessionFailure catch (failure) {
      yield QyroFailed(kind: _kindOf(failure.code), code: failure.code);
    }
  }

  Stream<QyroTransferState> _drain(
    Stream<_Sample> samples,
    Future<int> outcome,
    String address,
  ) async* {
    var last = const _Sample(0, 0);
    final subscription = samples.listen((sample) => last = sample);
    // **QYR-0384: esto era `await outcome` a secas, y por eso un envío que
    // fallaba al abrir no decía nada.**
    //
    // `QyroSession.send` se construye **fuera** del `try` del worker, así que si
    // lanza —una dirección que no parsea, un puerto que no se puede abrir, la
    // biblioteca que no carga— el futuro se completa con un error, y este
    // `await` dentro de un `async*` lo convierte en un **error de stream**. La
    // pantalla hace `await for` sin `catch`, así que no llega ningún estado:
    // pulsar Enviar no hacía nada visible, que es el peor final posible.
    //
    // `_drainReceive` ya lo capturaba. Esta mitad no.
    var code = QyroCode.unknown;
    String? detail;
    try {
      code = await outcome;
    } on QyroSessionFailure catch (failure) {
      code = failure.code;
    } on Object catch (error) {
      detail = '$error';
      // Un worker puede morir por cosas que no son un `QyroSessionFailure`, y
      // ninguna de ellas debe salir por el stream sin un estado: «no pasó nada»
      // no es un final que alguien pueda leer. `unknown` llega a la pantalla
      // como un fallo, que es lo que fue.
    }
    await subscription.cancel();

    if (code == 0) {
      yield QyroDelivered(fileCount: 1, destination: address);
      return;
    }
    yield QyroMoving(done: last.done, total: last.total, fingerprint: '');
    yield QyroFailed(kind: _kindOf(code), code: code, detail: detail);
  }

  @override
  Stream<QyroTransferState> receive({
    required String bind,
    required String destination,
    required Future<bool> Function(QyroAwaitingDecision offer) decide,
  }) async* {
    final where = destination.isEmpty ? defaultDestination() : destination;
    // **QYR-0384, y es el cinturón del arreglo de QYR-0373.** Esto lanzaba
    // dentro de un `async*` **antes del primer `yield`**, así que la pantalla no
    // recibía ni un estado: pulsar Recibir no hacía nada visible. Era el camino
    // exacto que tomaba `/Qyro` en Android, y seguirá siendo el camino de
    // cualquier carpeta que no se pueda crear -- un disco lleno, un permiso.
    try {
      Directory(where).createSync(recursive: true);
    } on FileSystemException {
      yield const QyroFailed(kind: QyroFailureKind.noRoom);
      return;
    }

    // QYR-0322, and this is the whole fix. `QyroSession.receive` binds and
    // accepts inside one call and does not return until a peer connects, so
    // anything recorded *after* it is recorded too late to be typed into
    // another machine. The port is known in advance (ADR-0041 §3), so the
    // address is known in advance too, and it is written down here -- before
    // the blocking open -- which is what makes `ownPairingString()` answer
    // while somebody is still reading the code aloud.
    final candidates = await listenCandidates();
    _listeningAddress =
        candidates.isEmpty ? _hostPortOf(bind) : candidates.first.address;

    yield const QyroConnecting();

    // **The session runs in a worker isolate, and it is not optional.**
    //
    // `QyroSession.receive` binds *and* accepts inside one call and does not
    // return until a peer connects (ADR-0032 §7: a `_blocking` symbol may not
    // run where frames are drawn). Until phase 12 this ran right here, on the
    // isolate that draws the interface, so tapping Recibir froze the whole
    // application -- no repaint, no navigation, no cancel -- until somebody
    // connected or the process was killed. The send path had used
    // `Isolate.run` since phase 02; this one never did, and the file header
    // claimed otherwise.
    //
    // What crosses, and nothing else: integers, text, and one boolean back.
    // No `Pointer` is ever sent, because an address in one isolate's view
    // means nothing in another's.
    final library = _libraryOverride();
    final events = ReceivePort();
    final sink = events.sendPort;
    final states = StreamController<QyroTransferState>();
    SendPort? answerTo;

    events.listen((message) async {
      if (message is! List || message.isEmpty) return;
      switch (message[0] as String) {
        case 'offer':
          answerTo = message[4] as SendPort;
          final offer = QyroAwaitingDecision(
            fingerprint: message[1] as String,
            trust: QyroPeerTrust.values[message[2] as int],
            // **Estaba escrito a mano como lista vacía**, porque no había
            // símbolo que trajera los nombres: `Session::offered_files()`
            // existía desde QYR-0364 y no cruzaba la frontera. La tarjeta que
            // los dibuja ya estaba hecha. ADR-0032 enmienda 6.
            fileNames: (message[5] as List<Object?>).cast<String>(),
            totalBytes: message[3] as int,
          );
          states.add(offer);
          // The decision is asked on **this** isolate, which is the one with a
          // person attached to it, and the answer is the only thing that goes
          // back. ADR-0036 §1: nothing is accepted on its own.
          answerTo?.send(await decide(offer));
        case 'moving':
          states.add(
            QyroMoving(
              done: message[1] as int,
              total: message[2] as int,
              fingerprint: message[3] as String,
            ),
          );
      }
    });

    final outcome = _receiveInIsolate(library, bind, where, sink);

    yield* _drainReceive(states.stream, outcome, where);
    await states.close();
    events.close();
  }

  /// Lanza el trabajador del receptor **desde un ámbito que no contiene nada
  /// más**.
  ///
  /// **QYR-0403, y es un defecto de producto, no de la prueba.** `Isolate.run`
  /// serialisa **todo el contexto léxico** del closure que recibe, no sólo lo
  /// que ese closure usa. El closure vivía dentro de `receive()`, y en ese
  /// mismo ámbito viven `decide` —el callback que trae quien llama— y
  /// `states`, un `StreamController`. Los dos viajaban en el grafo del mensaje,
  /// y el isolate los rechaza:
  ///
  /// ```text
  /// Illegal argument in isolate message: (object is a DynamicLibrary)
  /// ```
  ///
  /// El `DynamicLibrary` entra por `decide`: el callback lo escribe quien llama
  /// a `receive`, y su contexto es el de **quien llama** — una pantalla, una
  /// prueba— donde vive el propio servicio, que tiene la biblioteca dentro.
  /// **Recibir no podía arrancar**, y lo que salía era
  /// `QyroFailureKind.internal`: «algo interno falló».
  ///
  /// El arreglo es el mismo que `send` ya llevaba escrito para el `ReceivePort`
  /// tres pantallas más arriba, aplicado al ámbito entero: el closure se crea
  /// **aquí**, donde lo único que hay son estos cuatro valores, y los cuatro
  /// cruzan sin problema.
  static Future<int> _receiveInIsolate(
    String? library,
    String bind,
    String where,
    SendPort sink,
  ) async {
    return Isolate.run<int>(() async {
      final bindings = library == null
          ? QyroSessionBindings.openDefault()
          : QyroSessionBindings.open(library);
      final trust = QyroTrustBindings(bindings, bindings.library);
      final session = QyroSession.receive(
        bindings: bindings,
        bind: bind,
        destination: where,
      );
      // Si el camino feliz ya materializó, el `finally` no vuelve a hacerlo.
      var finished = false;
      try {
        // **ADR-0032 enmienda 6.** Esto daba UN `stepBlocking()` y preguntaba,
        // con un comentario que decía «un paso trae la oferta y el manifiesto».
        // Son **dos**, medidos: tras el primero `offered_files` está vacío y
        // `progress().total` es 0. Así que la tarjeta ofrecía «0 archivos, 0 B»
        // -- y 0 no es «no lo sé» para quien lo lee, es «nada».
        //
        // El número vive en `Session::await_offer`, en Rust, no aquí.
        trust.awaitOffer(session);
        final offered = trust.offeredFiles(session);
        // The real verdict, not a hardcoded `newPeer`. With ADR-0040 the
        // fingerprint on the other end is stable between transfers, so
        // `Changed` finally means what ADR-0031 says it means.
        final fingerprint = trust.peerFingerprint(session);
        // Keyed by the fingerprint, because the peers screen has no other name
        // for a device nobody has named yet, and a verdict under a name the
        // person never chose would be a verdict about the wrong thing. A book
        // that cannot answer is **not** a reason to claim the peer is known.
        QyroPeerTrust verdict;
        try {
          verdict = trust.peerTrust(session, fingerprint);
        } on QyroSessionFailure {
          // A book that cannot answer is not a reason to claim the peer is
          // known. `newPeer` makes the screen ask, which is the safe end of
          // the mistake.
          verdict = QyroPeerTrust.newPeer;
        }

        // The worker is **not** inside a blocking call at this point, which is
        // the only reason it can wait for an answer at all.
        final answer = ReceivePort();
        sink.send(<Object>[
          'offer',
          fingerprint,
          verdict.index,
          // El total del **manifiesto**, no el del progreso: es lo que el par
          // dice que va a mandar, que es la pregunta que se le hace a la
          // persona. El progreso mide lo que ya llegó, y todavía no llegó nada.
          offered.totalBytes,
          answer.sendPort,
          offered.names,
        ]);
        final accepted = await answer.first as bool;
        answer.close();

        if (!accepted) {
          trust.reject(session, QyroRejectReason.declined);
          return _receiveRefusedByMe;
        }

        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
          final now = session.progress();
          sink.send(<Object>['moving', now.done, now.total, fingerprint]);
        }
        if (state != QyroSessionState.completed) {
          return _receiveIntegrity;
        }
        // **QYR-0374: materialising is part of arriving, so its failure is part
        // of the answer.**
        //
        // Esto devolvía `0` —éxito— y dejaba `finish` para el `finally`, que se
        // tragaba el fallo con el argumento de que «el final ya dijo lo que
        // hubo». No lo había dicho: el final es `completed` porque **la
        // transferencia** terminó, y `finish` se niega por una razón del sistema
        // de archivos que el final no conoce.
        //
        // Medido mandando el mismo archivo dos veces a la misma carpeta: el
        // segundo llega al 100 % y no se guarda nada, porque Qyro no
        // sobrescribe nunca (ADR-0027 §4). Con la versión anterior, el teléfono
        // decía **«entregado»** y en el disco no había nada — que es exactamente
        // la forma de QYR-0357 que el comentario de abajo dice haber cerrado.
        //
        // `finished` evita llamarlo dos veces: el `finally` sigue haciéndolo en
        // todos los demás finales, que es para lo que está.
        finished = true;
        try {
          session.finish();
          return 0;
        } on QyroSessionFailure catch (failure) {
          return failure.code;
        }
      } on QyroSessionFailure catch (failure) {
        return failure.code;
      } finally {
        // **QYR-0357. Nothing arrives without this.**
        //
        // `finish` verifies each item's digest and renames its `.qyro-part` to
        // the final name (ADR-0027 §4). No symbol reached it until ADR-0032
        // amendment 3, so this receiver reported "delivered" and left a part
        // file on disk -- the worst shape a failure takes, because a person is
        // left believing they have the file.
        //
        // In `finally`, so it runs on **every** ending and not only the happy
        // one: a receiver that stopped early leaves a part per started item and
        // nothing else removes it (QYR-0087, QYR-0088). A refusal materialises
        // nothing and releases the parts, which is the same call.
        //
        // **Y sólo cuando el camino feliz no lo hizo ya** (QYR-0374). En un
        // final que no fue `completed`, tragarse el fallo aquí sí es correcto y
        // no esconde nada: la transferencia ya fracasó por otra razón, y no se
        // puede degradar más de lo que está. Lo que no era correcto era
        // tragárselo **también** cuando todo había ido bien hasta el último
        // paso, porque entonces el único fallo que hubo era éste.
        if (!finished) {
          try {
            session.finish();
          } on QyroSessionFailure {
            // Ver arriba: este final ya fracasó por su cuenta.
          }
        }
        session.dispose();
      }
    });
  }

  /// Interleaves the worker's states with its ending.
  Stream<QyroTransferState> _drainReceive(
    Stream<QyroTransferState> states,
    Future<int> outcome,
    String destination,
  ) async* {
    final buffered = StreamController<QyroTransferState>();
    final subscription = states.listen(buffered.add);
    try {
      final pending = <QyroTransferState>[];
      final reader = buffered.stream.listen(pending.add);
      var code = 0;
      String? detail;
      try {
        code = await outcome;
      } on QyroSessionFailure catch (failure) {
        code = failure.code;
      } on Object catch (error) {
        // QYR-0384, la misma razón que en `_drain`: un worker puede morir por
        // algo que no es un `QyroSessionFailure`, y salir por el stream sin un
        // estado deja la pantalla en blanco.
        //
        // **Y QYR-0403: la excepción se guarda.** Antes se tiraba aquí mismo, y
        // lo que llegaba al otro extremo era `internal` a secas: ni número ni
        // frase, o sea nada sobre lo que actuar.
        code = QyroCode.unknown;
        detail = '$error';
      }
      await reader.cancel();
      for (final state in pending) {
        yield state;
      }
      yield switch (code) {
        0 => QyroDelivered(fileCount: 1, destination: destination),
        _receiveRefusedByMe =>
          const QyroFailed(kind: QyroFailureKind.refusedByMe),
        _receiveIntegrity => const QyroFailed(kind: QyroFailureKind.integrity),
        _ => QyroFailed(kind: _kindOf(code), code: code, detail: detail),
      };
    } finally {
      await subscription.cancel();
      await buffered.close();
    }
  }

  /// The local history.
  ///
  /// `qyro_fs::history` records it and no C symbol reads it yet, so this is
  /// empty rather than wrong: an empty list is a true statement about what this
  /// build can show, and a fabricated one would not be.
  @override
  Future<List<QyroHistoryEntry>> history() async => const <QyroHistoryEntry>[];

  static QyroFailureKind _kindOf(int code) => switch (code) {
        QyroCode.peerUnreachable => QyroFailureKind.unreachable,
        QyroCode.notAuthenticated => QyroFailureKind.keyChanged,
        QyroCode.transferRefused => QyroFailureKind.refusedByPeer,
        QyroCode.storageRefused => QyroFailureKind.noRoom,
        QyroCode.cancelled => QyroFailureKind.cancelled,
        QyroCode.tooManyFiles => QyroFailureKind.tooManyFiles,
        QyroCode.portUnavailable => QyroFailureKind.portUnavailable,
        // **QYR-0386: aquí el comodín mandaba ocho códigos a «integridad».**
        //
        // `integrity` dibuja «llegó algo que no verificó», que es una acusación
        // concreta contra el otro extremo. Siete de esos ocho no tienen nada que
        // ver con él, y uno —`identityUnreadable`— es el estado de **este**
        // aparato cuando no puede abrir su propia identidad: la persona leía
        // «los datos llegaron mal» mientras no había llegado nada.
        QyroCode.identityUnreadable => QyroFailureKind.identityUnreadable,
        QyroCode.badArgument => QyroFailureKind.badAddress,
        // QYR-0392. Éste no viene del motor: lo pone esta capa cuando la huella
        // autenticada no es la que prometía el código escaneado.
        _sendNotTheExpectedDevice => QyroFailureKind.notTheExpectedDevice,
        // El resto son fallos internos y se dicen así. Menos informativo y
        // verdad, que es la única propiedad que un mensaje de error necesita.
        _ => QyroFailureKind.internal,
      };

  /// The deepest directory every path shares.
  ///
  /// The engine names each item relative to a root (ADR-0026), so two files from
  /// different folders must not both become their last component — that would
  /// make the receiver arbitrate a collision the sender created.
  /// Every separator spelled the way this platform spells it.
  ///
  /// Only rewrites `/` to `\` on Windows; elsewhere `\` is a legal character in
  /// a filename and rewriting it would corrupt names rather than fix paths.
  static String _withPlatformSeparators(String path) =>
      Platform.isWindows ? path.replaceAll('/', Platform.pathSeparator) : path;

  static String _commonRoot(List<String> paths) {
    if (paths.isEmpty) return '.';
    final separator = Platform.pathSeparator;
    // **Normalised before splitting, and it is not tidiness.** Windows accepts
    // forward slashes everywhere -- every Win32 path API does -- so `C:/b.bin`
    // is a perfectly valid path that this function used to split on `\` alone.
    // The last segment came out as `a/b.bin`, the root as the *grandparent*, and
    // the name that travelled was `a/b.bin`: the receiver then wrote the file
    // one directory deeper than anybody had named, silently. A file that lands
    // somewhere else is not a smaller failure than a file that does not land.
    final normalised =
        paths.map((path) => _withPlatformSeparators(path)).toList();
    var prefix = normalised.first.split(separator)..removeLast();
    for (final path in normalised.skip(1)) {
      final parts = path.split(separator)..removeLast();
      var shared = 0;
      while (shared < prefix.length &&
          shared < parts.length &&
          prefix[shared] == parts[shared]) {
        shared++;
      }
      prefix = prefix.sublist(0, shared);
    }
    if (prefix.isEmpty) return separator;

    // **Un archivo en la raíz de una unidad** (QYR-0390). `D:\video.mp4` deja
    // `prefix` en `['D:']`, y unir eso da **`D:`** — que en Windows no es la
    // raíz de la unidad, es «el directorio actual de la unidad D», una cosa
    // distinta y con historia propia. El motor hace `strip_prefix(root)` sobre
    // cada ruta, y `D:\video.mp4` no empieza por `D:` en componentes: uno lleva
    // raíz y el otro no. Así que mandar cualquier cosa que esté en la raíz de
    // una unidad salía como `BadArgument`.
    //
    // El separador lo convierte en la raíz de verdad. Y la comprobación es
    // estrecha a propósito -- una letra y dos puntos -- porque un directorio
    // que se llame `datos:` no existe en Windows y no puede confundirse con esto.
    final root = prefix.join(separator);
    if (prefix.length == 1 && RegExp(r'^[A-Za-z]:$').hasMatch(prefix.first)) {
      return '$root$separator';
    }
    return root;
  }
}

/// The `host:port` a bind string names, when nothing better is available.
///
/// Used only when interface enumeration comes back empty -- a real state on a
/// machine still waiting for APIPA (R8 §8). A wildcard host is rewritten to
/// loopback rather than left as `0.0.0.0`, because a code that says `0.0.0.0`
/// is a code that cannot be typed anywhere, and loopback at least says plainly
/// that this device is only reachable from itself right now.
String _hostPortOf(String bind) {
  if (bind.startsWith('0.0.0.0:')) {
    return '127.0.0.1:${bind.substring('0.0.0.0:'.length)}';
  }
  return bind;
}
