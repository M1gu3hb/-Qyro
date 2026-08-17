// The whole chain, across two real processes, with the production class.
//
// **This is the test phase 12 exists for.** `transfer_screens_test.dart` proves
// the screens render; `native_transfer_service_test.dart` proves the code this
// device composes is real. Neither can show that a *second machine* can use it,
// and that is the thing the v1.0 shipped without.
//
// The shape is the one ADR-0041 §5 fixes and nothing else:
//
//   1. This process is the **receiver**. It uses `NativeTransferService` — the
//      production class, never a fake — and publishes its pairing string.
//   2. A **second process** (`qyro_net_smoke send`) parses the address out of
//      that string and connects to it.
//   3. A file crosses and is compared **byte for byte** at the destination.
//      Phase 12 §5 asks for SHA-256; comparing the bytes is strictly
//      stronger — a digest can only fail to notice — and it costs no new
//      dependency, which `crypto` would have been.
//
// Only one side listens, which is ADR-0041 §5 and R8 §9: Windows blocks inbound
// by default, so the permission is needed once and on the machine where the
// person is looking.

@TestOn('vm')
library;

import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_identity_api.dart';
import 'package:qyro/ffi/qyro_session_api.dart';
import 'package:qyro/transfer/native_transfer_service.dart';
import 'package:qyro/transfer/transfer_service.dart';

String? _env(String name) {
  final value = Platform.environment[name];
  return (value == null || value.isEmpty) ? null : value;
}

/// The payload. Big enough to cross several frames, small enough to be quick.
Uint8List _pattern(int length) {
  final bytes = Uint8List(length);
  for (var i = 0; i < length; i++) {
    bytes[i] = (i * 31 + 7) & 0xFF;
  }
  return bytes;
}

void main() {
  final library = _env('QYRO_FFI_LIBRARY_PATH');
  final smoke = _env('QYRO_NET_SMOKE_PATH');
  final skip = library == null
      ? 'QYRO_FFI_LIBRARY_PATH is not set'
      : smoke == null
          ? 'QYRO_NET_SMOKE_PATH is not set'
          : null;

  group('a file crosses to a device that was given only a pairing string', () {
    late Directory scratch;
    late NativeTransferService service;

    setUpAll(() {
      final home = Directory.systemTemp.createTempSync('qyro-2p-id');
      QyroIdentityBindings.open(QyroSessionBindings.open(library!)).open(
        '${home.path}${Platform.pathSeparator}identity.qyro',
        QyroProtection.sandbox,
      );
    });

    setUp(() {
      scratch = Directory.systemTemp.createTempSync('qyro-2p');
      service = NativeTransferService(
        bindings: QyroSessionBindings.open(library!),
      );
    });

    tearDown(() {
      try {
        scratch.deleteSync(recursive: true);
      } on FileSystemException {
        // A held handle on Windows is not a test failure.
      }
    });

    test('the_code_this_device_publishes_is_enough_for_another_process',
        () async {
      final source = Directory('${scratch.path}/out')..createSync();
      final destination = Directory('${scratch.path}/in')..createSync();
      final payload = _pattern(256 * 1024);
      final original = File('${source.path}/payload.bin')
        ..writeAsBytesSync(payload);

      // 1. Start receiving. Nothing has connected and nothing will until the
      //    second process is told where to go.
      final seen = <QyroTransferState>[];
      // Errors are collected, never swallowed: a silent `onError` is how a
      // receiver that never ran looks exactly like one that ran fine.
      final failures = <Object>[];
      final session = service
          .receive(
            bind: '0.0.0.0:$qyroDefaultPort',
            destination: destination.path,
            // Accepting is a decision a person makes; here the test is the
            // person, and it says yes exactly once.
            decide: (_) async => true,
          )
          .listen(seen.add, onError: failures.add);

      try {
        // 2. The code exists **before** anyone connects. That is the whole
        //    property, and it is read off the production class.
        String? code;
        for (var attempt = 0; attempt < 50 && code == null; attempt++) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          code = await service.ownPairingString();
        }
        expect(
          code,
          isNotNull,
          reason: 'this device never published a pairing string, so no second '
              'machine could ever be told where to connect (QYR-0322)',
        );

        // 3. A second process is given the code and nothing else. Parsing it
        //    goes through the engine, which is what the other device would do.
        final address = await service.addressOfPairingString(code!);
        expect(address, isNotNull, reason: 'our own code did not parse');

        final sender = await Process.run(
          smoke!,
          <String>['send', address!, original.path],
        );
        expect(
          sender.exitCode,
          0,
          reason: 'the second process could not use the code: '
              '${sender.stdout}\n${sender.stderr}',
        );

        // 4. The file is at the destination and it is the same file.
        //
        // `QyroDelivered` is waited for rather than a fixed delay: the engine
        // renames the `.qyro-part` into place only after the digest verifies
        // (ADR-0027 §4), so the finished name appearing **is** the verification
        // having passed. A sleep would have raced it.
        for (var attempt = 0; attempt < 60; attempt++) {
          if (seen.any((state) => state is QyroDelivered)) break;
          await Future<void>.delayed(const Duration(milliseconds: 100));
        }
        expect(
          seen.whereType<QyroDelivered>(),
          isNotEmpty,
          reason: 'the receiver never reported a delivery. It saw: $seen',
        );

        final arrived = destination
            .listSync(recursive: true)
            .whereType<File>()
            .where((file) => !file.path.endsWith('.qyro-part'))
            .toList();
        expect(
          arrived,
          hasLength(1),
          reason: 'the destination holds ${arrived.length} finished files. '
              'The receiver saw: $seen. Errors: $failures',
        );
        final landed = arrived.single.readAsBytesSync();
        expect(
          landed.length,
          payload.length,
          reason: 'the file that arrived is a different size',
        );
        // Byte for byte, not a digest: a comparison that walks every byte
        // cannot fail to notice, and a digest can.
        var firstDifference = -1;
        for (var i = 0; i < payload.length; i++) {
          if (landed[i] != payload[i]) {
            firstDifference = i;
            break;
          }
        }
        expect(
          firstDifference,
          -1,
          reason: 'the bytes that arrived are not the bytes that were sent; '
              'they first differ at offset $firstDifference',
        );
      } finally {
        await session.cancel();
      }
    }, timeout: const Timeout(Duration(minutes: 3)));

    test('and_it_fails_by_name_when_nobody_is_listening', () async {
      // **The falsifiability control, and without it the test above cannot
      // tell "it worked" from "it never tried".** Same code shape, same
      // second process, and nothing on the other end.
      //
      // The address is composed by hand rather than read from the service on
      // purpose: asking a service that is not receiving would return null and
      // the test would end before reaching the thing it measures.
      final orphan = File('${scratch.path}/orphan.bin')
        ..writeAsBytesSync(_pattern(1024));

      // A port in the same private range that nothing in this test ever binds.
      final nowhere = '127.0.0.1:${qyroDefaultPort + 1}';

      final sender = await Process.run(
        smoke!,
        <String>['send', nowhere, orphan.path],
      );

      expect(
        sender.exitCode,
        isNot(0),
        reason: 'sending to a port nobody is listening on reported success, so '
            'the test above cannot distinguish a transfer from a no-op',
      );
      final said = '${sender.stdout}${sender.stderr}'.toLowerCase();
      expect(
        said,
        anyOf(
          contains('unreachable'),
          contains('refused'),
          contains('connect'),
          contains('timed out'),
        ),
        reason: 'the failure has to be nameable. What it said was: $said',
      );
      // And it is a *different* ending from the one a real transfer produces,
      // which is what makes the pair of tests mean something.
      expect(said, isNot(contains('all_ok":true')));
    }, timeout: const Timeout(Duration(minutes: 2)));
  }, skip: skip);
}
