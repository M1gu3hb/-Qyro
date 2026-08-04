import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/boot/ascii_logo_model.dart';
import 'package:qyro/boot/ascii_logo_painter.dart';
import 'package:qyro/boot/boot_status_model.dart';
import 'package:qyro/boot/cipher_rain_painter.dart';
import 'package:qyro/boot/scramble_decode_engine.dart';
import 'package:qyro/startup/startup_coordinator.dart';

void main() {
  test('boot status reports obligatory task progress and safe message keys',
      () {
    final status = BootStatusModel.fromSnapshot(
      StartupSnapshot(
        phase: StartupPhase.running,
        currentTask: StartupTask.nativeBridge,
        completedTasks: const <StartupTask>{
          StartupTask.branding,
          StartupTask.assets,
        },
        reducedMotion: false,
        lifecycleState: AppLifecycleState.resumed,
      ),
    );

    expect(status.progress, 0.5);
    expect(status.messageKey, 'startupNativeBridge');
    expect(status.isTerminalFailure, isFalse);
    expect(status.diagnosticCode, isNull);
  });

  test('boot status exposes sanitized terminal diagnostics', () {
    final status = BootStatusModel.fromSnapshot(
      StartupSnapshot(
        phase: StartupPhase.failed,
        completedTasks: const <StartupTask>{},
        reducedMotion: false,
        lifecycleState: AppLifecycleState.resumed,
        diagnostic: const StartupDiagnostic(
          code: 'library_not_found',
          userMessageKey: 'nativeBridgeUnavailable',
          technicalSummary: 'Native library not found: qyro_ffi.dll',
        ),
      ),
    );

    expect(status.isTerminalFailure, isTrue);
    expect(status.messageKey, 'nativeBridgeUnavailable');
    expect(status.diagnosticCode, 'library_not_found');
    expect(status.technicalSummary, contains('qyro_ffi.dll'));
  });

  test('ASCII painter renders a deterministic frame without the PNG asset', () {
    final model = _logo();
    final engine = ScrambleDecodeEngine(
      target: model.target,
      width: model.width,
      seed: 0x5159524F,
      revealMode: RevealMode.luminance,
      mask: model.mask,
      luminance: model.density,
    );
    final painter = AsciiLogoPainter(
      model: model,
      engine: engine,
      progress: 0.5,
      frameIndex: 7,
      color: const Color(0xFF51C8FF),
    );
    final recorder = ui.PictureRecorder();
    final canvas = ui.Canvas(recorder);

    expect(
      () => painter.paint(canvas, const Size(240, 120)),
      returnsNormally,
    );
    recorder.endRecording();

    expect(
      painter.shouldRepaint(
        AsciiLogoPainter(
          model: model,
          engine: engine,
          progress: 0.6,
          frameIndex: 8,
          color: const Color(0xFF51C8FF),
        ),
      ),
      isTrue,
    );
  });

  test('cipher rain uses the dark blue Qyro palette and paints safely', () {
    const painter = CipherRainPainter(progress: 0.4, seed: 0x5159524F);
    final recorder = ui.PictureRecorder();
    final canvas = ui.Canvas(recorder);

    expect(painter.rainColor, const Color(0xFF168BFF));
    expect(
      () => painter.paint(canvas, const Size(320, 180)),
      returnsNormally,
    );
    recorder.endRecording();
  });
}

AsciiLogoModel _logo() {
  return AsciiLogoModel.fromJsonString('''
{
  "width": 4,
  "height": 2,
  "aspectRatio": 2.0,
  "characterCells": [" Q  ", "YRO "],
  "mask": ["0100", "1110"],
  "density": [[0.0, 1.0, 0.0, 0.0], [0.8, 0.9, 1.0, 0.0]],
  "sourceChecksum": "sha256:test",
  "generatorVersion": "test",
  "provisional": true
}
''');
}
