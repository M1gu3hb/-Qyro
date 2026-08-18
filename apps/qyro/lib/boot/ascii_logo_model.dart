import 'dart:convert';

final class AsciiLogoModel {
  const AsciiLogoModel._({
    required this.width,
    required this.height,
    required this.aspectRatio,
    required this.rows,
    required this.target,
    required this.mask,
    required this.density,
    required this.sourceChecksum,
    required this.generatorVersion,
    required this.provisional,
  });

  factory AsciiLogoModel.fromJsonString(String source) {
    final Object? decoded;
    try {
      decoded = jsonDecode(source);
    } on FormatException {
      rethrow;
    }
    if (decoded is! Map<String, Object?>) {
      throw const FormatException('ASCII logo root must be a JSON object');
    }
    return AsciiLogoModel.fromJson(decoded);
  }

  factory AsciiLogoModel.fromJson(Map<String, Object?> json) {
    final width = _readInt(json, 'width');
    final height = _readInt(json, 'height');
    final aspectRatio = _readDouble(json, 'aspectRatio');
    if (width <= 0 || height <= 0 || aspectRatio <= 0) {
      throw const FormatException(
        'ASCII logo dimensions and aspect ratio must be positive',
      );
    }

    final rows = _readStringRows(json, 'characterCells', height);
    final maskRows = _readStringRows(json, 'mask', height);
    final densityRows = _readListRows(json, 'density', height);
    final mask = <bool>[];
    final density = <double>[];

    for (var rowIndex = 0; rowIndex < height; rowIndex++) {
      final rowRunes = rows[rowIndex].runes.toList(growable: false);
      final maskRunes = maskRows[rowIndex].runes.toList(growable: false);
      if (rowRunes.length != width || maskRunes.length != width) {
        throw FormatException(
          'ASCII logo row $rowIndex must contain exactly $width cells',
        );
      }
      if (densityRows[rowIndex].length != width) {
        throw FormatException(
          'ASCII logo density row $rowIndex must contain exactly $width cells',
        );
      }

      for (var column = 0; column < width; column++) {
        final maskRune = maskRunes[column];
        if (maskRune != 0x30 && maskRune != 0x31) {
          throw FormatException(
            'ASCII logo mask cell $rowIndex:$column must be 0 or 1',
          );
        }
        mask.add(maskRune == 0x31);

        final value = densityRows[rowIndex][column];
        if (value is! num || value < 0 || value > 1) {
          throw FormatException(
            'ASCII logo density cell $rowIndex:$column must be in 0..1',
          );
        }
        density.add(value.toDouble());
      }
    }

    return AsciiLogoModel._(
      width: width,
      height: height,
      aspectRatio: aspectRatio,
      rows: List<String>.unmodifiable(rows),
      target: rows.join(),
      mask: List<bool>.unmodifiable(mask),
      density: List<double>.unmodifiable(density),
      sourceChecksum: _readString(json, 'sourceChecksum'),
      generatorVersion: _readString(json, 'generatorVersion'),
      provisional: _readBool(json, 'provisional'),
    );
  }

  final int width;
  final int height;
  final double aspectRatio;
  final List<String> rows;
  final String target;
  final List<bool> mask;
  final List<double> density;
  final String sourceChecksum;
  final String generatorVersion;
  final bool provisional;

  int get cellCount => width * height;

  static int _readInt(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is! int) {
      throw FormatException('ASCII logo field "$key" must be an integer');
    }
    return value;
  }

  static double _readDouble(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is! num) {
      throw FormatException('ASCII logo field "$key" must be a number');
    }
    return value.toDouble();
  }

  static String _readString(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is! String || value.isEmpty) {
      throw FormatException(
        'ASCII logo field "$key" must be a non-empty string',
      );
    }
    return value;
  }

  static bool _readBool(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is! bool) {
      throw FormatException('ASCII logo field "$key" must be a boolean');
    }
    return value;
  }

  static List<String> _readStringRows(
    Map<String, Object?> json,
    String key,
    int height,
  ) {
    final values = _readRows(json, key, height);
    final rows = <String>[];
    for (final value in values) {
      if (value is! String) {
        throw FormatException(
          'ASCII logo field "$key" must contain only strings',
        );
      }
      rows.add(value);
    }
    return rows;
  }

  static List<List<Object?>> _readListRows(
    Map<String, Object?> json,
    String key,
    int height,
  ) {
    final values = _readRows(json, key, height);
    final rows = <List<Object?>>[];
    for (final value in values) {
      if (value is! List<Object?>) {
        throw FormatException(
          'ASCII logo field "$key" must contain only arrays',
        );
      }
      rows.add(value);
    }
    return rows;
  }

  static List<Object?> _readRows(
    Map<String, Object?> json,
    String key,
    int height,
  ) {
    final value = json[key];
    if (value is! List<Object?> || value.length != height) {
      throw FormatException(
        'ASCII logo field "$key" must contain exactly $height rows',
      );
    }
    return value;
  }
}
