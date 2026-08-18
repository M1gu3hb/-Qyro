import 'dart:convert';

final class BrandingValidationException implements Exception {
  const BrandingValidationException(this.message);

  final String message;

  @override
  String toString() => 'BrandingValidationException: $message';
}

final class BrandingGenerationResult {
  const BrandingGenerationResult({
    required this.dartSource,
    required this.isProvisional,
    required this.provisionalFields,
  });

  final String dartSource;
  final bool isProvisional;
  final List<String> provisionalFields;
}

final class BrandingGenerator {
  const BrandingGenerator();

  static const _requiredTextFields = <String>[
    'appName',
    'creatorName',
    'studioName',
    'signatureText',
  ];
  static const _colorFields = <String>[
    'primaryColor',
    'secondaryColor',
    'backgroundColor',
  ];
  static const _optionalTextFields = <String>['website', 'repository'];
  static const _maximumLengths = <String, int>{
    'appName': 40,
    'creatorName': 80,
    'studioName': 80,
    'signatureText': 120,
    'bundleIdBase': 150,
    'website': 240,
    'repository': 240,
  };

  static final _placeholder = RegExp(r'REPLACE_WITH_[A-Z0-9_]+');
  static final _hexColor = RegExp(r'^#[0-9A-Fa-f]{6}$');
  static final _bundleId = RegExp(
    r'^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*){2,}$',
  );
  static final _dangerousCharacters = RegExp(
    r'[\u0000-\u001F\u007F\u202A-\u202E\u2066-\u2069]',
  );

  BrandingGenerationResult generate(
    String jsonSource, {
    bool requireFinal = false,
  }) {
    final decoded = _decode(jsonSource);
    final values = <String, String>{};

    for (final field in _requiredTextFields) {
      values[field] = _readText(decoded, field, required: true);
    }
    values['bundleIdBase'] = _readText(
      decoded,
      'bundleIdBase',
      required: true,
    );
    for (final field in _optionalTextFields) {
      values[field] = _readText(decoded, field, required: false);
    }
    for (final field in _colorFields) {
      final color = _readText(decoded, field, required: true);
      if (!_hexColor.hasMatch(color)) {
        throw BrandingValidationException(
          '$field must use #RRGGBB hexadecimal format',
        );
      }
      values[field] = color.toUpperCase();
    }

    final bundleId = values['bundleIdBase']!;
    if (!_placeholder.hasMatch(bundleId) && !_bundleId.hasMatch(bundleId)) {
      throw const BrandingValidationException(
        'bundleIdBase must be a reverse-DNS identifier',
      );
    }

    final provisionalFields = <String>[];
    for (final entry in values.entries) {
      if (_placeholder.hasMatch(entry.value)) {
        provisionalFields.add(entry.key);
      }
    }
    if (bundleId == 'com.owner.qyro') {
      provisionalFields.add('bundleIdBase');
    }
    final uniqueProvisional = provisionalFields.toSet().toList()
      ..sort((left, right) => _fieldOrder(left).compareTo(_fieldOrder(right)));

    if (requireFinal && uniqueProvisional.isNotEmpty) {
      throw BrandingValidationException(
        'Branding is provisional: ${uniqueProvisional.join(', ')}',
      );
    }

    final visibleValues = Map<String, String>.of(values);
    for (final field in uniqueProvisional) {
      if (_placeholder.hasMatch(visibleValues[field] ?? '')) {
        visibleValues[field] = '';
      }
    }

    return BrandingGenerationResult(
      dartSource: _render(visibleValues, uniqueProvisional),
      isProvisional: uniqueProvisional.isNotEmpty,
      provisionalFields: List<String>.unmodifiable(uniqueProvisional),
    );
  }

  Map<String, Object?> _decode(String source) {
    try {
      final value = jsonDecode(source);
      if (value is! Map<String, Object?>) {
        throw const BrandingValidationException(
          'Branding root must be a JSON object',
        );
      }
      return value;
    } on FormatException catch (error) {
      throw BrandingValidationException(
        'Invalid branding JSON: ${error.message}',
      );
    }
  }

  String _readText(
    Map<String, Object?> values,
    String field, {
    required bool required,
  }) {
    final value = values[field];
    if (value == null && !required) {
      return '';
    }
    if (value is! String || (required && value.trim().isEmpty)) {
      throw BrandingValidationException(
        '$field must be a non-empty string',
      );
    }
    if (_dangerousCharacters.hasMatch(value)) {
      throw BrandingValidationException(
        '$field contains control or bidirectional characters',
      );
    }
    final maximum = _maximumLengths[field] ?? 7;
    if (value.runes.length > maximum) {
      throw BrandingValidationException(
        '$field exceeds the maximum length of $maximum',
      );
    }
    return value;
  }

  String _render(
    Map<String, String> values,
    List<String> provisionalFields,
  ) {
    String literal(String field) => jsonEncode(values[field]);
    String colorValue(String field) => values[field]!.substring(1);
    final provisional = provisionalFields.isNotEmpty;
    final fieldDeclaration = provisionalFields.isEmpty
        ? '  static const provisionalFields = <String>[];'
        : [
            '  static const provisionalFields = <String>[',
            for (final field in provisionalFields)
              '    ${jsonEncode(field)},',
            '  ];',
          ].join('\n');

    return '''// GENERATED CODE - DO NOT MODIFY BY HAND.
// Generated by tools/branding_generator.

abstract final class GeneratedBranding {
  static const appName = ${literal('appName')};
  static const creatorName = ${literal('creatorName')};
  static const studioName = ${literal('studioName')};
  static const signatureText = ${literal('signatureText')};
  static const bundleIdBase = ${literal('bundleIdBase')};
  static const website = ${literal('website')};
  static const repository = ${literal('repository')};
  static const primaryColorHex = ${literal('primaryColor')};
  static const secondaryColorHex = ${literal('secondaryColor')};
  static const backgroundColorHex = ${literal('backgroundColor')};
  static const primaryColorValue = 0xFF${colorValue('primaryColor')};
  static const secondaryColorValue = 0xFF${colorValue('secondaryColor')};
  static const backgroundColorValue = 0xFF${colorValue('backgroundColor')};
  static const isProvisional = $provisional;
$fieldDeclaration
}
''';
  }

  int _fieldOrder(String field) {
    const order = <String>[
      ..._requiredTextFields,
      'bundleIdBase',
      ..._optionalTextFields,
      ..._colorFields,
    ];
    final index = order.indexOf(field);
    return index < 0 ? order.length : index;
  }
}
