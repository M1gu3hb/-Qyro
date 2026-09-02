import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../generated/branding.g.dart';
import '../l10n/generated/app_localizations.dart';
import '../startup/startup_coordinator.dart';
import 'ascii_logo_model.dart';
import 'ascii_logo_painter.dart';
import 'boot_sequence_controller.dart';
import 'boot_status_model.dart';
import 'cipher_rain_painter.dart';
import 'scramble_decode_engine.dart';
import 'scrambled_line.dart';

class BootScreen extends StatefulWidget {
  const BootScreen({
    required this.coordinator,
    required this.onFinished,
    this.logoModel,
    super.key,
  });

  final StartupCoordinator coordinator;
  final VoidCallback onFinished;
  final AsciiLogoModel? logoModel;

  @override
  State<BootScreen> createState() => _BootScreenState();
}

class _BootScreenState extends State<BootScreen>
    with SingleTickerProviderStateMixin {
  static const _logoSeed = 0x5159524F;
  static const _wordmarkSeed = 0x5159524D;
  static const _taglineSeed = 0x51595441;
  static const _signatureSeed = 0x51595347;

  late final AnimationController _animation;
  late final BootSequenceController _sequence;

  AsciiLogoModel? _model;
  ScrambleDecodeEngine? _engine;
  var _started = false;
  var _finished = false;
  var _finishScheduled = false;
  var _assetLoadFailed = false;

  @override
  void initState() {
    super.initState();
    _animation = AnimationController(
      vsync: this,
      duration: BootSequenceController.sequenceDuration,
    )..addListener(_handleAnimation);
    _sequence = BootSequenceController()..addListener(_handleSequence);
    widget.coordinator.addListener(_handleStartup);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final reducedMotion = MediaQuery.disableAnimationsOf(context);
    _sequence.updateReducedMotion(reducedMotion);

    if (_started) {
      if (reducedMotion) {
        _animation.stop();
      }
      return;
    }
    _started = true;

    final providedModel = widget.logoModel;
    if (providedModel != null) {
      _installModel(providedModel, reducedMotion: reducedMotion);
    } else {
      _loadGeneratedLogo(reducedMotion: reducedMotion);
    }

    _sequence.updateStartupReady(
      widget.coordinator.snapshot.phase == StartupPhase.ready,
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      unawaited(widget.coordinator.start(reducedMotion: reducedMotion));
      if (!reducedMotion) {
        unawaited(_animation.forward());
      } else {
        _maybeFinish();
      }
    });
  }

  @override
  void dispose() {
    widget.coordinator.removeListener(_handleStartup);
    _sequence
      ..removeListener(_handleSequence)
      ..dispose();
    _animation
      ..removeListener(_handleAnimation)
      ..dispose();
    super.dispose();
  }

  void _loadGeneratedLogo({required bool reducedMotion}) {
    DefaultAssetBundle.of(context)
        .loadString('assets/generated/logo_ascii.json')
        .then((source) {
      if (!mounted) {
        return;
      }
      final model = AsciiLogoModel.fromJsonString(source);
      setState(() {
        _installModel(model, reducedMotion: reducedMotion);
      });
    }, onError: (_) {
      if (mounted) {
        setState(() => _assetLoadFailed = true);
      }
    });
  }

  void _installModel(
    AsciiLogoModel model, {
    required bool reducedMotion,
  }) {
    _model = model;
    _engine = ScrambleDecodeEngine(
      target: model.target,
      width: model.width,
      seed: _logoSeed,
      reducedMotion: reducedMotion,
      revealMode: RevealMode.luminance,
      mask: model.mask,
      luminance: model.density,
      targetAlphas: model.density,
    );
  }

  void _handleAnimation() {
    final elapsed = Duration(
      microseconds: (_animation.value *
              BootSequenceController.sequenceDuration.inMicroseconds)
          .round(),
    );
    _sequence.updateElapsed(elapsed);
  }

  void _handleSequence() {
    if (mounted) {
      setState(() {});
    }
    _maybeFinish();
  }

  void _handleStartup() {
    _sequence.updateStartupReady(
      widget.coordinator.snapshot.phase == StartupPhase.ready,
    );
    if (mounted) {
      setState(() {});
    }
    _maybeFinish();
  }

  void _maybeFinish() {
    if (_finished || _finishScheduled || !_sequence.canFinish) {
      return;
    }
    _finishScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _finished || !_sequence.canFinish) {
        _finishScheduled = false;
        return;
      }
      _finished = true;
      _animation.stop();
      widget.onFinished();
    });
  }

  void _skip() {
    if (_sequence.skip()) {
      _animation.stop();
      _maybeFinish();
    }
  }

  void _retry() {
    unawaited(widget.coordinator.retry());
  }

  KeyEventResult _handleKey(FocusNode node, KeyEvent event) {
    if (!_sequence.canSkip || event is! KeyDownEvent) {
      return KeyEventResult.ignored;
    }

    final key = event.logicalKey;
    if (key == LogicalKeyboardKey.enter ||
        key == LogicalKeyboardKey.space ||
        key == LogicalKeyboardKey.escape) {
      _skip();
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final snapshot = widget.coordinator.snapshot;
    final status = BootStatusModel.fromSnapshot(snapshot);
    final progress = _sequence.visualProgress;
    final logoProgress = BootSequenceController.phaseProgress(
      progress,
      0.08,
      0.82,
    );
    final rainProgress = BootSequenceController.phaseProgress(
      progress,
      0,
      0.78,
    );
    // The wordmark, tagline and signature resolve in sequence so they do not
    // compete with the logo or with each other.
    final wordmarkProgress = BootSequenceController.phaseProgress(
      progress,
      0.30,
      0.72,
    );
    final taglineProgress = BootSequenceController.phaseProgress(
      progress,
      0.52,
      0.88,
    );
    final signatureProgress = BootSequenceController.phaseProgress(
      progress,
      0.66,
      0.95,
    );
    // Advances the noise without advancing the reveal, so unresolved cells keep
    // churning instead of freezing between reveal steps.
    final noiseFrame = (progress * 330).floor();
    final isProvisional =
        snapshot.branding?.isProvisional ?? GeneratedBranding.isProvisional;
    // When startup has failed, the diagnostic and the retry control matter more
    // than the decorative lines. Dropping them also keeps retry reachable on a
    // short viewport instead of pushing it below the fold.
    final hasFailed = status.isTerminalFailure || _assetLoadFailed;
    final signature =
        hasFailed ? null : _signatureFor(strings, isProvisional: isProvisional);

    return Focus(
      autofocus: true,
      onKeyEvent: _handleKey,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        // **QYR-0397: saltar es saltar la animación, y tras un fallo no hay
        // adónde saltar.** `_maybeFinish` exige `canFinish`, que exige
        // `_startupReady`, que es falso justo aquí. Así que la pantalla entera
        // aceptaba el toque, `_skip()` ponía el progreso visual a 1, y **no
        // pasaba nada más** — ni navegación, ni mensaje.
        //
        // Un control encendido que no hace nada enseña que la aplicación se
        // colgó. Apagado, al lado de REINTENTAR encendido, dice cuál es el que
        // está vivo.
        onTap: _sequence.canSkip && !hasFailed ? _skip : null,
        child: Scaffold(
          backgroundColor: const Color(
            GeneratedBranding.backgroundColorValue,
          ),
          body: SafeArea(
            child: Stack(
              children: [
                const Positioned.fill(child: _TerminalBackground()),
                Positioned.fill(
                  child: RepaintBoundary(
                    child: CustomPaint(
                      painter: CipherRainPainter(
                        progress: rainProgress,
                        seed: _logoSeed,
                        reducedMotion: _sequence.reducedMotion,
                      ),
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(20, 16, 20, 12),
                  child: Column(
                    children: [
                      if (isProvisional)
                        _ProvisionalBanner(
                          label: strings.bootProvisionalBranding,
                        )
                      else
                        const SizedBox(height: 24),
                      Expanded(
                        child: SingleChildScrollView(
                          child: ConstrainedBox(
                            constraints: const BoxConstraints(maxWidth: 680),
                            child: Column(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                _buildLogo(
                                  logoProgress,
                                  strings.bootLogoSemantics,
                                ),
                                const SizedBox(height: 22),
                                ScrambledLine(
                                  key: const Key('boot-wordmark'),
                                  text: GeneratedBranding.appName.toUpperCase(),
                                  progress: wordmarkProgress,
                                  frameIndex: noiseFrame,
                                  seed: _wordmarkSeed,
                                  reducedMotion: _sequence.reducedMotion,
                                  style: const TextStyle(
                                    color: Color(
                                      GeneratedBranding.secondaryColorValue,
                                    ),
                                    fontFamily: 'monospace',
                                    fontSize: 28,
                                    fontWeight: FontWeight.w700,
                                    letterSpacing: 10,
                                  ),
                                ),
                                if (!hasFailed) ...[
                                  const SizedBox(height: 8),
                                  ScrambledLine(
                                    key: const Key('boot-tagline'),
                                    text: strings.bootTagline,
                                    progress: taglineProgress,
                                    frameIndex: noiseFrame,
                                    seed: _taglineSeed,
                                    reducedMotion: _sequence.reducedMotion,
                                    style: const TextStyle(
                                      color: Color(0xFF7FA8C8),
                                      fontFamily: 'monospace',
                                      fontSize: 11,
                                      letterSpacing: 3,
                                    ),
                                  ),
                                ],
                                if (signature != null) ...[
                                  const SizedBox(height: 10),
                                  ScrambledLine(
                                    key: const Key('boot-signature'),
                                    text: signature,
                                    progress: signatureProgress,
                                    frameIndex: noiseFrame,
                                    seed: _signatureSeed,
                                    reducedMotion: _sequence.reducedMotion,
                                    style: const TextStyle(
                                      color: Color(0xFF5F7F9B),
                                      fontFamily: 'monospace',
                                      fontSize: 10,
                                      letterSpacing: 2,
                                    ),
                                  ),
                                ],
                                const SizedBox(height: 12),
                                _BootStatus(
                                  status: status,
                                  protocolVersion: snapshot.protocolVersion,
                                  assetLoadFailed: _assetLoadFailed,
                                  strings: strings,
                                  onRetry: _retry,
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      Align(
                        alignment: Alignment.centerRight,
                        child: TextButton(
                          key: const Key('boot-skip'),
                          // QYR-0397. Ver el comentario de `onTap` arriba.
                          onPressed:
                              _sequence.canSkip && !hasFailed ? _skip : null,
                          child: Text(strings.bootSkip),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Resolves the signature line, or null when nothing may be shown.
  ///
  /// A configured, non-provisional signature is displayed as authored. When
  /// branding is still provisional the creator's name is unknown, so nothing is
  /// invented: debug builds show a clearly marked "not configured" notice to
  /// make the gap visible, and release builds show no line at all.
  static String? _signatureFor(
    AppLocalizations strings, {
    required bool isProvisional,
  }) {
    final configured = GeneratedBranding.signatureText.trim();
    if (configured.isNotEmpty && !isProvisional) {
      return configured;
    }
    if (kDebugMode) {
      return strings.bootSignatureUnset;
    }
    return null;
  }

  Widget _buildLogo(double progress, String semanticLabel) {
    final model = _model;
    final engine = _engine;
    if (model == null || engine == null) {
      return const SizedBox(
        height: 220,
        child: Center(
          child: SizedBox.square(
            dimension: 28,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        ),
      );
    }

    final alpha = (0.35 + progress * 0.65).clamp(0.0, 1.0).toDouble();
    final color = const Color(
      GeneratedBranding.secondaryColorValue,
    ).withValues(alpha: alpha);

    return Semantics(
      container: true,
      image: true,
      label: semanticLabel,
      child: ExcludeSemantics(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final width = math.min(constraints.maxWidth * 0.92, 620.0);
            final height = math.min(width / model.aspectRatio, 300.0);
            return RepaintBoundary(
              child: CustomPaint(
                size: Size(width, height),
                painter: AsciiLogoPainter(
                  model: model,
                  engine: engine,
                  progress: progress,
                  frameIndex: (_sequence.visualProgress * 330).floor(),
                  color: color,
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}

class _BootStatus extends StatelessWidget {
  const _BootStatus({
    required this.status,
    required this.protocolVersion,
    required this.assetLoadFailed,
    required this.strings,
    required this.onRetry,
  });

  final BootStatusModel status;
  final String? protocolVersion;
  final bool assetLoadFailed;
  final AppLocalizations strings;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    final diagnosticCode =
        status.diagnosticCode ?? (assetLoadFailed ? 'asset_invalid' : null);
    final technicalSummary = _diagnosticDetail(strings, diagnosticCode);

    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 480),
      child: Column(
        children: [
          Text(
            _messageFor(strings, status.messageKey),
            textAlign: TextAlign.center,
            style: const TextStyle(
              color: Color(0xFFAFC5D9),
              fontFamily: 'monospace',
              fontSize: 12,
              letterSpacing: 1.2,
            ),
          ),
          const SizedBox(height: 10),
          LinearProgressIndicator(
            value: status.phase == StartupPhase.ready ? 1 : status.progress,
            minHeight: 2,
            backgroundColor: const Color(0xFF10263D),
          ),
          if (protocolVersion != null) ...[
            const SizedBox(height: 8),
            Text(
              protocolVersion!,
              style: const TextStyle(
                color: Color(0xFF6F90AC),
                fontFamily: 'monospace',
                fontSize: 11,
              ),
            ),
          ],
          if (diagnosticCode != null) ...[
            const SizedBox(height: 12),
            Text(
              strings.bootDiagnostic(diagnosticCode),
              textAlign: TextAlign.center,
              style: const TextStyle(
                color: Color(0xFFFFC86B),
                fontFamily: 'monospace',
                fontSize: 11,
              ),
            ),
            if (technicalSummary != null)
              Text(
                technicalSummary,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: Color(0xFF92A9BD),
                  fontFamily: 'monospace',
                  fontSize: 10,
                ),
              ),
          ],
          if (status.isTerminalFailure || assetLoadFailed) ...[
            const SizedBox(height: 14),
            FilledButton.tonalIcon(
              key: const Key('boot-retry'),
              onPressed: onRetry,
              icon: const Icon(Icons.refresh),
              label: Text(strings.bootRetry),
            ),
          ],
        ],
      ),
    );
  }

  static String _messageFor(AppLocalizations strings, String key) {
    return switch (key) {
      'startupIdle' => strings.bootStatusIdle,
      'startupPreparing' => strings.bootStatusPreparing,
      'startupBranding' => strings.bootStatusBranding,
      'startupAssets' => strings.bootStatusAssets,
      'startupNativeBridge' => strings.bootStatusNativeBridge,
      'startupInterface' => strings.bootStatusInterface,
      'startupReady' => strings.bootStatusReady,
      'startupTimeout' => strings.bootStatusTimeout,
      'startupCancelled' => strings.bootStatusCancelled,
      'nativeBridgeUnavailable' => strings.bootStatusNativeUnavailable,
      'startupAssetInvalid' => strings.bootStatusAssetInvalid,
      'startupInterfaceUnavailable' => strings.bootStatusInterfaceUnavailable,
      _ => strings.bootStatusFailed,
    };
  }

  static String? _diagnosticDetail(
    AppLocalizations strings,
    String? code,
  ) {
    return switch (code) {
      null => null,
      'library_not_found' => strings.diagnosticLibraryNotFound,
      'symbol_not_found' => strings.diagnosticSymbolNotFound,
      'null_pointer' => strings.diagnosticNullPointer,
      'invalid_length' => strings.diagnosticInvalidLength,
      'invalid_utf8' => strings.diagnosticInvalidUtf8,
      'incompatible_version' => strings.diagnosticIncompatibleVersion,
      'startup_timeout' => strings.diagnosticStartupTimeout,
      'startup_failed' => strings.diagnosticStartupFailed,
      'interface_unavailable' => strings.diagnosticInterfaceUnavailable,
      'asset_invalid' => strings.diagnosticAssetInvalid,
      _ => strings.diagnosticUnknown,
    };
  }
}

class _ProvisionalBanner extends StatelessWidget {
  const _ProvisionalBanner({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: Color(0x26168BFF),
        border: Border.fromBorderSide(
          BorderSide(color: Color(0x66168BFF)),
        ),
        borderRadius: BorderRadius.all(Radius.circular(8)),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        child: Text(
          label,
          textAlign: TextAlign.center,
          style: const TextStyle(
            color: Color(0xFF9FCFFF),
            fontFamily: 'monospace',
            fontSize: 10,
            letterSpacing: 1,
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
          radius: 1.18,
          colors: [
            Color(0xFF082D59),
            Color(GeneratedBranding.backgroundColorValue),
          ],
        ),
      ),
    );
  }
}
