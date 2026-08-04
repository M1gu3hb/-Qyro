import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/app.dart';
import 'package:qyro/boot/ascii_logo_model.dart';
import 'package:qyro/boot/boot_screen.dart';
import 'package:qyro/startup/native_bridge.dart';
import 'package:qyro/startup/startup_coordinator.dart';

void main() {
  testWidgets('skip never bypasses an obligatory startup task', (tester) async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(loadNativeBridge: () => native.future);
    var finished = false;

    await tester.pumpWidget(
      MaterialApp(
        home: BootScreen(
          coordinator: coordinator,
          logoModel: _logo(),
          onFinished: () => finished = true,
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1100));
    await tester.tap(find.byKey(const Key('boot-skip')));
    await tester.pump();

    expect(finished, isFalse);
    expect(coordinator.snapshot.currentTask, StartupTask.nativeBridge);

    native.complete(const _FakeBridge());
    await tester.pumpAndSettle();

    expect(finished, isTrue);
  });

  testWidgets('reduced motion still waits for startup readiness',\n      (tester) async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(loadNativeBridge: () => native.future);
    var finished = false;

    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: BootScreen(
            coordinator: coordinator,
            logoModel: _logo(),
            onFinished: () => finished = true,
          ),
        ),
      ),
    );
    await tester.pump();

    expect(finished, isFalse);

    native.complete(const _FakeBridge());
    await tester.pumpAndSettle();

    expect(finished, isTrue);
  });

  testWidgets('ASCII logo is one semantic image and never a primary PNG',
      (tester) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(
      MaterialApp(
        home: BootScreen(
          coordinator: _coordinator(),
          logoModel: _logo(),
          onFinished: () {},
        ),
      ),
    );
    await tester.pump();

    expect(find.bySemanticsLabel('Logo de Qyro'), findsOneWidget);
    expect(find.byType(Image), findsNothing);

    semantics.dispose();
  });

  testWidgets('keyboard skip works after the guard', (tester) async {
    var finished = false;

    await tester.pumpWidget(
      MaterialApp(
        home: BootScreen(
          coordinator: _coordinator(),
          logoModel: _logo(),
          onFinished: () => finished = true,
        ),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1100));
    await tester.sendKeyEvent(LogicalKeyboardKey.enter);
    await tester.pumpAndSettle();

    expect(finished, isTrue);
  });

  testWidgets('failure offers retry and preserves safe diagnostics',
      (tester) async {
    var attempts = 0;
    final coordinator = _coordinator(
      verifyAssets: () async {
        attempts += 1;
        if (attempts == 1) {
          throw const StartupTaskFailure(
            code: 'asset_invalid',
            userMessageKey: 'startupAssetInvalid',
            technicalSummary: 'Generated ASCII logo failed validation',
          );
        }
      },
    );
    var finished = false;

    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: BootScreen(
            coordinator: coordinator,
            logoModel: _logo(),
            onFinished: () => finished = true,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('boot-retry')), findsOneWidget);
    expect(find.textContaining('asset_invalid'), findsOneWidget);

    await tester.tap(find.byKey(const Key('boot-retry')));
    await tester.pumpAndSettle();

    expect(attempts, 2);
    expect(finished, isTrue);
  });

  testWidgets('Home keeps transfer actions visibly disabled', (tester) async {
    await tester.pumpWidget(
      QyroApp(
        startupCoordinator: _coordinator(),
        bootLogoModel: _logo(),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1100));
    await tester.tap(find.byKey(const Key('boot-skip')));
    await tester.pumpAndSettle();

    final actions = tester.widgetList<FilledButton>(find.byType(FilledButton));
    expect(actions, isNotEmpty);
    expect(actions.every((button) => button.onPressed == null), isTrue);
    expect(find.text('Enviar'), findsOneWidget);
    expect(find.text('Recibir'), findsOneWidget);
  });
}

StartupCoordinator _coordinator({
  Future<void> Function()? verifyAssets,
  Future<NativeBridge> Function()? loadNativeBridge,
}) {
  return StartupCoordinator(
    loadBranding: () async => const StartupBranding(isProvisional: true),
    verifyAssets: verifyAssets ?? () async {},
    loadNativeBridge: loadNativeBridge ?? () async => const _FakeBridge(),
    initializeInterface: () async {},
  );
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

final class _FakeBridge implements NativeBridge {
  const _FakeBridge();

  @override
  String protocolVersion() => 'QYRO/1';
}
