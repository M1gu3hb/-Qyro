// The production service, measured — not a fake answering a literal.
//
// **Why this file exists.** `transfer_screens_test.dart` proves the four
// screens render every state, driven by a `FakeService` whose
// `ownPairingString()` returns whatever the test wrote next to the assertion.
// That is the right tool for rendering and the wrong one for truth: it passed
// for months while `NativeTransferService.ownPairingString()` returned `null`
// for every transfer the product ever attempted, because `_listeningAddress`
// was read and never written (QYR-0322, reopened as P0 in phase 12).
//
// Everything here runs against `NativeTransferService`, the class that ships.

@TestOn('vm')
library;

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_session_api.dart';
import 'package:qyro/ffi/qyro_identity_api.dart';
import 'package:qyro/transfer/native_transfer_service.dart';
import 'package:qyro/transfer/transfer_service.dart';

String? get _libraryPath {
  final value = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
  return (value == null || value.isEmpty) ? null : value;
}

void main() {
  final library = _libraryPath;
  final skip = library == null ? 'QYRO_FFI_LIBRARY_PATH is not set' : null;

  group('the pairing code this device shows', () {
    late Directory scratch;
    late NativeTransferService service;

    // One identity per test **process**, not per test.
    //
    // ADR-0040 §2 makes a process hold exactly one: a second `open` naming a
    // different path is `bad_argument`, because two identities in one process
    // is not a state the engine can be in. The first draft of this file opened
    // one per `setUp` against a fresh temporary directory and every test after
    // the first was refused — the engine was right and the test was wrong, in
    // the same way and for the same reason as in phase 11.
    late Directory identityHome;

    setUpAll(() {
      identityHome = Directory.systemTemp.createTempSync('qyro-native-id');
      // Sandbox, because this runs on runners with no platform wrapper
      // (ADR-0040 amendment 1). What matters here is that an identity exists,
      // not which wrapper protected it.
      QyroIdentityBindings.open(QyroSessionBindings.open(library!)).open(
        '${identityHome.path}${Platform.pathSeparator}identity.qyro',
        QyroProtection.sandbox,
      );
    });

    setUp(() {
      scratch = Directory.systemTemp.createTempSync('qyro-native-service');
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

    test('listen_candidates_are_composed_before_anything_is_bound', () async {
      // The whole point of the fixed port (ADR-0041 §3): the code exists
      // before a socket does. Nothing in this test binds anything.
      final candidates = await service.listenCandidates();

      // A CI runner always has at least one non-loopback IPv4 interface. If
      // this is ever empty the machine is the anomaly, and the assertion says
      // so rather than passing vacuously.
      expect(
        candidates,
        isNotEmpty,
        reason: 'no non-loopback IPv4 interface on this machine, so this test '
            'cannot tell a working enumeration from a broken one',
      );

      for (final candidate in candidates) {
        expect(
          candidate.pairingString,
          startsWith('QYRO1|'),
          reason: 'the code must be the format qyro_pairing_parse accepts',
        );
        expect(
          candidate.address,
          endsWith(':$qyroDefaultPort'),
          reason: 'the port must be the fixed one, or the code names a port '
              'nothing will be listening on',
        );
        expect(candidate.interfaceName, isNotEmpty);
        // Loopback in a code is the single most useless thing this could emit:
        // it works only against oneself and says nothing about why.
        expect(candidate.address, isNot(startsWith('127.')));
        // A zone-id is local to the node and does not travel (RFC 4007), so a
        // link-local address in a code is a datum that means something else on
        // the machine that types it.
        expect(candidate.address, isNot(contains('%')));
      }
    });

    test('the_code_this_device_shows_is_one_the_engine_can_parse', () async {
      // The falsifiability half of the previous test. A string that merely
      // *looks* like a pairing code proves nothing; this hands it to the same
      // parser the other device will use.
      final candidates = await service.listenCandidates();
      final parsed =
          await service.addressOfPairingString(candidates.first.pairingString);

      expect(
        parsed,
        candidates.first.address,
        reason: 'the engine parsed a different address out of our own code, '
            'so the two halves of the chain disagree',
      );

      // And the control: a string that is not one of ours is refused rather
      // than half-accepted.
      expect(
          await service.addressOfPairingString('not a pairing code'), isNull);
    });

    test('own_pairing_string_is_not_null_once_this_device_is_listening',
        () async {
      // The direct regression against the defect, with the name phase 12 asks
      // for. Before the fix this returned null unconditionally.
      //
      // `receive()` assigns the listening address **before** the blocking open,
      // so the code exists while somebody is still reading it aloud. The stream
      // is subscribed and then abandoned: `QyroSession.receive` does not return
      // until a peer connects, and no peer ever will here — which is exactly
      // the state this test is about.
      expect(
        await service.ownPairingString(),
        isNull,
        reason: 'nothing is listening yet, and a code naming no listener is a '
            'code that does not work (ADR-0035 §2)',
      );

      final destination = Directory('${scratch.path}/in')..createSync();
      final subscription = service
          .receive(
            bind: '0.0.0.0:$qyroDefaultPort',
            destination: destination.path,
            decide: (_) async => false,
          )
          .listen(null, onError: (Object _) {});

      try {
        // Pumping the event loop is enough, and that is the property: the
        // assignment happens **before** the blocking open, not after it.
        await Future<void>.delayed(const Duration(milliseconds: 300));

        final code = await service.ownPairingString();
        expect(
          code,
          isNotNull,
          reason: 'this is QYR-0322. The address half of the code was read '
              'from a field nothing assigned, so this was null for every '
              'transfer the product ever attempted',
        );
        expect(code, startsWith('QYRO1|'));
        expect(code, contains(':$qyroDefaultPort|'));
      } finally {
        // **Cancelling the subscription does not stop the work**, and that is
        // worth knowing rather than discovering: the session runs inside
        // `Isolate.run`, blocked in `accept`, and an isolate cannot be
        // cancelled from outside. The first draft of this test ended here with
        // a `cancel()` and hung the whole runner for ten minutes.
        //
        // So the listener is unblocked the only way it can be: something
        // connects. A bare socket is enough — the handshake then fails, the
        // isolate returns an error, and `onError` swallows it. That the port
        // accepts a connection at all is itself the second half of what this
        // test is about.
        try {
          final socket = await Socket.connect(
            InternetAddress.loopbackIPv4,
            qyroDefaultPort,
            timeout: const Duration(seconds: 5),
          );
          socket.destroy();
        } on SocketException {
          // Nothing was listening after all; the assertions above already
          // decided whether that matters.
        }
        await subscription.cancel();
      }
    });
  }, skip: skip);
}
