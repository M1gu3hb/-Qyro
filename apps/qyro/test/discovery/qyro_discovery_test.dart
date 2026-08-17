// The Dart side of `dev.qyro/discovery`, exercised without a device.
//
// What these tests can and cannot prove is worth being explicit about, because
// this repository has a rule against inventing hardware evidence: a fake
// `MethodChannel` proves the **Dart half** — that the channel is opened, that
// the arguments are the ones Kotlin parses, that a malformed reply cannot reach
// the interface. It proves nothing about `NsdManager`. That is phase 19, on a
// real network, and the blank stays blank until then.

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/discovery/qyro_discovery.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel('dev.qyro/discovery');
  final calls = <MethodCall>[];
  Object? reply;
  PlatformException? failure;

  setUp(() {
    calls.clear();
    reply = null;
    failure = null;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (failure != null) {
        throw failure!;
      }
      return reply;
    });
  });

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  group('the channel Kotlin has been waiting four phases for', () {
    test('advertise sends the two arguments DiscoveryChannel.kt reads',
        () async {
      // The Kotlin side does `call.argument<Int>("port")` and
      // `call.argument<String>("fingerprint")` and refuses anything else with
      // `bad_argument`. These names are the contract, and nothing but this test
      // holds the two languages to it.
      await const QyroAndroidDiscovery().advertise(
        port: 49517,
        fingerprint: 'ab12cd34ab12cd34ab12cd34ab12cd34',
      );

      expect(calls, hasLength(1));
      expect(calls.single.method, 'advertise');
      final arguments = calls.single.arguments as Map<Object?, Object?>;
      expect(arguments['port'], 49517);
      expect(arguments['fingerprint'], 'ab12cd34ab12cd34ab12cd34ab12cd34');
    });

    test('browse turns the platform maps into peers with a pairing code',
        () async {
      reply = <Map<Object?, Object?>>[
        <Object?, Object?>{
          'address': '192.168.1.9:49517',
          'fingerprint': 'ab12cd34ab12cd34ab12cd34ab12cd34',
        },
      ];

      final peers = await const QyroAndroidDiscovery().browse();

      expect(peers, hasLength(1));
      expect(peers.single.address, '192.168.1.9:49517');
      // The code is assembled here from the announced fields, in the format
      // ADR-0035 fixed and `qyro_session` parses.
      expect(
        peers.single.pairingCode,
        'QYRO1|192.168.1.9:49517|ab12cd34ab12cd34ab12cd34ab12cd34',
      );
    });

    test('a malformed entry is dropped, not shown with a blank identity',
        () async {
      // The channel is a boundary, and a boundary that trusts its input is not
      // a boundary. Kotlin already refuses services without a valid
      // fingerprint; this is the second wall.
      reply = <Map<Object?, Object?>>[
        <Object?, Object?>{'address': 42, 'fingerprint': 'ab12'},
        <Object?, Object?>{'address': '10.0.0.2:49517'},
        <Object?, Object?>{
          'address': '10.0.0.3:49517',
          'fingerprint': 'ff00ff00ff00ff00ff00ff00ff00ff00',
        },
      ];

      final peers = await const QyroAndroidDiscovery().browse();

      expect(peers, hasLength(1), reason: 'a broken entry reached the screen');
      expect(peers.single.address, '10.0.0.3:49517');
    });

    test('the same device on two interfaces is one device', () async {
      // Deduplicated by fingerprint, never by address (ADR-0043 §5). A phone on
      // Wi-Fi and a cable announces twice, and listing it twice reads as
      // "someone is impersonating them".
      reply = <Map<Object?, Object?>>[
        <Object?, Object?>{
          'address': '192.168.1.9:49517',
          'fingerprint': 'ab12cd34ab12cd34ab12cd34ab12cd34',
        },
        <Object?, Object?>{
          'address': '169.254.3.7:49517',
          'fingerprint': 'ab12cd34ab12cd34ab12cd34ab12cd34',
        },
      ];

      final peers = await const QyroAndroidDiscovery().browse();

      expect(peers, hasLength(1));
      // The first address announced is the one kept, deterministically.
      expect(peers.single.address, '192.168.1.9:49517');
    });

    test('and two different devices are not collapsed into one', () async {
      // The control for the test above. A dedup that returned one peer for
      // everything would satisfy it perfectly and destroy the feature.
      reply = <Map<Object?, Object?>>[
        <Object?, Object?>{
          'address': '192.168.1.9:49517',
          'fingerprint': 'ab12cd34ab12cd34ab12cd34ab12cd34',
        },
        <Object?, Object?>{
          'address': '192.168.1.10:49517',
          'fingerprint': 'ff00ff00ff00ff00ff00ff00ff00ff00',
        },
      ];

      final peers = await const QyroAndroidDiscovery().browse();

      expect(peers, hasLength(2), reason: 'two devices were shown as one');
    });

    test('a device with no NsdManager says so instead of looking quiet',
        () async {
      // The distinction the file picker taught this codebase: an empty list
      // from a platform that cannot ask is indistinguishable from an empty list
      // from a quiet network, and a person shown the second when it was the
      // first concludes the other device is off.
      failure = PlatformException(
        code: 'unavailable',
        message: 'this device has no NsdManager',
      );

      expect(
        const QyroAndroidDiscovery().browse(),
        throwsA(isA<QyroDiscoveryUnavailable>()),
      );
    });

    test('stop is allowed to be pointless', () async {
      // Stopping something never started, or on a device with no NsdManager, is
      // the caller asking for the browse to be over. It is over.
      failure =
          PlatformException(code: 'unavailable', message: 'no NsdManager');
      await const QyroAndroidDiscovery().stop();
      expect(calls.single.method, 'stop');
    });
  });

  group('which platform gets discovery', () {
    test('android opens the channel', () {
      expect(
        discoveryForPlatform(operatingSystem: 'android'),
        isA<QyroAndroidDiscovery>(),
      );
    });

    test('windows refuses by name rather than answering empty', () async {
      final discovery = discoveryForPlatform(operatingSystem: 'windows');
      expect(discovery, isA<QyroNoDiscovery>());
      // Not `[]`. The desktop GUI has no discovery symbol at the C frontier,
      // and saying "I cannot ask" is a different sentence from "nobody is
      // there".
      await expectLater(
        discovery.browse(),
        throwsA(isA<QyroDiscoveryUnavailable>()),
      );
    });

    test('and an unknown platform is refused too, not defaulted', () async {
      final discovery = discoveryForPlatform(operatingSystem: 'fuchsia');
      await expectLater(
        discovery.browse(),
        throwsA(isA<QyroDiscoveryUnavailable>()),
      );
    });
  });
}
