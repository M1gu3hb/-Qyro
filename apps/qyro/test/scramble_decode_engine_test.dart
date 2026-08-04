import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/boot/scramble_decode_engine.dart';

void main() {
  group('ScrambleDecodeEngine', () {
    test('the same seed produces the same frame', () {
      final first = ScrambleDecodeEngine(target: 'QYRO', seed: 42);
      final second = ScrambleDecodeEngine(target: 'QYRO', seed: 42);

      expect(first.frameAt(0.5), second.frameAt(0.5));
    });

    test('zero progress contains no revealed target cells', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 7);

      expect(engine.frameAt(0), isNot('QYRO'));
      expect(engine.revealedCellCount(0), 0);
    });

    test('full progress reveals the target', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 7);

      expect(engine.frameAt(1), 'QYRO');
      expect(engine.revealedCellCount(1), 4);
    });

    test('progress is clamped to the supported range', () {
      final engine = ScrambleDecodeEngine(target: 'QYRO', seed: 7);

      expect(engine.frameAt(2), 'QYRO');
      expect(engine.revealedCellCount(-1), 0);
    });

    test('reduced motion always returns the final frame', () {
      final engine = ScrambleDecodeEngine(
        target: 'QYRO',
        seed: 7,
        reducedMotion: true,
      );

      expect(engine.frameAt(0), 'QYRO');
    });
  });
}
