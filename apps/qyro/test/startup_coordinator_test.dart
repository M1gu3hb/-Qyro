import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_native_api.dart';
import 'package:qyro/startup/native_bridge.dart';
import 'package:qyro/startup/startup_coordinator.dart';

void main() {
  test('runs real startup tasks in order and reports ready', () async {
    final calls = <String>[];
    final coordinator = StartupCoordinator(
      loadBranding: () async {
        calls.add('branding');
        return const StartupBranding(isProvisional: false);
      },
      verifyAssets: () async => calls.add('assets'),
      loadNativeBridge: () async {
        calls.add('native');
        return const _FakeBridge();
      },
      initializeInterface: () async => calls.add('interface'),
    );

    await coordinator.start(reducedMotion: true);

    expect(calls, <String>['branding', 'assets', 'native', 'interface']);
    expect(coordinator.snapshot.phase, StartupPhase.ready);
    expect(coordinator.snapshot.protocolVersion, 'QYRO/1');
    expect(coordinator.snapshot.reducedMotion, isTrue);
    expect(coordinator.snapshot.completedTasks, StartupTask.values.toSet());
  });

  test('does not report ready before an obligatory task finishes', () async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(
      loadNativeBridge: () => native.future,
    );

    final startup = coordinator.start();
    await Future<void>.delayed(Duration.zero);

    expect(coordinator.snapshot.phase, StartupPhase.running);
    expect(coordinator.snapshot.currentTask, StartupTask.nativeBridge);

    native.complete(const _FakeBridge());
    await startup;

    expect(coordinator.snapshot.phase, StartupPhase.ready);
  });

  test('preserves typed native diagnostics without raw UI errors', () async {
    const failure = QyroInvalidUtf8Failure();
    final coordinator = _coordinator(
      loadNativeBridge: () async => const _FakeBridge(failure: failure),
    );

    await coordinator.start();

    expect(coordinator.snapshot.phase, StartupPhase.failed);
    expect(coordinator.snapshot.diagnostic?.code, failure.code);
    expect(
      coordinator.snapshot.diagnostic?.userMessageKey,
      failure.userMessageKey,
    );
  });

  test('times out a pending task and supports retry', () async {
    var attempts = 0;
    final firstAttempt = Completer<void>();
    final coordinator = _coordinator(
      timeout: const Duration(milliseconds: 20),
      verifyAssets: () async {
        attempts += 1;
        if (attempts == 1) {
          await firstAttempt.future;
        }
      },
    );

    await coordinator.start();

    expect(coordinator.snapshot.phase, StartupPhase.timedOut);
    expect(coordinator.snapshot.currentTask, StartupTask.assets);

    await coordinator.retry();

    expect(attempts, 2);
    expect(coordinator.snapshot.phase, StartupPhase.ready);
  });

  test('cancellation ignores late task completion', () async {
    final native = Completer<NativeBridge>();
    final coordinator = _coordinator(
      loadNativeBridge: () => native.future,
    );

    final startup = coordinator.start();
    await Future<void>.delayed(Duration.zero);
    coordinator.cancel();
    native.complete(const _FakeBridge());
    await startup;

    expect(coordinator.snapshot.phase, StartupPhase.cancelled);
  });

  test('resuming the app does not repeat completed startup', () async {
    var nativeLoads = 0;
    final coordinator = _coordinator(
      loadNativeBridge: () async {
        nativeLoads += 1;
        return const _FakeBridge();
      },
    );

    await coordinator.start();
    coordinator.handleAppLifecycleState(AppLifecycleState.paused);
    coordinator.handleAppLifecycleState(AppLifecycleState.resumed);

    expect(nativeLoads, 1);
    expect(coordinator.snapshot.phase, StartupPhase.ready);
    expect(coordinator.snapshot.lifecycleState, AppLifecycleState.resumed);
  });
}

StartupCoordinator _coordinator({
  Future<void> Function()? verifyAssets,
  Future<NativeBridge> Function()? loadNativeBridge,
  Duration timeout = const Duration(seconds: 1),
}) {
  return StartupCoordinator(
    loadBranding: () async =>
        const StartupBranding(isProvisional: true),
    verifyAssets: verifyAssets ?? () async {},
    loadNativeBridge:
        loadNativeBridge ?? () async => const _FakeBridge(),
    initializeInterface: () async {},
    timeout: timeout,
  );
}

final class _FakeBridge implements NativeBridge {
  const _FakeBridge({
    this.version = 'QYRO/1',
    this.failure,
  });

  final String version;
  final QyroNativeFailure? failure;

  @override
  String protocolVersion() {
    final currentFailure = failure;
    if (currentFailure != null) {
      throw currentFailure;
    }
    return version;
  }
}
