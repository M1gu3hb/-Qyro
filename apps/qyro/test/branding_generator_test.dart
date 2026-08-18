import 'dart:convert';
import 'dart:io';

import 'package:branding_generator/branding_generator.dart';
import 'package:flutter_test/flutter_test.dart';

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

  test('valid config emits constants', () {
    final result = generator.generate(jsonEncode(validConfig()));
    const colorConstant = 'static const primaryColorValue = 0xFF168BFF;';

    expect(result.isProvisional, isFalse);
    expect(result.dartSource, contains('static const appName = "Qyro";'));
    expect(result.dartSource, contains(colorConstant));
    expect(result.provisionalFields, isEmpty);
  });

  test('placeholders are provisional and hidden', () {
    final config = validConfig()
      ..['creatorName'] = 'REPLACE_WITH_CREATOR_NAME'
      ..['signatureText'] = 'Built by REPLACE_WITH_CREATOR_NAME'
      ..['bundleIdBase'] = 'com.owner.qyro';
    final result = generator.generate(jsonEncode(config));
    final expectedFields = <String>[
      'creatorName',
      'signatureText',
      'bundleIdBase',
    ];

    expect(result.isProvisional, isTrue);
    expect(result.provisionalFields, containsAll(expectedFields));
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

  test('invalid values are rejected', () {
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

  test('committed branding matches the development fallback', () {
    final example = File('../../config/branding.example.json');
    final committed = File('lib/generated/branding.g.dart');
    final expected = generator.generate(example.readAsStringSync()).dartSource;

    expect(committed.readAsStringSync(), expected);
  });
}
