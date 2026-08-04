import 'package:flutter/foundation.dart';

final class BootSequenceController extends ChangeNotifier {
  BootSequenceController({bool reducedMotion = false})
      : _reducedMotion = reducedMotion,
        _visualProgress = reducedMotion ? 1 : 0;

  static const sequenceDuration = Duration(milliseconds: 5500);
  static const skipGuard = Duration(seconds: 1);

  bool _reducedMotion;
  Duration _elapsed = Duration.zero;
  double _visualProgress;
  bool _startupReady = false;
  bool _skipped = false;

  bool get reducedMotion => _reducedMotion;
  Duration get elapsed => _elapsed;
  double get visualProgress => _visualProgress;
  bool get startupReady => _startupReady;
  bool get wasSkipped => _skipped;
  bool get isVisualComplete => _visualProgress >= 1;
  bool get canSkip => _reducedMotion || canSkipAt(_elapsed);
  bool get canFinish => isVisualComplete && _startupReady;

  void updateReducedMotion(bool value) {
    if (_reducedMotion == value) {
      return;
    }
    _reducedMotion = value;
    if (value) {
      _visualProgress = 1;
    }
    notifyListeners();
  }

  void updateElapsed(Duration elapsed) {
    final normalizedElapsed = elapsed.isNegative ? Duration.zero : elapsed;
    final progress = normalizedElapsed.inMicroseconds /
        sequenceDuration.inMicroseconds;
    final nextProgress = (_reducedMotion || _skipped)
        ? 1.0
        : progress.clamp(0.0, 1.0).toDouble();
    if (_elapsed == normalizedElapsed && _visualProgress == nextProgress) {
      return;
    }
    _elapsed = normalizedElapsed;
    _visualProgress = nextProgress;
    notifyListeners();
  }

  bool skip() {
    if (!canSkip) {
      return false;
    }
    if (!_skipped || !isVisualComplete) {
      _skipped = true;
      _visualProgress = 1;
      notifyListeners();
    }
    return true;
  }

  void updateStartupReady(bool value) {
    if (_startupReady == value) {
      return;
    }
    _startupReady = value;
    notifyListeners();
  }

  static bool canSkipAt(Duration elapsed) {
    return elapsed >= skipGuard;
  }

  static double phaseProgress(double progress, double start, double end) {
    if (end <= start) {
      throw ArgumentError.value(end, 'end', 'Must be greater than start');
    }
    return ((progress - start) / (end - start)).clamp(0.0, 1.0).toDouble();
  }
}
