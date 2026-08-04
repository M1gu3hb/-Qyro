import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/app.dart';
import 'package:qyro/boot/boot_screen.dart';

void main() {
  testWidgets('boot skip is guarded before entering Home', (tester) async {
    await tester.pumpWidget(const QyroApp());

    final skipBeforeGuard = tester.widget<TextButton>(
      find.widgetWithText(TextButton, 'OMITIR'),
    );
    expect(skipBeforeGuard.onPressed, isNull);

    await tester.pump(const Duration(milliseconds: 1100));

    final skipAfterGuard = tester.widget<TextButton>(
      find.widgetWithText(TextButton, 'OMITIR'),
    );
    expect(skipAfterGuard.onPressed, isNotNull);

    await tester.tap(find.widgetWithText(TextButton, 'OMITIR'));
    await tester.pump();

    expect(find.text('Enviar'), findsOneWidget);
    expect(find.text('Recibir'), findsOneWidget);
  });

  testWidgets('system reduced motion bypasses animation', (tester) async {
    var finished = false;

    await tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: BootScreen(onFinished: () => finished = true),
        ),
      ),
    );
    await tester.pump();

    expect(finished, isTrue);
  });
}
