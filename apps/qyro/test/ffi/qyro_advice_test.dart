// The channel advisor, reached from the face that did not have it.
//
// **This is the seam phase 21 exists to exercise.** `qyro_session::advise` had a
// caller in the CLI and none in the GUI, which is exactly the shape of the five
// dead capabilities this project has shipped: two halves that work and a middle
// nobody crossed.
//
// The assertions here are deliberately about **agreement**, not about wording.
// A test that pinned the exact sentence would break on every improvement to the
// text and teach somebody to edit the test instead of reading it. What must not
// drift is that both faces get the *same* decision from the *same* module.

@Tags(<String>['ffi'])
library;

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_identity_api.dart';
import 'package:qyro/ffi/qyro_session_api.dart';

String? _env(String name) {
  final value = Platform.environment[name];
  return (value == null || value.isEmpty) ? null : value;
}

void main() {
  final library = _env('QYRO_FFI_LIBRARY_PATH');
  final skip = library == null ? 'QYRO_FFI_LIBRARY_PATH is not set' : null;

  group('the channel advisor, across the C frontier', () {
    late QyroIdentityBindings bindings;

    setUpAll(() {
      bindings = QyroIdentityBindings.open(QyroSessionBindings.open(library!));
    });

    test('a shared network is offered first and takes no time worth naming',
        () {
      final advice = bindings.advice(
        hasNetwork: true,
        peerDiscovered: true,
        hasSerialPort: false,
        otherHasCamera: false,
        payloadLength: 1024 * 1024,
      );

      expect(advice, contains('network'));
      expect(advice, contains('qyro find'));
      // The boring question belongs in front of a *slow* channel. Asking it when
      // the network works is noise, and noise in front of the fast path is how
      // somebody ends up on the slow one.
      expect(advice, isNot(contains('CD or DVD burner')));
    });

    test('a slow channel is preceded by the boring question', () {
      final advice = bindings.advice(
        hasNetwork: false,
        peerDiscovered: false,
        hasSerialPort: true,
        otherHasCamera: false,
        payloadLength: 1024 * 1024,
      );

      // `FASE-16` §2, now enforced on the face that never had it: a CD-R moves
      // 700 MB in five minutes, and offering seventeen minutes of anything
      // without saying so is bad product.
      expect(advice, contains('CD or DVD burner'));
      expect(advice, contains('serial'));
    });

    test('serial is offered before optical, because it is ten times faster',
        () {
      final advice = bindings.advice(
        hasNetwork: false,
        peerDiscovered: false,
        hasSerialPort: true,
        otherHasCamera: true,
        payloadLength: 1024 * 1024,
      );

      final serialAt = advice.indexOf('serial cable');
      final opticalAt = advice.indexOf('QR codes');
      expect(serialAt, greaterThanOrEqualTo(0));
      expect(opticalAt, greaterThanOrEqualTo(0));
      expect(
        serialAt,
        lessThan(opticalAt),
        reason: 'ADR-0046 §4 puts serial first: both are slow, and one is an '
            'order of magnitude faster and does not ask anybody to hold a '
            'phone steady for minutes',
      );
    });

    test('with nothing available it says what to plug in, not nothing', () {
      final advice = bindings.advice(
        hasNetwork: false,
        peerDiscovered: false,
        hasSerialPort: false,
        otherHasCamera: false,
        payloadLength: 4096,
      );

      // An interface handed an empty string draws a blank panel, and a blank
      // panel is where somebody decides the product is broken.
      expect(advice, isNotEmpty);
      expect(advice, contains('no way to reach'));
      expect(advice, contains('serial'));
      expect(advice, contains('camera'));
    });

    test('a bigger file is never quoted as faster', () {
      // Not a pinned string: a relationship. Whatever the wording becomes, an
      // estimate that shrinks as the file grows is a bug somebody would ship.
      final small = bindings.advice(
        hasNetwork: false,
        peerDiscovered: false,
        hasSerialPort: true,
        otherHasCamera: false,
        payloadLength: 4096,
      );
      final large = bindings.advice(
        hasNetwork: false,
        peerDiscovered: false,
        hasSerialPort: true,
        otherHasCamera: false,
        payloadLength: 64 * 1024 * 1024,
      );

      // 4 KB over 9 KB/s is under a second, and the engine says exactly that
      // -- which is why this asserts the *absence* of the slow words rather
      // than the presence of 'seconds'. The first draft looked for 'seconds'
      // and failed against "less than a second", which was the advisor being
      // right and the test being lazy.
      expect(small, contains('less than a second'));
      expect(small, isNot(contains('minutes')));
      expect(
        large,
        anyOf(contains('minutes'), contains('hours')),
        reason: '64 MB over a serial cable is not measured in seconds',
      );
    });
  }, skip: skip);
}
