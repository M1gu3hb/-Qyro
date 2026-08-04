import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import '../../../tools/branding_generator/lib/branding_generator.dart';

void main() {
  const generator = BrandingGenerator();

  Map<String, Object?> validConfig() => <String, Object?>{
        'appName': 'Qyro',
        'creatorName': 'Ada Example',
        'studioName': 'Example Studio',
        'signatureText': 'Built by Ada Example',
        'bundleIdBase': 'dev.example.qyro',
        'website': 'https://example.test',
        'repository': 'https://github.com/example/qyro',
        'primaryColor': '#168BFF',
        'secondaryColor': '#51C8FF',
        'backgroundColor': '#03070D',
      };

  test('valid branding generates non-provisional compile-time constants', () {
    final result = generator.generate(jsonEncode(validConfig()));

    expect(result.isProvisional, isFalse);
    expect(result.dartSource, contains('static const appName = "Qyro";'));
    expect(
      result.dartSource,
      contains('static const primaryColorValue = 0xFF168BFF;'),
    );
    expect(result.provisionalFields, isEmpty);
  });

  test('placeholder values are detected and not exposed as visible branding', () {
    final config = validConfig()
      ..['creatorName'] = 'REPLACE_WITH_CREATOR_NAME'
      ..['signatureText'] = 'Built by REPLACE_WITH_CREATOR_NAME'
      ..['bundleIdBase'] = 'com.owner.qyro';

    final result = generator.generate(jsonEncode(config));

    expect(result.isProvisional, isTrue);
    expect(
      result.provisionalFields,
      containsAll(<String>['creatorName', 'signatureText', 'bundleIdBase']),
    );
    expect(result.dartSource, contains('static const creatorName = "";'));
    expect(result.dartSource, contains('static const signatureText = "";'));
    expect(result.dartSource, isNot(contains('REPLACE_WITH_CREATOR_NAME')));
  });

  test('public generation rejects provisional branding', () {
    final config = validConfig()..['bundleIdBase'] = 'com.owner.qyro';

    expect(
      () => generator.generate(jsonEncode(config), requireFinal: true),
      throwsA(
        isA<BrandingValidationException>().having(
          (error) => error.message,
          'message',
          contains('provisional'),
        ),
      ),
    );
  });

  test('invalid colors, control characters, and bundle IDs are rejected', () {
    for (final invalid in <Map<String, Object?>>[
      validConfig()..['primaryColor'] = 'blue',
      validConfig()..['appName'] = 'Qyro\nInjected',
      validConfig()..['bundleIdBase'] = 'not a bundle id',
    ]) {
      expect(
        () => generator.generate(jsonEncode(invalid)),
        throwsA(isA<BrandingValidationException>()),
      );
    }
  });

  test('committed branding is generated from the development fallback', () {
    final example =
        File('../../config/branding.example.json').readAsStringSync();
    final committed = File('lib/generated/branding.g.dart').readAsStringSync();

    expect(committed, generator.generate(example).dartSource);
  });
}
