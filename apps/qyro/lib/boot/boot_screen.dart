import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'scramble_decode_engine.dart';

class BootScreen extends StatefulWidget {
  const BootScreen({required this.onFinished, super.key});

  final VoidCallback onFinished;

  @override
  State<BootScreen> createState() => _BootScreenState();
}

class _BootScreenState extends State<BootScreen>
    with SingleTickerProviderStateMixin {
  static const _duration = Duration(milliseconds: 5500);
  static const _skipGuard = Duration(seconds: 1);

  late final AnimationController _controller;
  late final ScrambleDecodeEngine _engine;
  Timer? _skipTimer;
  var _canSkip = false;
  var _finished = false;
  var _reducedMotionScheduled = false;

  @override
  void initState() {
    super.initState();
    _engine = ScrambleDecodeEngine(target: 'QYRO', seed: 0x5159524F);
    _controller = AnimationController(vsync: this, duration: _duration)
      ..addStatusListener((status) {
        if (status == AnimationStatus.completed) {
          _finish();
        }
      })
      ..forward();

    _skipTimer = Timer(_skipGuard, () {
      if (mounted && !_finished) {
        setState(() => _canSkip = true);
      }
    });
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (!_reducedMotionScheduled && MediaQuery.disableAnimationsOf(context)) {
      _reducedMotionScheduled = true;
      _controller.stop();
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _finish();
        }
      });
    }
  }

  @override
  void dispose() {
    _skipTimer?.cancel();
    _controller.dispose();
    super.dispose();
  }

  void _finish() {
    if (_finished) {
      return;
    }
    _finished = true;
    _skipTimer?.cancel();
    widget.onFinished();
  }

  KeyEventResult _handleKey(FocusNode node, KeyEvent event) {
    if (!_canSkip || event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }

    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.enter ||
        key == LogicalKeyboardKey.space ||
        key == LogicalKeyboardKey.escape) {
      _finish();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    return Focus(
      autofocus: true,
      onKeyEvent: _handleKey,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: _canSkip ? _finish : null,
        child: Scaffold(
          body: SafeArea(
            child: AnimatedBuilder(
              animation: _controller,
              builder: (context, child) {
                final progress = Curves.easeOutCubic.transform(
                  _controller.value,
                );
                return Stack(
                  children: [
                    const Positioned.fill(child: _TerminalBackground()),
                    Center(
                      child: RepaintBoundary(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Image.asset(
                              'assets/brand/qyro-logo.png',
                              width: 156,
                              height: 156,
                              semanticLabel: 'Logo de Qyro',
                            ),
                            const SizedBox(height: 24),
                            Semantics(
                              label: 'Qyro',
                              child: Text(
                                _engine.frameAt(progress),
                                style: const TextStyle(
                                  color: Color(0xFF51C8FF),
                                  fontFamily: 'monospace',
                                  fontSize: 34,
                                  fontWeight: FontWeight.w700,
                                  letterSpacing: 8,
                                ),
                              ),
                            ),
                            const SizedBox(height: 16),
                            const Text(
                              'INICIALIZANDO INTERFAZ LOCAL',
                              style: TextStyle(
                                color: Color(0xFF9BB8D3),
                                fontFamily: 'monospace',
                                fontSize: 12,
                                letterSpacing: 1.4,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                    Positioned(
                      right: 12,
                      bottom: 8,
                      child: TextButton(
                        onPressed: _canSkip ? _finish : null,
                        child: const Text('OMITIR'),
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ),
    );
  }
}

class _TerminalBackground extends StatelessWidget {
  const _TerminalBackground();

  @override
  Widget build(BuildContext context) {
    return const DecoratedBox(
      decoration: BoxDecoration(
        gradient: RadialGradient(
          radius: 1.15,
          colors: [
            Color(0xFF082D59),
            Color(0xFF03070D),
          ],
        ),
      ),
    );
  }
}
