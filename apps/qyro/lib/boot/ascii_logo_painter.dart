import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'ascii_logo_model.dart';
import 'scramble_decode_engine.dart';

final class AsciiLogoPainter extends CustomPainter {
  const AsciiLogoPainter({
    required this.model,
    required this.engine,
    required this.progress,
    required this.frameIndex,
    required this.color,
  });

  final AsciiLogoModel model;
  final ScrambleDecodeEngine engine;
  final double progress;
  final int frameIndex;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty || model.cellCount == 0) {
      return;
    }

    final frame = engine.frameAt(progress, frameIndex: frameIndex);
    final buffer = StringBuffer();
    for (var row = 0; row < model.height; row++) {
      final start = row * model.width;
      buffer.write(frame.substring(start, start + model.width));
      if (row + 1 < model.height) {
        buffer.writeln();
      }
    }

    final cellWidth = size.width / model.width;
    final cellHeight = size.height / model.height;
    final fontSize = math.min(cellWidth / 0.61, cellHeight);
    final painter = TextPainter(
      text: TextSpan(
        text: buffer.toString(),
        style: TextStyle(
          color: color,
          fontFamily: 'monospace',
          fontSize: fontSize,
          height: 1,
          letterSpacing: 0,
        ),
      ),
      textDirection: TextDirection.ltr,
      textHeightBehavior: const TextHeightBehavior(
        applyHeightToFirstAscent: false,
        applyHeightToLastDescent: false,
      ),
    )..layout(maxWidth: size.width);

    final offset = Offset(
      (size.width - painter.width) / 2,
      (size.height - painter.height) / 2,
    );
    painter.paint(canvas, offset);
  }

  @override
  bool shouldRepaint(covariant AsciiLogoPainter oldDelegate) {
    return oldDelegate.model != model ||
        oldDelegate.engine != engine ||
        oldDelegate.progress != progress ||
        oldDelegate.frameIndex != frameIndex ||
        oldDelegate.color != color;
  }
}
