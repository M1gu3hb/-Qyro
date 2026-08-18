import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/boot/scramble_decode_engine.dart';

void main() {
  group('ScrambleDecodeEngine', () {
    test('same seed and frame index are deterministic', () {
      final first = ScrambleDecodeEngine(target: 'QYRO', seed: 42);
      final second = ScrambleDecodeEngine(target: 'QYRO', seed: 42);

      expect(
        first.frameAt(0.5, frameIndex: 7),
        second.frameAt(0.5, frameIndex: 7),
      );
    });

    test('noise changes between frames without changing progress', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 42);

      expect(
        engine.frameAt(0, frameIndex: 1),
        isNot(engine.frameAt(0, frameIndex: 2)),
      );
    });

    test('progress is clamped and full progress reveals target', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 7);

      expect(engine.revealedCellCount(-1), 0);
      expect(engine.frameAt(2, frameIndex: 0), 'QYRO');
      expect(engine.revealedCellCount(2), 4);
    });

    test('reduced motion and skip return the final frame', () {
      final reduced = ScrambleDecodeEngine(
        target: 'QYRO',
        seed: 7,
        reducedMotion: true,
      );
      final skipped = ScrambleDecodeEngine(target: 'QYRO', seed: 7)..skip();

      expect(reduced.frameAt(0, frameIndex: 0), 'QYRO');
      expect(skipped.frameAt(0, frameIndex: 0), 'QYRO');
    });

    test('cancellation freezes the last rendered frame', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 7);
      final before = engine.frameAt(0.25, frameIndex: 3);

      engine.cancel();

      expect(engine.frameAt(0.9, frameIndex: 20), before);
      expect(engine.isCancelled, isTrue);
    });

    test('column reveal completes earlier columns first', () {
      final engine = ScrambleDecodeEngine(
        target: 'ABCD',
        seed: 3,
        width: 2,
        revealMode: RevealMode.columns,
      );

      expect(engine.isCellRevealed(0, 0.25), isTrue);
      expect(engine.isCellRevealed(1, 0.25), isFalse);
      expect(engine.isCellRevealed(2, 0.25), isTrue);
    });

    test('radial reveal begins at the center', () {
      final engine = ScrambleDecodeEngine(
        target: '123456789',
        seed: 3,
        width: 3,
        revealMode: RevealMode.radial,
      );

      expect(engine.isCellRevealed(4, 0), isTrue);
      expect(engine.isCellRevealed(0, 0), isFalse);
      expect(engine.isCellRevealed(0, 1), isTrue);
    });

    test('mask keeps absent cells blank', () {
      final engine = ScrambleDecodeEngine(
        target: 'AB',
        seed: 3,
        width: 2,
        revealMode: RevealMode.mask,
        mask: const <bool>[true, false],
      );

      expect(engine.frameAt(0, frameIndex: 1).runes.last, 0x20);
      expect(engine.frameAt(1, frameIndex: 1), 'A ');
    });

    test('luminance reveals bright cells before dark cells', () {
      final engine = ScrambleDecodeEngine(
        target: 'AB',
        seed: 3,
        width: 2,
        revealMode: RevealMode.luminance,
        luminance: const <double>[1, 0],
      );

      expect(engine.isCellRevealed(0, 0), isTrue);
      expect(engine.isCellRevealed(1, 0), isFalse);
    });

    test('empty, one-cell, and Unicode targets are supported', () {
      final empty = ScrambleDecodeEngine(target: '', seed: 1);
      final small = ScrambleDecodeEngine(target: 'X', seed: 1, width: 1);
      final unicode = ScrambleDecodeEngine(target: 'QÝ🔒', seed: 1);

      expect(empty.frameAt(0, frameIndex: 0), '');
      expect(small.frameAt(1, frameIndex: 0), 'X');
      expect(unicode.frameAt(1, frameIndex: 0), 'QÝ🔒');
      expect(unicode.cellCount, 3);
    });

    test('alpha, color, and normalized progress callbacks are retained', () {
      final progressEvents = <double>[];
      final engine = ScrambleDecodeEngine(
        target: 'AB',
        seed: 1,
        targetAlphas: const <double>[0.4, 0.8],
        targetColors: const <int>[0xFF168BFF, 0xFF51C8FF],
        onProgress: progressEvents.add,
      );

      engine.frameAt(1.5, frameIndex: 0);

      expect(engine.targetAlphaAt(0), 0.4);
      expect(engine.targetColorAt(1), 0xFF51C8FF);
      expect(progressEvents, <double>[1]);
    });
  });
}
