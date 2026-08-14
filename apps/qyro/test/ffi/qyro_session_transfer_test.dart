// The test that defines phase 02: Dart moves a real file between two OS
// processes, over a socket, and compares it byte for byte.
//
// The receiver is `qyro_net_smoke serve`, which already exists and speaks the
// same qyro_transfer engine and the same qyro_net handshake as qyro_session.
// It prints `LISTENING <port>` and flushes *before* it accepts, so the test is
// synchronised on the thing that matters instead of sleeping and hoping.
//
// Both binaries are located through environment variables, the same mechanism
// the Windows job already uses for the library:
//
//   QYRO_FFI_LIBRARY_PATH   the qyro_ffi dynamic library
//   QYRO_NET_SMOKE_PATH     the qyro_net_smoke executable
//
// Without them the group is skipped with a reason, because a test that silently
// passes when it could not run is worse than one that says it did not.

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_session_api.dart';

/// Two chunk windows and a bit more than the phase asks for.
///
/// The engine moves 64 KiB chunks behind a window of 16. The phase requires at
/// least 8 MiB so the window, the go-back-N and the flow control are actually
/// exercised; a file that fits in one window would pass a transfer that never
/// refills.
const _transferBytes = 8 * 1024 * 1024 + 13;

String? get _libraryPath {
  final value = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
  return (value == null || value.isEmpty) ? null : value;
}

String? get _smokePath {
  final value = Platform.environment['QYRO_NET_SMOKE_PATH'];
  return (value == null || value.isEmpty) ? null : value;
}

/// Writes `length` deterministic bytes without holding them all.
///
/// Deterministic so a failure says *which* byte differs; a random file makes a
/// failure unreproducible.
void _writePattern(File file, int length) {
  file.parent.createSync(recursive: true);
  final sink = file.openSync(mode: FileMode.write);
  final block = Uint8List(4096);
  var written = 0;
  while (written < length) {
    final take = min(block.length, length - written);
    for (var index = 0; index < take; index++) {
      block[index] = (written + index) % 251;
    }
    sink.writeFromSync(block, 0, take);
    written += take;
  }
  sink.closeSync();
}

/// A receiver process, plus the port it actually bound.
final class _Receiver {
  _Receiver(this.process, this.port, this.destination, this.finished);

  final Process process;
  final int port;
  final Directory destination;
  final Future<int> finished;
}

Future<_Receiver> _startReceiver(String smoke, Directory destination) async {
  final process = await Process.start(smoke, <String>[
    'serve',
    '0',
    destination.path,
  ]);

  // One subscription, not two. `stdout` is single-subscription, so reading the
  // announcement with `await for` and then draining the rest is
  // `Bad state: Stream has already been listened to` -- which is exactly what
  // the first draft of this helper did.
  final tail = <String>[];
  final announced = Completer<int>();
  final drained = Completer<void>();
  process.stdout.transform(utf8.decoder).transform(const LineSplitter()).listen(
    (line) {
      tail.add(line);
      if (!announced.isCompleted && line.startsWith('LISTENING ')) {
        announced.complete(
          int.parse(line.substring('LISTENING '.length).trim()),
        );
      }
    },
    onDone: () {
      if (!drained.isCompleted) {
        drained.complete();
      }
      if (!announced.isCompleted) {
        announced.completeError(
          StateError('the receiver exited without announcing a port: $tail'),
        );
      }
    },
  );
  process.stderr.drain<void>();

  final port = await announced.future.timeout(const Duration(seconds: 30));
  final finished = drained.future.then((_) => process.exitCode);
  return _Receiver(process, port, destination, finished);
}

void main() {
  final library = _libraryPath;
  final smoke = _smokePath;
  final reason = library == null
      ? 'QYRO_FFI_LIBRARY_PATH is not set'
      : smoke == null
          ? 'QYRO_NET_SMOKE_PATH is not set'
          : null;

  group('a file crosses two processes driven from Dart', () {
    late Directory scratch;
    late QyroSessionBindings bindings;

    setUp(() {
      scratch = Directory.systemTemp.createTempSync('qyro-dart-transfer');
      bindings = QyroSessionBindings.open(library!);
    });

    tearDown(() {
      try {
        scratch.deleteSync(recursive: true);
      } on FileSystemException {
        // A held handle on Windows is not a test failure.
      }
    });

    test('a_file_crosses_two_processes_driven_from_dart', () async {
      final source = Directory('${scratch.path}/send')..createSync();
      final destination = Directory('${scratch.path}/recv')..createSync();
      final original = File('${source.path}/payload.bin');
      _writePattern(original, _transferBytes);

      final receiver = await _startReceiver(smoke!, destination);

      final emissions = <QyroProgress>[];
      final session = QyroSession.send(
        bindings: bindings,
        to: '127.0.0.1:${receiver.port}',
        root: source.path,
        files: <String>[original.path],
        onProgress: emissions.add,
      );

      late final QyroSessionState state;
      try {
        state = await session.run();
      } finally {
        session.dispose();
      }
      await receiver.finished.timeout(const Duration(seconds: 60));

      expect(state, QyroSessionState.completed);

      // The transfer happened at all -- trap 4 of the phase document. A test
      // that never checks there was a transfer can be measuring the empty set.
      expect(emissions, isNotEmpty, reason: 'no progress ever reached Dart');
      final last = emissions.last;
      expect(last.total, _transferBytes);
      expect(last.done, last.total);
      expect(last.total, greaterThan(0));

      // Monotone.
      for (var index = 1; index < emissions.length; index++) {
        expect(
          emissions[index].done,
          greaterThanOrEqualTo(emissions[index - 1].done),
          reason: 'progress went backwards at emission $index',
        );
      }

      // And the budget of ADR-0033 §4 held across the real boundary.
      expect(
        emissions.length,
        lessThanOrEqualTo(102),
        reason: '${emissions.length} emissions for $_transferBytes bytes',
      );

      final arrived = File('${destination.path}/payload.bin');
      expect(arrived.existsSync(), isTrue, reason: 'nothing was materialised');
      final expected = original.readAsBytesSync();
      final actual = arrived.readAsBytesSync();
      expect(actual.length, expected.length);
      expect(actual, orderedEquals(expected));

      // No partial survives its own success.
      expect(
        File('${destination.path}/payload.bin.qyro-part').existsSync(),
        isFalse,
      );
    }, timeout: const Timeout(Duration(minutes: 5)));

    test('a_corrupted_transfer_is_detected_by_this_test', () async {
      // R2 §1.7. The byte-for-byte comparison above is only evidence if it can
      // see a changed byte. Rather than corrupt the wire -- which the AEAD
      // would reject, proving something else -- this flips a bit in the arrival
      // and confirms the comparison the test above relies on notices.
      final source = Directory('${scratch.path}/send')..createSync();
      final destination = Directory('${scratch.path}/recv')..createSync();
      final original = File('${source.path}/payload.bin');
      _writePattern(original, 512 * 1024);

      final receiver = await _startReceiver(smoke!, destination);
      final session = QyroSession.send(
        bindings: bindings,
        to: '127.0.0.1:${receiver.port}',
        root: source.path,
        files: <String>[original.path],
      );
      try {
        expect(await session.run(), QyroSessionState.completed);
      } finally {
        session.dispose();
      }
      await receiver.finished.timeout(const Duration(seconds: 60));

      final arrived = File('${destination.path}/payload.bin');
      final tampered = Uint8List.fromList(arrived.readAsBytesSync());
      final midpoint = tampered.length ~/ 2;
      tampered[midpoint] = tampered[midpoint] ^ 0x01;

      final expected = original.readAsBytesSync();
      expect(
        tampered.length,
        expected.length,
        reason: 'flipping a bit must not change the length',
      );
      expect(
        tampered,
        isNot(orderedEquals(expected)),
        reason: 'a single flipped bit was invisible to the comparison the test '
            'above depends on',
      );
    }, timeout: const Timeout(Duration(minutes: 5)));

    test('a_session_without_an_observer_still_completes', () async {
      // ADR-0033 §2: no observer must never be a second code path.
      final source = Directory('${scratch.path}/send')..createSync();
      final destination = Directory('${scratch.path}/recv')..createSync();
      final original = File('${source.path}/payload.bin');
      _writePattern(original, 512 * 1024);

      final receiver = await _startReceiver(smoke!, destination);
      final session = QyroSession.send(
        bindings: bindings,
        to: '127.0.0.1:${receiver.port}',
        root: source.path,
        files: <String>[original.path],
      );
      try {
        expect(await session.run(), QyroSessionState.completed);
      } finally {
        session.dispose();
      }
      await receiver.finished.timeout(const Duration(seconds: 60));

      expect(
        File('${destination.path}/payload.bin').readAsBytesSync(),
        orderedEquals(original.readAsBytesSync()),
      );
    }, timeout: const Timeout(Duration(minutes: 5)));

    test('closing_from_dart_leaves_no_handle_and_no_thread', () async {
      // The table holds four sessions (ADR-0032 §4). Opening and disposing more
      // than four in a row can only work if dispose actually frees the slot, so
      // this is the leak check the C surface can express.
      final source = Directory('${scratch.path}/send')..createSync();
      final original = File('${source.path}/payload.bin');
      _writePattern(original, 64 * 1024);

      for (var round = 0; round < 6; round++) {
        final destination = Directory('${scratch.path}/recv$round')
          ..createSync();
        final receiver = await _startReceiver(smoke!, destination);
        final session = QyroSession.send(
          bindings: bindings,
          to: '127.0.0.1:${receiver.port}',
          root: source.path,
          files: <String>[original.path],
        );
        try {
          expect(
            await session.run(),
            QyroSessionState.completed,
            reason:
                'round $round failed; a table that never frees a slot refuses '
                'the fifth session with QYRO_ERR_TABLE_FULL',
          );
        } finally {
          session.dispose();
        }
        await receiver.finished.timeout(const Duration(seconds: 60));
      }

      // And a disposed session refuses further work by name rather than by
      // crashing on a stale handle.
      final destination = Directory('${scratch.path}/recv-last')..createSync();
      final receiver = await _startReceiver(smoke!, destination);
      final session = QyroSession.send(
        bindings: bindings,
        to: '127.0.0.1:${receiver.port}',
        root: source.path,
        files: <String>[original.path],
      );
      await session.run();
      session.dispose();
      session.dispose(); // idempotent
      expect(session.progress, throwsA(isA<QyroSessionFailure>()));
      await receiver.finished.timeout(const Duration(seconds: 60));
    }, timeout: const Timeout(Duration(minutes: 10)));
  }, skip: reason);
}
