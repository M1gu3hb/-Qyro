import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/boot/scrambled_line.dart';
import 'package:qyro/generated/branding.g.dart';
import 'package:qyro/l10n/generated/app_localizations.dart';

Widget _host(Widget child, {Locale locale = const Locale('en')}) {
  return MaterialApp(
    locale: locale,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: Center(child: child)),
  );
}

String _renderedText(WidgetTester tester, Key key) {
  final text = tester.widget<Text>(
    find.descendant(of: find.byKey(key), matching: find.byType(Text)),
  );
  return text.data ?? '';
}

void main() {
  const style = TextStyle(fontFamily: 'monospace', fontSize: 14);
  const key = Key('line');

  testWidgets('a line does not show its final text from the first frame',
      (tester) async {
    await tester.pumpWidget(
      _host(
        const ScrambledLine(
          key: key,
          text: 'QYRO',
          progress: 0,
          seed: 0x5159524D,
          style: style,
        ),
      ),
    );

    final rendered = _renderedText(tester, key);
    expect(rendered.length, 'QYRO'.length);
    expect(
      rendered,
      isNot('QYRO'),
      reason: 'the wordmark must start scrambled, not resolved',
    );
  });

  testWidgets('a line resolves to its exact text at full progress',
      (tester) async {
    await tester.pumpWidget(
      _host(
        const ScrambledLine(
          key: key,
          text: 'QYRO',
          progress: 1,
          seed: 0x5159524D,
          style: style,
        ),
      ),
    );

    expect(_renderedText(tester, key), 'QYRO');
  });

  testWidgets('the same seed and progress always produce the same frame',
      (tester) async {
    Future<String> frameFor(double progress) async {
      await tester.pumpWidget(
        _host(
          ScrambledLine(
            key: key,
            text: 'QYRO',
            progress: progress,
            seed: 0x5159524D,
            frameIndex: 12,
            style: style,
          ),
        ),
      );
      return _renderedText(tester, key);
    }

    final first = await frameFor(0.4);
    await frameFor(0.9);
    final again = await frameFor(0.4);
    expect(again, first, reason: 'frames must be deterministic for a seed');
  });

  testWidgets('the noise keeps churning while the reveal stands still',
      (tester) async {
    Future<String> frameAt(int frameIndex) async {
      await tester.pumpWidget(
        _host(
          ScrambledLine(
            key: key,
            text: 'QYRO WORDMARK',
            progress: 0.35,
            seed: 0x5159524D,
            frameIndex: frameIndex,
            style: style,
          ),
        ),
      );
      return _renderedText(tester, key);
    }

    final frames = <String>{};
    for (final index in [0, 5, 11, 19, 27]) {
      frames.add(await frameAt(index));
    }
    expect(
      frames.length,
      greaterThan(1),
      reason: 'unresolved cells must keep changing between frames',
    );
  });

  testWidgets('reduced motion resolves immediately without animating',
      (tester) async {
    await tester.pumpWidget(
      _host(
        const ScrambledLine(
          key: key,
          text: 'QYRO',
          progress: 0,
          seed: 0x5159524D,
          reducedMotion: true,
          style: style,
        ),
      ),
    );

    expect(
      _renderedText(tester, key),
      'QYRO',
      reason: 'reduced motion must skip the scramble entirely',
    );
  });

  testWidgets('assistive technology reads the resolved text, never the noise',
      (tester) async {
    final handle = tester.ensureSemantics();
    await tester.pumpWidget(
      _host(
        const ScrambledLine(
          key: key,
          text: 'QYRO',
          progress: 0.2,
          seed: 0x5159524D,
          style: style,
        ),
      ),
    );

    expect(find.bySemanticsLabel('QYRO'), findsOneWidget);
    handle.dispose();
  });

  test('provisional branding carries no creator name to display', () {
    // The guarantee behind the boot signature: nothing in generated branding
    // can supply a name while the configuration is still provisional.
    expect(GeneratedBranding.isProvisional, isTrue);
    expect(GeneratedBranding.creatorName, isEmpty);
    expect(GeneratedBranding.signatureText, isEmpty);
    expect(
      GeneratedBranding.provisionalFields,
      containsAll(<String>['creatorName', 'signatureText']),
    );
  });
}
