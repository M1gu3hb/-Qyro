import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../ffi/qyro_native_api.dart';
import 'native_bridge.dart';

enum StartupTask {
  branding,
  assets,
  nativeBridge,
  interface,
}

enum StartupPhase {
  idle,
  running,
  ready,
  failed,
  timedOut,
  cancelled,
}

final class StartupBranding {
  const StartupBranding({required this.isProvisional});

  final bool isProvisional;
}

final class StartupDiagnostic {
  const StartupDiagnostic({
    required this.code,
    required this.userMessageKey,
    required this.technicalSummary,
  });

  final String code;
  final String userMessageKey;
  final String technicalSummary;
}

final class StartupTaskFailure implements Exception {
  const StartupTaskFailure({
    required this.code,
    required this.userMessageKey,
    required this.technicalSummary,
  });

  final String code;
  final String userMessageKey;
  final String technicalSummary;
}

final class StartupSnapshot {
  StartupSnapshot({
    required this.phase,
    required this.completedTasks,
    required this.reducedMotion,
    required this.lifecycleState,
    this.currentTask,
    this.protocolVersion,
    this.branding,
    this.diagnostic,
  }) : completedTasks = Set<StartupTask>.unmodifiable(completedTasks);

  factory StartupSnapshot.initial() {
    return StartupSnapshot(
      phase: StartupPhase.idle,
      completedTasks: const <StartupTask>{},
      reducedMotion: false,
      lifecycleState: AppLifecycleState.resumed,
    );
  }

  final StartupPhase phase;
  final StartupTask? currentTask;
  final Set<StartupTask> completedTasks;
  final String? protocolVersion;
  final StartupBranding? branding;
  final StartupDiagnostic? diagnostic;
  final bool reducedMotion;
  final AppLifecycleState lifecycleState;
}

typedef StartupBrandingLoader = Future<StartupBranding> Function();
typedef StartupAssetVerifier = Future<void> Function();
typedef StartupNativeBridgeLoader = Future<NativeBridge> Function();
typedef StartupInterfaceInitializer = Future<void> Function();

final class StartupCoordinator extends ChangeNotifier {
  StartupCoordinator({
    required StartupBrandingLoader loadBranding,
    required StartupAssetVerifier verifyAssets,
    required StartupNativeBridgeLoader loadNativeBridge,
    required StartupInterfaceInitializer initializeInterface,
    this.timeout = const Duration(seconds: 8),
  })  : _loadBranding = loadBranding,
        _verifyAssets = verifyAssets,
        _loadNativeBridge = loadNativeBridge,
        _initializeInterface = initializeInterface;

  final StartupBrandingLoader _loadBranding;
  final StartupAssetVerifier _verifyAssets;
  final StartupNativeBridgeLoader _loadNativeBridge;
  final StartupInterfaceInitializer _initializeInterface;
  final Duration timeout;

  StartupSnapshot _snapshot = StartupSnapshot.initial();
  var _generation = 0;
  var _disposed = false;

  StartupSnapshot get snapshot => _snapshot;

  Future<void> start({bool reducedMotion = false}) async {
    if (_snapshot.phase == StartupPhase.running ||
        _snapshot.phase == StartupPhase.ready) {
      return;
    }

    final generation = ++_generation;
    _replace(
      phase: StartupPhase.running,
      currentTask: null,
      completedTasks: const <StartupTask>{},
      protocolVersion: null,
      branding: null,
      diagnostic: null,
      reducedMotion: reducedMotion,
    );

    try {
      await _run(generation).timeout(timeout);
    } on TimeoutException {
      if (_isCurrent(generation)) {
        _generation += 1;
        _replace(
          phase: StartupPhase.timedOut,
          diagnostic: const StartupDiagnostic(
            code: 'startup_timeout',
            userMessageKey: 'startupTimeout',
            technicalSummary: 'An obligatory startup task exceeded its timeout',
          ),
        );
      }
    } on _StartupCancelled {
      // Cancellation already published its terminal snapshot.
    } on QyroNativeFailure catch (failure) {
      if (_isCurrent(generation)) {
        _replace(
          phase: StartupPhase.failed,
          diagnostic: StartupDiagnostic(
            code: failure.code,
            userMessageKey: failure.userMessageKey,
            technicalSummary: failure.diagnostic,
          ),
        );
      }
    } on StartupTaskFailure catch (failure) {
      if (_isCurrent(generation)) {
        _replace(
          phase: StartupPhase.failed,
          diagnostic: StartupDiagnostic(
            code: failure.code,
            userMessageKey: failure.userMessageKey,
            technicalSummary: failure.technicalSummary,
          ),
        );
      }
    } catch (_) {
      if (_isCurrent(generation)) {
        _replace(
          phase: StartupPhase.failed,
          diagnostic: const StartupDiagnostic(
            code: 'startup_failed',
            userMessageKey: 'startupFailed',
            technicalSummary: 'An unexpected startup task failure occurred',
          ),
        );
      }
    }
  }

  Future<void> retry() {
    return start(reducedMotion: _snapshot.reducedMotion);
  }

  void cancel() {
    if (_snapshot.phase != StartupPhase.running) {
      return;
    }
    _generation += 1;
    _replace(
      phase: StartupPhase.cancelled,
      currentTask: null,
      diagnostic: null,
    );
  }

  void handleAppLifecycleState(AppLifecycleState state) {
    _replace(lifecycleState: state);
  }

  Future<void> _run(int generation) async {
    final branding = await _perform(
      generation,
      StartupTask.branding,
      _loadBranding,
    );
    _ensureCurrent(generation);
    _replace(branding: branding);

    await _perform(generation, StartupTask.assets, _verifyAssets);
    final bridge = await _perform(
      generation,
      StartupTask.nativeBridge,
      _loadNativeBridge,
    );
    final version = bridge.protocolVersion();
    if (version != QyroNativeApi.supportedProtocolVersion) {
      throw QyroIncompatibleVersionFailure(actual: version);
    }
    _replace(protocolVersion: version);

    await _perform(
      generation,
      StartupTask.interface,
      _initializeInterface,
    );
    _ensureCurrent(generation);
    _replace(
      phase: StartupPhase.ready,
      currentTask: null,
      diagnostic: null,
    );
  }

  Future<T> _perform<T>(
    int generation,
    StartupTask task,
    Future<T> Function() operation,
  ) async {
    _ensureCurrent(generation);
    _replace(currentTask: task);
    final value = await operation();
    _ensureCurrent(generation);
    final completed = Set<StartupTask>.of(_snapshot.completedTasks)..add(task);
    _replace(completedTasks: completed);
    return value;
  }

  void _ensureCurrent(int generation) {
    if (!_isCurrent(generation)) {
      throw const _StartupCancelled();
    }
  }

  bool _isCurrent(int generation) {
    return !_disposed && generation == _generation;
  }

  void _replace({
    StartupPhase? phase,
    Object? currentTask = _notProvided,
    Set<StartupTask>? completedTasks,
    Object? protocolVersion = _notProvided,
    Object? branding = _notProvided,
    Object? diagnostic = _notProvided,
    bool? reducedMotion,
    AppLifecycleState? lifecycleState,
  }) {
    if (_disposed) {
      return;
    }
    _snapshot = StartupSnapshot(
      phase: phase ?? _snapshot.phase,
      currentTask: identical(currentTask, _notProvided)
          ? _snapshot.currentTask
          : currentTask as StartupTask?,
      completedTasks: completedTasks ?? _snapshot.completedTasks,
      protocolVersion: identical(protocolVersion, _notProvided)
          ? _snapshot.protocolVersion
          : protocolVersion as String?,
      branding: identical(branding, _notProvided)
          ? _snapshot.branding
          : branding as StartupBranding?,
      diagnostic: identical(diagnostic, _notProvided)
          ? _snapshot.diagnostic
          : diagnostic as StartupDiagnostic?,
      reducedMotion: reducedMotion ?? _snapshot.reducedMotion,
      lifecycleState: lifecycleState ?? _snapshot.lifecycleState,
    );
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    _generation += 1;
    super.dispose();
  }

  static const _notProvided = Object();
}

final class _StartupCancelled implements Exception {
  const _StartupCancelled();
}
