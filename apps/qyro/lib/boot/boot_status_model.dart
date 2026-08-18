import '../startup/startup_coordinator.dart';

final class BootStatusModel {
  const BootStatusModel._({
    required this.phase,
    required this.progress,
    required this.messageKey,
    required this.isTerminalFailure,
    this.diagnosticCode,
    this.technicalSummary,
  });

  factory BootStatusModel.fromSnapshot(StartupSnapshot snapshot) {
    final diagnostic = snapshot.diagnostic;
    final progress = snapshot.completedTasks.length / StartupTask.values.length;
    final messageKey = switch (snapshot.phase) {
      StartupPhase.idle => 'startupIdle',
      StartupPhase.running => _taskMessageKey(snapshot.currentTask),
      StartupPhase.ready => 'startupReady',
      StartupPhase.failed => diagnostic?.userMessageKey ?? 'startupFailed',
      StartupPhase.timedOut => diagnostic?.userMessageKey ?? 'startupTimeout',
      StartupPhase.cancelled => 'startupCancelled',
    };

    return BootStatusModel._(
      phase: snapshot.phase,
      progress: progress.clamp(0.0, 1.0).toDouble(),
      messageKey: messageKey,
      isTerminalFailure: snapshot.phase == StartupPhase.failed ||
          snapshot.phase == StartupPhase.timedOut,
      diagnosticCode: diagnostic?.code,
      technicalSummary: diagnostic?.technicalSummary,
    );
  }

  final StartupPhase phase;
  final double progress;
  final String messageKey;
  final bool isTerminalFailure;
  final String? diagnosticCode;
  final String? technicalSummary;

  static String _taskMessageKey(StartupTask? task) {
    return switch (task) {
      StartupTask.branding => 'startupBranding',
      StartupTask.assets => 'startupAssets',
      StartupTask.nativeBridge => 'startupNativeBridge',
      StartupTask.interface => 'startupInterface',
      null => 'startupPreparing',
    };
  }
}
