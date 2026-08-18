import 'package:flutter/widgets.dart';

import 'scramble_decode_engine.dart';

/// A single line of text revealed through [ScrambleDecodeEngine].
///
/// The engine is built once per text/seed pair and reused across frames, so
/// animating a line allocates only the returned string. Under reduced motion the
/// engine resolves immediately to the final text rather than animating.
class ScrambledLine extends StatefulWidget {
  const ScrambledLine({
    required this.text,
    required this.progress,
    required this.seed,
    required this.style,
    this.frameIndex = 0,
    this.reducedMotion = false,
    this.textAlign = TextAlign.center,
    this.semanticsLabel,
    super.key,
  });

  /// The final text the line resolves to.
  final String text;

  /// Reveal progress in 0..1.
  final double progress;

  /// Deterministic seed. The same seed and progress always give the same frame.
  final int seed;

  final TextStyle style;

  /// Advances the noise without advancing the reveal, so unresolved cells keep
  /// churning while the line is on screen.
  final int frameIndex;

  final bool reducedMotion;
  final TextAlign textAlign;

  /// Announced to assistive technology instead of the scrambled characters.
  final String? semanticsLabel;

  @override
  State<ScrambledLine> createState() => _ScrambledLineState();
}

class _ScrambledLineState extends State<ScrambledLine> {
  late ScrambleDecodeEngine _engine;

  @override
  void initState() {
    super.initState();
    _engine = _buildEngine();
  }

  @override
  void didUpdateWidget(ScrambledLine oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text ||
        oldWidget.seed != widget.seed ||
        oldWidget.reducedMotion != widget.reducedMotion) {
      _engine = _buildEngine();
    }
  }

  ScrambleDecodeEngine _buildEngine() {
    return ScrambleDecodeEngine(
      target: widget.text,
      seed: widget.seed,
      reducedMotion: widget.reducedMotion,
      revealMode: RevealMode.leftToRight,
    );
  }

  @override
  Widget build(BuildContext context) {
    final frame = _engine.frameAt(
      widget.progress,
      frameIndex: widget.frameIndex,
    );
    // The scrambled characters are decoration; assistive technology reads the
    // resolved text so a screen reader never announces noise.
    return Semantics(
      label: widget.semanticsLabel ?? widget.text,
      child: ExcludeSemantics(
        child: Text(frame, textAlign: widget.textAlign, style: widget.style),
      ),
    );
  }
}
