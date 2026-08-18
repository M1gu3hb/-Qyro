import 'dart:math' as math;

enum RevealMode {
  leftToRight,
  columns,
  radial,
  mask,
  luminance,
}

/// Deterministic, precomputed scramble/reveal engine.
///
/// Target cells, reveal thresholds, random seeds, alpha, and color are allocated
/// once. [frameAt] reuses its rune buffer; only the returned Dart string is new.
final class ScrambleDecodeEngine {
  ScrambleDecodeEngine({
    required this.target,
    required this.seed,
    int? width,
    this.reducedMotion = false,
    this.revealMode = RevealMode.leftToRight,
    this.noiseAlphabet = _defaultNoiseAlphabet,
    this.noiseIntensity = 1,
    List<bool>? mask,
    List<double>? luminance,
    List<double>? targetAlphas,
    List<int>? targetColors,
    this.onProgress,
  })  : assert(noiseAlphabet.length > 1, 'Noise alphabet is too small'),
        assert(
          noiseIntensity >= 0 && noiseIntensity <= 1,
          'Noise intensity must be in the range 0..1',
        ),
        _targetRunes = target.runes.toList(growable: false),
        _noiseRunes = noiseAlphabet.runes.toList(growable: false),
        width = width ?? math.max(1, target.runes.length) {
    if (this.width <= 0) {
      throw ArgumentError.value(this.width, 'width', 'Must be positive');
    }
    _validateLength(mask, 'mask');
    _validateLength(luminance, 'luminance');
    _validateLength(targetAlphas, 'targetAlphas');
    _validateLength(targetColors, 'targetColors');

    _mask = List<bool>.generate(
      cellCount,
      (index) => mask?[index] ?? true,
      growable: false,
    );
    _luminance = List<double>.generate(
      cellCount,
      (index) => (luminance?[index] ?? 1).clamp(0, 1).toDouble(),
      growable: false,
    );
    _targetAlphas = List<double>.generate(
      cellCount,
      (index) => (targetAlphas?[index] ?? 1).clamp(0, 1).toDouble(),
      growable: false,
    );
    _targetColors = List<int>.generate(
      cellCount,
      (index) => targetColors?[index] ?? 0xFF51C8FF,
      growable: false,
    );
    _cellSeeds = List<int>.generate(
      cellCount,
      (index) => _mix(seed ^ (index * 0x45D9F3B)),
      growable: false,
    );
    _revealThresholds = List<double>.generate(
      cellCount,
      _thresholdFor,
      growable: false,
    );
    _frameRunes = List<int>.filled(cellCount, 0x20, growable: false);
    _maskedTarget = String.fromCharCodes(
      List<int>.generate(
        cellCount,
        (index) => _mask[index] ? _targetRunes[index] : 0x20,
        growable: false,
      ),
    );
  }

  static const String _defaultNoiseAlphabet =
      r'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#@$%&*+=<>/\\|.:;[]{}()_';

  final String target;
  final int seed;
  final int width;
  final bool reducedMotion;
  final RevealMode revealMode;
  final String noiseAlphabet;
  final double noiseIntensity;
  final void Function(double progress)? onProgress;

  final List<int> _targetRunes;
  final List<int> _noiseRunes;
  late final List<bool> _mask;
  late final List<double> _luminance;
  late final List<double> _targetAlphas;
  late final List<int> _targetColors;
  late final List<int> _cellSeeds;
  late final List<double> _revealThresholds;
  late final List<int> _frameRunes;
  late final String _maskedTarget;

  var _cancelled = false;
  var _skipped = false;
  String? _lastFrame;

  int get cellCount => _targetRunes.length;
  int get height => cellCount == 0 ? 0 : (cellCount / width).ceil();
  bool get isCancelled => _cancelled;

  void skip() {
    _skipped = true;
    _lastFrame = _maskedTarget;
  }

  void cancel() {
    _cancelled = true;
  }

  int revealedCellCount(double progress) {
    final normalized = progress.clamp(0.0, 1.0).toDouble();
    if (reducedMotion || _skipped) {
      return _mask.where((included) => included).length;
    }
    var count = 0;
    for (var index = 0; index < cellCount; index++) {
      if (isCellRevealed(index, normalized)) {
        count += 1;
      }
    }
    return count;
  }

  bool isCellRevealed(int index, double progress) {
    RangeError.checkValidIndex(index, _targetRunes, 'index');
    if (!_mask[index]) {
      return false;
    }
    if (reducedMotion || _skipped) {
      return true;
    }
    final normalized = progress.clamp(0.0, 1.0).toDouble();
    return normalized >= _revealThresholds[index];
  }

  double targetAlphaAt(int index) {
    RangeError.checkValidIndex(index, _targetAlphas, 'index');
    return _targetAlphas[index];
  }

  int targetColorAt(int index) {
    RangeError.checkValidIndex(index, _targetColors, 'index');
    return _targetColors[index];
  }

  String frameAt(double progress, {int frameIndex = 0}) {
    if (_cancelled) {
      return _lastFrame ?? _maskedTarget;
    }

    final normalized = progress.clamp(0.0, 1.0).toDouble();
    onProgress?.call(normalized);
    if (cellCount == 0) {
      _lastFrame = '';
      return '';
    }
    if (reducedMotion || _skipped || normalized >= 1) {
      _lastFrame = _maskedTarget;
      return _maskedTarget;
    }

    for (var index = 0; index < cellCount; index++) {
      if (!_mask[index]) {
        _frameRunes[index] = 0x20;
      } else if (isCellRevealed(index, normalized)) {
        _frameRunes[index] = _targetRunes[index];
      } else {
        _frameRunes[index] = _noiseRune(index, frameIndex);
      }
    }
    _lastFrame = String.fromCharCodes(_frameRunes);
    return _lastFrame!;
  }

  int _noiseRune(int index, int frameIndex) {
    final state = _mix(_cellSeeds[index] ^ (frameIndex * 0x27D4EB2D));
    final strength = (state & 0xFFFF) / 0xFFFF;
    if (strength > noiseIntensity) {
      return 0x20;
    }
    var rune = _noiseRunes[state % _noiseRunes.length];
    if (rune == _targetRunes[index]) {
      rune = _noiseRunes[(state + 1) % _noiseRunes.length];
    }
    return rune;
  }

  double _thresholdFor(int index) {
    if (cellCount == 0 || !_mask[index]) {
      return 1;
    }
    final x = index % width;
    final y = index ~/ width;
    return switch (revealMode) {
      RevealMode.leftToRight => (index + 1) / cellCount,
      RevealMode.columns => width == 1 ? 0 : x / (width - 1),
      RevealMode.radial => _radialThreshold(x, y),
      RevealMode.mask => _maskedThreshold(index),
      RevealMode.luminance => 1 - _luminance[index],
    };
  }

  double _radialThreshold(int x, int y) {
    final rows = math.max(1, height);
    final centerX = (width - 1) / 2;
    final centerY = (rows - 1) / 2;
    final maximum = math.sqrt(centerX * centerX + centerY * centerY);
    if (maximum == 0) {
      return 0;
    }
    final deltaX = x - centerX;
    final deltaY = y - centerY;
    return math.sqrt(deltaX * deltaX + deltaY * deltaY) / maximum;
  }

  double _maskedThreshold(int index) {
    var activeBefore = 0;
    var activeTotal = 0;
    for (var candidate = 0; candidate < cellCount; candidate++) {
      if (_mask[candidate]) {
        activeTotal += 1;
        if (candidate < index) {
          activeBefore += 1;
        }
      }
    }
    return activeTotal == 0 ? 1 : (activeBefore + 1) / activeTotal;
  }

  void _validateLength(List<Object?>? values, String name) {
    if (values != null && values.length != cellCount) {
      throw ArgumentError.value(
        values.length,
        name,
        'Must contain exactly $cellCount values',
      );
    }
  }

  static int _mix(int value) {
    var mixed = value & 0x7FFFFFFF;
    mixed = ((mixed ^ (mixed >> 16)) * 0x45D9F3B) & 0x7FFFFFFF;
    mixed = ((mixed ^ (mixed >> 16)) * 0x45D9F3B) & 0x7FFFFFFF;
    return (mixed ^ (mixed >> 16)) & 0x7FFFFFFF;
  }
}
