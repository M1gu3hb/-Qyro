import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/app.dart';
import 'package:qyro/boot/ascii_logo_model.dart';
import 'package:qyro/boot/boot_screen.dart';
import 'package:qyro/l10n/generated/app_localizations.dart';
import 'package:qyro/startup/native_bridge.dart';
import 'package:qyro/startup/startup_coordinator.dart';

void main() {
  testWidgets('skip never bypasses an obligatory startup task', (tester) async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(loadNativeBridge: () => native.future);
    var finished = false;

    await tester.pumpWidget(
      _TestApp(
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

  testWidgets('reduced motion still waits for startup readiness',
      (tester) async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(loadNativeBridge: () => native.future);
    var finished = false;

    await tester.pumpWidget(
      _TestApp(
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
      _TestApp(
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
      _TestApp(
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
      _TestApp(
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

  testWidgets('tras un fallo, SALTAR se apaga y REINTENTAR es el unico vivo',
      (tester) async {
    // **QYR-0397.** `_skip()` pone el progreso visual a 1 y llama a
    // `_maybeFinish`, que exige `canFinish` -- y eso exige `startupReady`, que
    // es falso justo despues de un fallo. Asi que el boton quedaba **encendido
    // y muerto**: pulsarlo no navegaba, no avisaba, no hacia nada. Un control
    // encendido que no responde ensena que la aplicacion se colgo.
    final coordinator = _coordinator(
      verifyAssets: () async {
        throw const StartupTaskFailure(
          code: 'asset_invalid',
          userMessageKey: 'startupAssetInvalid',
          technicalSummary: 'Generated ASCII logo failed validation',
        );
      },
    );
    var finished = false;

    await tester.pumpWidget(
      _TestApp(
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

    expect(
      find.byKey(const Key('boot-retry')),
      findsOneWidget,
      reason: 'sin reintentar, apagar saltar dejaria la pantalla sin salida',
    );
    expect(
      tester
          .widget<ButtonStyleButton>(find.byKey(const Key('boot-skip')))
          .onPressed,
      isNull,
      reason: 'SALTAR sigue encendido tras el fallo, y no puede hacer nada',
    );

    // Y el toque en cualquier parte de la pantalla tampoco finge que sirve.
    await tester.tapAt(const Offset(20, 20));
    await tester.pumpAndSettle();
    expect(finished, isFalse);
  });

  testWidgets('English locale renders the complete Home baseline',
      (tester) async {
    await tester.pumpWidget(
      QyroApp(
        locale: const Locale('en'),
        startupCoordinator: _coordinator(),
        bootLogoModel: _logo(),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1100));
    await tester.tap(find.byKey(const Key('boot-skip')));
    await tester.pumpAndSettle();

    expect(find.text('Send'), findsOneWidget);
    expect(find.text('Receive'), findsOneWidget);
    expect(
      find.text('Send a file to another device on this network.'),
      findsOneWidget,
    );
    // The sentence that explained why the buttons were off is gone. Asserting
    // its absence and not only the new one's presence, because leaving it in
    // place beside working buttons would be lying in the other direction
    // (ADR-0036 §5).
    expect(
      find.text('Transfer features are not implemented yet.'),
      findsNothing,
    );
  });

  testWidgets('Home offers both transfer actions and neither is disabled',
      (tester) async {
    // This test used to assert the opposite, and it was right to: the buttons
    // were `onPressed: null` from the first commit because enabling them before
    // the engine existed would have been the one lie this project spent seven
    // months not telling.
    //
    // The five conditions of ADR-0036 §5 are met and evidenced in
    // `docs/reports/fase-05-la-interfaz-y-los-botones.md`, so the assertion is
    // inverted rather than deleted — the property still matters, its value
    // changed.
    await tester.pumpWidget(
      QyroApp(
        locale: const Locale('es'),
        startupCoordinator: _coordinator(),
        bootLogoModel: _logo(),
      ),
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1100));
    await tester.tap(find.byKey(const Key('boot-skip')));
    await tester.pumpAndSettle();

    final buttons = tester.widgetList<FilledButton>(find.byType(FilledButton));
    expect(
      buttons.length,
      2,
      reason: 'Home should offer exactly Send and Receive, found '
          '${buttons.length}',
    );
    for (final button in buttons) {
      expect(
        button.onPressed,
        isNotNull,
        reason: 'a transfer action is still disabled',
      );
    }
    expect(find.text('Enviar'), findsOneWidget);
    expect(find.text('Recibir'), findsOneWidget);
    expect(
      find.text('Funciones de transferencia aún no implementadas.'),
      findsNothing,
    );
  });
}

class _TestApp extends StatelessWidget {
  const _TestApp({required this.home});

  final Widget home;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      locale: const Locale('es'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: home,
    );
  }
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
