import 'dart:math' as math;

import 'package:flutter/material.dart';

final class CipherRainPainter extends CustomPainter {
  const CipherRainPainter({
    required this.progress,
    required this.seed,
    this.rainColor = const Color(0xFF168BFF),
    this.reducedMotion = false,
  });

  final double progress;
  final int seed;
  final Color rainColor;
  final bool reducedMotion;

  @override
  void paint(Canvas canvas, Size size) {
    if (size.isEmpty || reducedMotion) {
      return;
    }

    final normalized = progress.clamp(0.0, 1.0).toDouble();
    final fade = (1 - normalized).clamp(0.15, 1.0).toDouble();
    final columns = math.max(12, (size.width / 18).floor());
    final columnWidth = size.width / columns;
    final paint = Paint()
      ..strokeWidth = 1.2
      ..strokeCap = StrokeCap.round;

    for (var column = 0; column < columns; column++) {
      final offset = _unit(seed ^ (column * 0x45D9F3B));
      final speed = 0.55 + _unit(seed ^ (column * 0x27D4EB2D)) * 0.9;
      final head =
          ((offset + normalized * speed) % 1.0) * (size.height + 80) - 40;
      final x = (column + 0.5) * columnWidth;

      for (var trail = 0; trail < 7; trail++) {
        var y = head - trail * 16;
        while (y < -8) {
          y += size.height + 80;
        }
        final alpha = fade * (1 - trail / 7) * 0.34;
        paint.color = rainColor.withValues(alpha: alpha);
        final length = 3 + _unit(seed ^ column ^ (trail * 8191)) * 8;
        canvas.drawLine(Offset(x, y), Offset(x, y + length), paint);
      }
    }
  }

  @override
  bool shouldRepaint(covariant CipherRainPainter oldDelegate) {
    return oldDelegate.progress != progress ||
        oldDelegate.seed != seed ||
        oldDelegate.rainColor != rainColor ||
        oldDelegate.reducedMotion != reducedMotion;
  }

  static double _unit(int value) {
    var mixed = value & 0x7FFFFFFF;
    mixed = ((mixed ^ (mixed >> 16)) * 0x45D9F3B) & 0x7FFFFFFF;
    mixed = ((mixed ^ (mixed >> 16)) * 0x45D9F3B) & 0x7FFFFFFF;
    return ((mixed ^ (mixed >> 16)) & 0xFFFF) / 0xFFFF;
  }
}
