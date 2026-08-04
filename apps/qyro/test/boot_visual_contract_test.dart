import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/boot/ascii_logo_model.dart';
import 'package:qyro/boot/boot_sequence_controller.dart';

void main() {
  group('AsciiLogoModel', () {
    test('parses and flattens the generated asset contract', () {
      final model = AsciiLogoModel.fromJsonString('''
{
  "width": 2,
  "height": 2,
  "aspectRatio": 1.0,
  "characterCells": ["A ", "BC"],
  "mask": ["10", "01"],
  "density": [[1.0, 0.0], [0.25, 0.75]],
  "sourceChecksum": "sha256:abc",
  "generatorVersion": "1.0.0",
  "provisional": true
}
''');

      expect(model.width, 2);
      expect(model.height, 2);
      expect(model.target, 'A BC');
      expect(model.mask, <bool>[true, false, false, true]);
      expect(model.density, <double>[1, 0, 0.25, 0.75]);
      expect(model.provisional, isTrue);
    });

    test('rejects malformed dimensions instead of painting partial data', () {
      expect(
        () => AsciiLogoModel.fromJsonString('''
{
  "width": 3,
  "height": 1,
  "aspectRatio": 1.0,
  "characterCells": ["AB"],
  "mask": ["11"],
  "density": [[1.0, 1.0]],
  "sourceChecksum": "sha256:abc",
  "generatorVersion": "1.0.0",
  "provisional": true
}
'''),
        throwsFormatException,
      );
    });
  });

  group('BootSequenceController', () {
    test('defines the 5.5 second sequence and one second skip guard', () {
      expect(
        BootSequenceController.sequenceDuration,
        const Duration(milliseconds: 5500),
      );
      expect(
        BootSequenceController.skipGuard,
        const Duration(seconds: 1),
      );
      expect(BootSequenceController.canSkipAt(const Duration(milliseconds: 999)), isFalse);
      expect(BootSequenceController.canSkipAt(const Duration(seconds: 1)), isTrue);
    });

    test('maps phases continuously and clamps out-of-range values', () {
      expect(BootSequenceController.phaseProgress(-1, 0, 0.2), 0);
      expect(BootSequenceController.phaseProgress(0.1, 0, 0.2), closeTo(0.5, 0.001));
      expect(BootSequenceController.phaseProgress(0.4, 0, 0.2), 1);
    });

    test('reduced motion completes the visual sequence immediately', () {
      final controller = BootSequenceController(reducedMotion: true);

      expect(controller.visualProgress, 1);
      expect(controller.isVisualComplete, isTrue);
      expect(controller.canSkip, isTrue);
    });

    test('skip completes visuals but never claims startup is ready', () {
      final controller = BootSequenceController();

      controller.updateElapsed(const Duration(seconds: 1));
      expect(controller.skip(), isTrue);
      expect(controller.isVisualComplete, isTrue);
      expect(controller.startupReady, isFalse);

      controller.updateStartupReady(true);
      expect(controller.canFinish, isTrue);
    });
  });
}
