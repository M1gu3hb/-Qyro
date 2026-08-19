// The four cells nobody has ever executed.
//
// Since phase 13 the engine has **two consumers** — the Flutter GUI across the
// FFI, and the `qyro` binary calling `qyro_session` directly — and ADR-0042 §2
// accepted the consequence: *a capability is not done until both reach it.*
//
// **And the seam between them had never been crossed.** That is precisely the
// shape of the five dead capabilities this project has shipped: two halves that
// work and a middle nobody walked. `Session::finish` was found the day somebody
// first put a Dart receiver against a real sender; it was not found by reading.
//
// The peer here is **the real `qyro` binary**, never a smoke harness. ADR-0046
// and the phase document are explicit about why: the harness is what hid the
// identity defect for five phases running. A test whose other end is a fixture
// proves the fixture.
//
// # What this does not prove
//
// Two machines. Everything here is loopback on one host, so a NIC, a switch, a
// firewall and a cable are all outside it. That is phase 19.

@Tags(<String>['ffi'])
library;

import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_identity_api.dart';
import 'package:qyro/ffi/qyro_session_api.dart';
import 'package:qyro/transfer/native_transfer_service.dart';
import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/transfer/transfer_service.dart';

String? _env(String name) {
  final value = Platform.environment[name];
  return (value == null || value.isEmpty) ? null : value;
}

/// Paths built with the platform's own separator.
///
/// **Not cosmetic.** `NativeTransferService._commonRoot` splits on
/// `Platform.pathSeparator` and drops the last segment, so a path written with a
/// forward slash on Windows has `out/payload.bin` as its *last* segment: the
/// root comes out as the grandparent and the name that travels is
/// `out/payload.bin`. The file then lands one directory deeper than anybody
/// looked, which is how this test spent an afternoon reading «nothing was
/// materialised» when the bytes had arrived.
String _join(String directory, String name) =>
    '$directory${Platform.pathSeparator}$name';

String _under(Directory parent, String name) => _join(parent.path, name);

Uint8List _pattern(int length) {
  final bytes = Uint8List(length);
  for (var i = 0; i < length; i++) {
    bytes[i] = (i * 31 + 7) & 0xFF;
  }
  return bytes;
}

/// A private copy of the binary, so two CLI processes are two *devices*.
///
/// The identity lives beside the executable (ADR-0042: a program that writes
/// into `%APPDATA%` has installed itself). Two processes from one copy would
/// therefore share a fingerprint and the test would be a device sending to
/// itself — which passes and proves nothing.
File _privateCli(String binary, Directory home) {
  final copy = File('${home.path}${Platform.pathSeparator}qyro.exe');
  File(binary).copySync(copy.path);
  return copy;
}

/// Vacia la salida de un proceso hijo, y **no es higiene: es correccion**.
///
/// Un hijo que escribe a una tuberia que nadie lee **se bloquea** cuando el
/// bufer del sistema se llena — unos pocos KB. Deja de dar pasos, el otro lado
/// se queda esperando, y el sintoma es un reloj de sesion que vence.
///
/// **QYR-0365 fue exactamente eso**, y costo tres sesiones: con 200 archivos el
/// receptor del CLI escribia 23 KB, se bloqueaba, y el emisor moria a los 60 s.
/// Se diagnostico como un defecto del motor y el motor mueve esos 200 archivos
/// en **0,33 s**. El defecto estaba en el arnes que lo media.
///
/// Las celdas de esta matriz mandan un archivo, asi que hoy no les pasa. Se
/// vacia igual: la diferencia entre «no pasa» y «no puede pasar» es la que hace
/// que alguien pierda tres sesiones.
void _drainChild(Process child) {
  child.stdout.drain<void>();
  child.stderr.drain<void>();
}

void main() {
  final library = _env('QYRO_FFI_LIBRARY_PATH');
  final cli = _env('QYRO_CLI_PATH');
  final skip = library == null
      ? 'QYRO_FFI_LIBRARY_PATH is not set'
      : cli == null
          ? 'QYRO_CLI_PATH is not set'
          : null;

  const port = 49517;

  group('the four cells of the GUI/CLI matrix', () {
    late Directory scratch;
    late NativeTransferService service;
    late QyroIdentityBindings identity;

    setUpAll(() {
      final home = Directory.systemTemp.createTempSync('qyro-matrix-id');
      identity = QyroIdentityBindings.open(QyroSessionBindings.open(library!));
      identity.open(
        '${home.path}${Platform.pathSeparator}identity.qyro',
        QyroProtection.sandbox,
      );
    });

    setUp(() {
      scratch = Directory.systemTemp.createTempSync('qyro-matrix');
      service = NativeTransferService(
        bindings: QyroSessionBindings.open(library!),
      );
    });

    tearDown(() async {
      // **Every cell binds the same port**, because ADR-0041 fixed one: the
      // Windows firewall grants inbound once per program and port, and a port
      // that moved per run would ask for a dialog every time. The consequence
      // here is that two cells cannot overlap, and a listener that has just
      // been killed does not release the socket instantly.
      //
      // This file passes on its own without the wait and failed inside the full
      // suite with it missing, which is the shape of a test that is green until
      // somebody runs it next to another one.
      await Future<void>.delayed(const Duration(seconds: 1));
      try {
        scratch.deleteSync(recursive: true);
      } on FileSystemException {
        // A held handle on Windows is not a test failure.
      }
    });

    test('CLI sends, GUI receives -- the direction the phone case needs',
        () async {
      final destination = Directory(_under(scratch, 'in'))..createSync();
      final source = Directory(_under(scratch, 'out'))..createSync();
      final payload = _pattern(128 * 1024);
      File(_join(source.path, 'payload.bin')).writeAsBytesSync(payload);

      final home = Directory(_under(scratch, 'cli'))..createSync();
      final sender = _privateCli(cli!, home);

      final seen = <QyroTransferState>[];
      final failures = <Object>[];
      final session = service
          .receive(
            bind: '0.0.0.0:$port',
            destination: destination.path,
            // The person says yes. It is a decision, and the test is the person.
            decide: (_) async => true,
          )
          .listen(seen.add, onError: failures.add);

      try {
        String? code;
        for (var attempt = 0; attempt < 50 && code == null; attempt++) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          code = await service.ownPairingString();
        }
        expect(code, isNotNull, reason: 'the GUI published no pairing code');

        final run = await Process.run(
          sender.path,
          <String>['send', _join(source.path, 'payload.bin'), '--to', code!],
        );
        expect(
          run.exitCode,
          0,
          reason: 'the CLI refused to send: ${run.stdout}\n${run.stderr}',
        );

        for (var attempt = 0; attempt < 100; attempt++) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          if (seen.any((state) => state is QyroDelivered)) break;
        }
      } finally {
        await session.cancel();
      }

      expect(failures, isEmpty, reason: 'the GUI receiver errored: $failures');
      final landed = File(_join(destination.path, 'payload.bin'));
      expect(landed.existsSync(), isTrue, reason: 'nothing was materialised');
      // Byte for byte. A file of the right length is not the same file, and
      // this project has QYR-0359 written down about exactly that distinction.
      expect(landed.readAsBytesSync(), equals(payload));
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('CLI sends to CLI -- and they are two devices, not one', () async {
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();
      final payload = _pattern(64 * 1024);
      File(_join(source.path, 'payload.bin')).writeAsBytesSync(payload);

      // Two copies, because the identity lives beside the executable: one copy
      // run twice would be a device sending to itself.
      final senderHome = Directory(_under(scratch, 'a'))..createSync();
      final receiverHome = Directory(_under(scratch, 'b'))..createSync();
      final sender = _privateCli(cli!, senderHome);
      final receiver = _privateCli(cli, receiverHome);

      final senderWho = await Process.run(sender.path, <String>['whoami']);
      final receiverWho = await Process.run(receiver.path, <String>['whoami']);
      final senderPrint = _fingerprintOf(senderWho.stdout.toString());
      final receiverPrint = _fingerprintOf(receiverWho.stdout.toString());
      expect(senderPrint, isNotEmpty);
      expect(
        senderPrint,
        isNot(equals(receiverPrint)),
        reason: 'both copies produced the same identity, so this would be one '
            'device talking to itself and would prove nothing',
      );

      // `--expect` and not a prompt: ADR-0042 §4 says a decision made *before*
      // the run, which is exactly what a test needs and is the production path.
      final listening = await Process.start(
        receiver.path,
        <String>['recv', '--out', destination.path, '--expect', senderPrint],
      );
      _drainChild(listening);
      try {
        await Future<void>.delayed(const Duration(seconds: 1));
        final run = await Process.run(
          sender.path,
          <String>[
            'send',
            _join(source.path, 'payload.bin'),
            '--to',
            '127.0.0.1:$port',
            '--expect',
            receiverPrint,
          ],
        );
        expect(
          run.exitCode,
          0,
          reason: 'the CLI refused to send: ${run.stdout}\n${run.stderr}',
        );
        await listening.exitCode.timeout(
          const Duration(seconds: 30),
          onTimeout: () => -1,
        );
      } finally {
        listening.kill();
      }

      final landed = File(_join(destination.path, 'payload.bin'));
      expect(landed.existsSync(), isTrue, reason: 'nothing was materialised');
      expect(landed.readAsBytesSync(), equals(payload));
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('GUI sends, CLI receives -- the scene of R7 §2, entire', () async {
      // **The cell the whole product is named after**: the phone sends and the
      // old PC receives in a terminal. Until phase 21 nobody had run it.
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();
      final payload = _pattern(96 * 1024);
      final original = File(_join(source.path, 'payload.bin'))
        ..writeAsBytesSync(payload);

      final home = Directory(_under(scratch, 'cli'))..createSync();
      final receiver = _privateCli(cli!, home);
      final who = await Process.run(receiver.path, <String>['whoami']);
      final receiverPrint = _fingerprintOf(who.stdout.toString());
      expect(receiverPrint, isNotEmpty);

      // **Not `ownPairingString()`.** That returns null unless this side is
      // listening, because it is built from a bound address (QYR-0322) -- and
      // here the GUI is the sender, so it has no address to publish. The
      // fingerprint is an identity fact, not a session fact, and it comes from
      // the identity surface.
      final minePrint = identity.fingerprint().replaceAll('-', '');
      expect(minePrint, isNotEmpty);

      final listening = await Process.start(
        receiver.path,
        <String>['recv', '--out', destination.path, '--expect', minePrint],
      );
      _drainChild(listening);
      try {
        await Future<void>.delayed(const Duration(seconds: 1));

        final seen = <QyroTransferState>[];
        await for (final state in service.send(
          address: '127.0.0.1:$port',
          files: <QyroPicked>[
            QyroPickedPath(
              path: original.path,
              name: 'payload.bin',
              size: payload.length,
            ),
          ],
          expectedFingerprint: receiverPrint,
        )) {
          seen.add(state);
          if (state is QyroDelivered || state is QyroFailed) break;
        }

        expect(
          seen.last,
          isA<QyroDelivered>(),
          reason: 'the GUI did not deliver: ${seen.last}',
        );
        await listening.exitCode
            .timeout(const Duration(seconds: 30), onTimeout: () => -1);
      } finally {
        listening.kill();
      }

      final landed = File(_join(destination.path, 'payload.bin'));
      expect(landed.existsSync(), isTrue, reason: 'nothing was materialised');
      expect(landed.readAsBytesSync(), equals(payload));
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('GUI sends to GUI -- two sessions, one engine, a real socket',
        () async {
      // Same process, two independent sessions over loopback. Not a shortcut:
      // both ends are the production class, and the bytes cross a socket.
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();
      final payload = _pattern(48 * 1024);
      final original = File(_join(source.path, 'payload.bin'))
        ..writeAsBytesSync(payload);

      final receiver = NativeTransferService(
        bindings: QyroSessionBindings.open(library!),
      );
      final seen = <QyroTransferState>[];
      final failures = <Object>[];
      final session = receiver
          .receive(
            bind: '0.0.0.0:$port',
            destination: destination.path,
            decide: (_) async => true,
          )
          .listen(seen.add, onError: failures.add);

      try {
        await Future<void>.delayed(const Duration(seconds: 1));
        final sent = <QyroTransferState>[];
        await for (final state in service.send(
          address: '127.0.0.1:$port',
          files: <QyroPicked>[
            QyroPickedPath(
              path: original.path,
              name: 'payload.bin',
              size: payload.length,
            ),
          ],
        )) {
          sent.add(state);
          if (state is QyroDelivered || state is QyroFailed) break;
        }
        expect(sent.last, isA<QyroDelivered>(), reason: '${sent.last}');

        for (var attempt = 0; attempt < 100; attempt++) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          if (seen.any((state) => state is QyroDelivered)) break;
        }
      } finally {
        await session.cancel();
      }

      expect(failures, isEmpty, reason: 'the receiver errored: $failures');
      final landed = File(_join(destination.path, 'payload.bin'));
      expect(landed.existsSync(), isTrue, reason: 'nothing was materialised');
      expect(landed.readAsBytesSync(), equals(payload));
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('doscientos archivos, y QYR-0365 no era del motor', () async {
      // **La celda que cierra QYR-0365, y lo cierra desmintiendola.**
      //
      // La ficha decia: cada archivo pequeño cuesta ~1,2 s, a los 60 s se corta
      // la sesion, y el bucle del emisor lo serializa. Tres sesiones lo
      // persiguieron dentro del motor.
      //
      // El motor mueve estos mismos 200 archivos en **0,33 s** —medido en
      // `qyro_session/tests/qyr_0365_measurement.rs`, 202 pasos y **cero**
      // lecturas vencidas— y esta celda los mueve en menos de un segundo.
      //
      // Lo que fallaba era el arnes: un hijo que escribe a una tuberia que nadie
      // lee **se bloquea** cuando el bufer del sistema se llena. Con 200
      // archivos el receptor escribe ~23 KB, se bloquea, deja de dar pasos, y el
      // emisor muere en el reloj de 60 s. `_drainChild` es la linea que lo
      // arregla, y esta prueba **fallaba antes de ella**: 60 295 ms y ni un
      // archivo entregado.
      const files = 200;
      const bytesEach = 64;

      final source = Directory(_under(scratch, 'many-out'))..createSync();
      final destination = Directory(_under(scratch, 'many-in'))..createSync();

      final picked = <QyroPicked>[];
      for (var i = 0; i < files; i++) {
        final name = 'f${i.toString().padLeft(3, '0')}.bin';
        final file = File(_join(source.path, name))
          ..writeAsBytesSync(
              Uint8List(bytesEach)..fillRange(0, bytesEach, 0x71));
        picked.add(QyroPickedPath(
          path: file.path,
          name: name,
          size: bytesEach,
        ));
      }

      final home = Directory(_under(scratch, 'many-cli'))..createSync();
      final receiver = _privateCli(cli!, home);
      final who = await Process.run(receiver.path, <String>['whoami']);
      final receiverPrint = _fingerprintOf(who.stdout.toString());
      final minePrint = identity.fingerprint().replaceAll('-', '');

      final listening = await Process.start(
        receiver.path,
        <String>['recv', '--out', destination.path, '--expect', minePrint],
      );
      _drainChild(listening);

      final started = DateTime.now();
      try {
        await Future<void>.delayed(const Duration(seconds: 1));
        final afterHandshake = DateTime.now();

        final seen = <QyroTransferState>[];
        await for (final state in service.send(
          address: '127.0.0.1:$port',
          files: picked,
          expectedFingerprint: receiverPrint,
        )) {
          seen.add(state);
          if (state is QyroDelivered || state is QyroFailed) break;
        }

        final moving = DateTime.now().difference(afterHandshake);
        // ignore: avoid_print
        print('\n--- QYR-0365: $files archivos de $bytesEach bytes ---');
        // ignore: avoid_print
        print('  tiempo:      ${moving.inMilliseconds} ms');
        // ignore: avoid_print
        print('  por archivo: '
            '${(moving.inMicroseconds / files / 1000).toStringAsFixed(1)} ms');
        // ignore: avoid_print
        print('  (el motor solo, en Rust: 2 ms por archivo)');

        expect(
          seen.last,
          isA<QyroDelivered>(),
          reason: 'con $files archivos la GUI no entrego: ${seen.last}',
        );

        // **Lo unico que se afirma sobre el tiempo**, porque es lo unico que no
        // depende de esta maquina: que no llego al reloj de sesion. Un umbral
        // fino aqui fallaria en un runner cargado y diria «regresion» donde solo
        // hubo un dia lento.
        expect(
          moving.inSeconds,
          lessThan(30),
          reason: 'tardo ${moving.inSeconds} s, que se acerca al reloj de 60 s '
              'que QYR-0365 describia. **No subas el reloj**: escondería esto',
        );
      } finally {
        listening.kill();
      }

      // Y llegaron los doscientos, no «casi».
      final landed = destination.listSync().whereType<File>().length;
      expect(landed, files, reason: 'llegaron $landed de $files');
      // ignore: avoid_print
      print('  entregados:  $landed/$files en '
          '${DateTime.now().difference(started).inMilliseconds} ms totales');
    });

    test('a path written with forward slashes lands where it was named',
        () async {
      // **The control for QYR-0363.** Windows accepts a forward slash in every
      // path API, so a path can arrive spelled that way -- from an argument, a
      // drag-and-drop, or a script. `_commonRoot` split on the platform
      // separator alone, so the last segment came out as `out<slash>payload.bin`
      // entire, the root as the grandparent, and the name that travelled
      // carried a directory nobody had named. The receiver then wrote the file
      // **one level deeper than anybody looked**, and reported success.
      //
      // This writes the path the broken way on purpose.
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();
      final payload = _pattern(16 * 1024);
      final original = File(_join(source.path, 'payload.bin'))
        ..writeAsBytesSync(payload);
      final slashed = original.path.replaceAll(Platform.pathSeparator, '/');
      expect(slashed, isNot(equals(original.path)),
          reason: 'this platform already uses forward slashes, so this control '
              'would pass without testing anything');

      final receiver = NativeTransferService(
        bindings: QyroSessionBindings.open(library!),
      );
      final session = receiver
          .receive(
            bind: '0.0.0.0:$port',
            destination: destination.path,
            decide: (_) async => true,
          )
          .listen((_) {}, onError: (_) {});

      try {
        await Future<void>.delayed(const Duration(seconds: 1));
        final sent = <QyroTransferState>[];
        await for (final state in service.send(
          address: '127.0.0.1:$port',
          files: <QyroPicked>[
            QyroPickedPath(
              path: slashed,
              name: 'payload.bin',
              size: payload.length,
            ),
          ],
        )) {
          sent.add(state);
          if (state is QyroDelivered || state is QyroFailed) break;
        }
        expect(sent.last, isA<QyroDelivered>(), reason: '${sent.last}');
        await Future<void>.delayed(const Duration(seconds: 1));
      } finally {
        await session.cancel();
      }

      expect(
        File(_join(destination.path, 'payload.bin')).existsSync(),
        isTrue,
        reason: 'the file did not land where it was named',
      );
      expect(
        Directory(_under(destination, 'out')).existsSync(),
        isFalse,
        reason: 'the file landed a directory deeper than anybody asked for, '
            'which is the defect this control exists for',
      );
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('a folder keeps its shape, empty subfolders included',
        () async {
      // **Escenario 1 de la fase 22.** Todo lo que este proyecto había probado
      // era un archivo suelto. Una carpeta con subcarpetas es lo primero que
      // hace cualquiera, y la estructura se preserva o no se preserva — no hay
      // término medio, y nadie lo había mirado.
      //
      // El árbol se compara **entrada por entrada**: sobrar un archivo falla
      // igual que faltar. Un destino que contiene lo que se mandó *y algo más*
      // no es un destino correcto.
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();

      final deep = Directory(_join(source.path, 'a'))..createSync();
      final deeper = Directory(_join(deep.path, 'b'))..createSync();
      // Una carpeta vacía: ADR-0047 §4 dice que **no viaja**, porque el
      // manifiesto lista archivos y una carpeta vacía no es un archivo. Se
      // afirma aquí para que la limitación esté escrita en una prueba y no sólo
      // en un documento.
      Directory(_join(source.path, 'vacia')).createSync();

      final top = File(_join(source.path, 'arriba.bin'))
        ..writeAsBytesSync(_pattern(4096));
      final nested = File(_join(deeper.path, 'hondo.bin'))
        ..writeAsBytesSync(_pattern(8192));

      final receiver = NativeTransferService(
        bindings: QyroSessionBindings.open(library!),
      );
      final failures = <Object>[];
      final session = receiver
          .receive(
            bind: '0.0.0.0:$port',
            destination: destination.path,
            decide: (_) async => true,
          )
          .listen((_) {}, onError: failures.add);

      try {
        await Future<void>.delayed(const Duration(seconds: 1));
        final sent = <QyroTransferState>[];
        await for (final state in service.send(
          address: '127.0.0.1:$port',
          files: <QyroPicked>[
            QyroPickedPath(
              path: top.path,
              name: 'arriba.bin',
              size: top.lengthSync(),
            ),
            QyroPickedPath(
              path: nested.path,
              name: 'hondo.bin',
              size: nested.lengthSync(),
            ),
            // **La carpeta vacía, mandada como entrada** (ADR-0050 enmienda 1).
            // Antes no estaba en esta lista, así que el motor nunca se enteraba
            // de ella: no es que la descartara, es que nadie se la decía.
            QyroPickedPath(
              path: _join(source.path, 'vacia'),
              name: 'vacia',
              size: 0,
            ),
          ],
        )) {
          sent.add(state);
          if (state is QyroDelivered || state is QyroFailed) break;
        }
        expect(sent.last, isA<QyroDelivered>(), reason: '${sent.last}');
        await Future<void>.delayed(const Duration(seconds: 1));
      } finally {
        await session.cancel();
      }

      expect(failures, isEmpty, reason: 'el receptor dio error: $failures');

      // Entrada por entrada, y ordenado para que la comparación no dependa del
      // orden en que el sistema de archivos devuelva las cosas.
      final landed = destination
          .listSync(recursive: true)
          .whereType<File>()
          .map((file) => file.path.substring(destination.path.length + 1))
          .toList()
        ..sort();

      expect(
        landed,
        equals(<String>[
          'a${Platform.pathSeparator}b${Platform.pathSeparator}hondo.bin',
          'arriba.bin',
        ]),
        reason: 'el árbol de destino no es el de origen',
      );
      expect(
        File(_join(destination.path, 'arriba.bin')).readAsBytesSync(),
        equals(_pattern(4096)),
      );
      expect(
        Directory(_join(destination.path, 'vacia')).existsSync(),
        isTrue,
        reason: 'una carpeta vacía NO viajó. **La decisión cambió** en ADR-0050 '
            'enmienda 1: viajan, con el ItemKind::Directory que el manifiesto '
            'ya tenía especificado, validado, probado y sin que nadie lo '
            'emitiera nunca. El documento se cambió primero, que es lo que el '
            'mensaje anterior de esta misma prueba exigía',
      );
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('with nobody listening, the CLI fails by name and does not hang',
        () async {
      // Control 1 of the phase document. A refusal that hangs is the failure
      // nobody can diagnose, and one that exits 0 is worse.
      final source = Directory(_under(scratch, 'out'))..createSync();
      File(_join(source.path, 'payload.bin')).writeAsBytesSync(_pattern(1024));
      final home = Directory(_under(scratch, 'cli'))..createSync();
      final sender = _privateCli(cli!, home);

      final run = await Process.run(
        sender.path,
        <String>[
          'send',
          _join(source.path, 'payload.bin'),
          '--to',
          // A port nothing is bound to.
          '127.0.0.1:49519',
        ],
      ).timeout(const Duration(seconds: 60));

      expect(run.exitCode, isNot(0), reason: 'sending into the void succeeded');
      final said = '${run.stdout}${run.stderr}';
      expect(
        said.toLowerCase(),
        contains('connect'),
        reason: 'the failure did not name what went wrong: $said',
      );
    }, timeout: const Timeout(Duration(minutes: 2)));

    test('a fingerprint that does not match is refused, by name, before bytes',
        () async {
      // Control 2. This is the product's security guarantee and it cannot hold
      // on one face only.
      final source = Directory(_under(scratch, 'out'))..createSync();
      final destination = Directory(_under(scratch, 'in'))..createSync();
      File(_join(source.path, 'payload.bin')).writeAsBytesSync(_pattern(4096));
      final home = Directory(_under(scratch, 'cli'))..createSync();
      final sender = _privateCli(cli!, home);

      final seen = <QyroTransferState>[];
      final session = service
          .receive(
            bind: '0.0.0.0:$port',
            destination: destination.path,
            decide: (_) async => true,
          )
          .listen(seen.add, onError: (_) {});

      try {
        String? code;
        for (var attempt = 0; attempt < 50 && code == null; attempt++) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          code = await service.ownPairingString();
        }

        final run = await Process.run(
          sender.path,
          <String>[
            'send',
            _join(source.path, 'payload.bin'),
            '--to',
            code!,
            // Thirty-two hex characters that are not this peer's.
            '--expect',
            'ffffffffffffffffffffffffffffffff',
          ],
        );

        expect(run.exitCode, isNot(0),
            reason: 'a wrong fingerprint was accepted');
        expect(
          '${run.stdout}${run.stderr}'.toUpperCase(),
          contains('REFUSED'),
          reason: 'the refusal did not say it was a refusal',
        );
        expect(
          File(_join(destination.path, 'payload.bin')).existsSync(),
          isFalse,
          reason: 'bytes landed despite the refusal',
        );
      } finally {
        await session.cancel();
      }
    }, timeout: const Timeout(Duration(minutes: 2)));
  }, skip: skip);
}

/// The compact fingerprint out of `qyro whoami`'s pairing line.
String _fingerprintOf(String output) {
  for (final line in output.split('\n')) {
    final trimmed = line.trim();
    if (trimmed.startsWith('QYRO1|')) {
      final parts = trimmed.split('|');
      if (parts.length >= 3) return parts[2].trim();
    }
  }
  return '';
}
