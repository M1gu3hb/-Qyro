/// Deterministic text scramble/reveal engine.
///
/// Rendering remains a UI concern; this class only calculates frames and does
/// no I/O, allocation-heavy setup, or timer work.
final class ScrambleDecodeEngine {
  ScrambleDecodeEngine({
    required this.target,
    required this.seed,
    this.reducedMotion = false,
    this.noiseAlphabet = _defaultNoiseAlphabet,
  }) : assert(noiseAlphabet.length > 1, 'Noise alphabet is too small');

  static const String _defaultNoiseAlphabet =
      r'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#@$%&*+=<>/|.:;[]{}()_';

  final String target;
  final int seed;
  final bool reducedMotion;
  final String noiseAlphabet;

  late final List<int> _targetRunes = target.runes.toList(growable: false);
  late final List<int> _noiseRunes = noiseAlphabet.runes.toList(growable: false);

  /// Returns how many target cells are permanently revealed.
  int revealedCellCount(double progress) {
    if (reducedMotion) {
      return _targetRunes.length;
    }

    final normalized = progress.clamp(0.0, 1.0);
    if (normalized == 1) {
      return _targetRunes.length;
    }
    return (_targetRunes.length * normalized).floor();
  }

  /// Calculates a deterministic frame at [progress].
  String frameAt(double progress) {
    if (reducedMotion || progress >= 1) {
      return target;
    }

    final revealed = revealedCellCount(progress);
    final frame = List<int>.of(_targetRunes, growable: false);
    var state = seed & 0x7fffffff;

    for (var index = revealed; index < frame.length; index++) {
      state = _next(state ^ index);
      var rune = _noiseRunes[state % _noiseRunes.length];
      if (rune == _targetRunes[index]) {
        rune = _noiseRunes[(state + 1) % _noiseRunes.length];
      }
      frame[index] = rune;
    }

    return String.fromCharCodes(frame);
  }

  static int _next(int value) =>
      (value * 1103515245 + 12345) & 0x7fffffff;
}
