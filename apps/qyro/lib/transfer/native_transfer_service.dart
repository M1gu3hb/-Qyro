// The transfer service, against the real engine.
//
// `QyroSession.stepBlocking` blocks without a bound (ADR-0032 §7), so the whole
// session — open, step to its ending, close — runs inside `Isolate.run`. Only
// three kinds of value cross back: progress triples, a terminal code, and text.
// Nothing that owns a pointer is ever sent, because a `Pointer` is an address in
// one isolate's view and means nothing in another's.

import 'dart:async';
import 'dart:io';
import 'dart:isolate';

import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/ffi/qyro_session_api.dart';
import 'package:qyro/ffi/qyro_trust_api.dart';
import 'package:qyro/transfer/transfer_service.dart';

/// Where received files land.
///
/// ADR-0034 §4: the app's own directory on Android and `Downloads/Qyro` on
/// Windows, and **no storage permission on either**.
String defaultDestination() {
  if (Platform.isWindows) {
    final home = Platform.environment['USERPROFILE'] ?? '.';
    return '$home\\Downloads\\Qyro';
  }
  // Android hands the app its own directory; the Kotlin side passes it in. Until
  // it does, the process working directory is the honest answer rather than a
  // guessed path that would fail at write time.
  return '${Directory.current.path}${Platform.pathSeparator}Qyro';
}

/// The library path this process loads the engine from.
String? _libraryOverride() {
  final value = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
  return (value == null || value.isEmpty) ? null : value;
}

/// One progress sample, small enough to cross an isolate boundary.
final class _Sample {
  const _Sample(this.done, this.total);
  final int done;
  final int total;
}

final class NativeTransferService implements QyroTransferService {
  NativeTransferService({QyroSessionBindings? bindings})
      : _bindings = bindings ?? QyroSessionBindings.openDefault() {
    _trust = QyroTrustBindings.openDefault(_bindings);
  }

  final QyroSessionBindings _bindings;
  late final QyroTrustBindings _trust;

  /// What the peers screen shows. Names and fingerprints only.
  ///
  /// The engine's book is keyed by name and the fingerprint lives with the
  /// identity, so a name with no live session has no fingerprint to show yet;
  /// it is listed with an empty one rather than hidden, because a peer the
  /// device remembers and does not display is a peer nobody can forget.
  @override
  Future<List<QyroPeerEntry>> knownPeers() async => _trust
      .listPeers()
      .map(
        (name) => QyroPeerEntry(
          name: name,
          fingerprint: '',
          trust: QyroPeerTrust.known,
        ),
      )
      .toList(growable: false);

  @override
  Future<bool> forgetPeer(String name) async => _trust.forgetPeer(name);

  @override
  Future<String?> addressOfPairingString(String text) async =>
      _trust.addressOfPairingString(text);

  /// Null until this device has a session to read an address from.
  ///
  /// A pairing string needs an address **and** a fingerprint, and both come from
  /// a live session (ADR-0035 §2). Inventing one before there is a session would
  /// be showing a code that does not work.
  @override
  Future<String?> ownPairingString() async => null;

  @override
  Future<List<QyroPicked>> pickFiles() => pickerForPlatform().pickFiles();

  @override
  Stream<QyroTransferState> send({
    required String address,
    required List<QyroPicked> files,
    String? expectedFingerprint,
  }) async* {
    if (files.isEmpty) {
      yield const QyroFailed(kind: QyroFailureKind.cancelled);
      return;
    }
    yield const QyroConnecting();

    final paths = files.whereType<QyroPickedPath>().map((f) => f.path).toList();
    if (paths.length != files.length) {
      // A descriptor cannot cross an isolate: it is an integer in this
      // process's table and the isolate shares that table, but ownership does
      // not survive being sent. Android's path goes through the same session on
      // this isolate instead, which blocks the frame for the length of the
      // transfer and is the honest cost until phase 07 measures it.
      yield* _sendDescriptors(address, files);
      return;
    }

    final root = _commonRoot(paths);
    final library = _libraryOverride();
    final port = ReceivePort();
    final samples = StreamController<_Sample>();
    port.listen((message) {
      if (message is List && message.length == 2) {
        samples.add(_Sample(message[0] as int, message[1] as int));
      }
    });

    final outcome = Isolate.run<int>(() {
      final bindings = library == null
          ? QyroSessionBindings.openDefault()
          : QyroSessionBindings.open(library);
      final session = QyroSession.send(
        bindings: bindings,
        to: address,
        root: root,
        files: paths,
        onProgress: (progress) =>
            port.sendPort.send(<int>[progress.done, progress.total]),
      );
      try {
        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
        }
        return state == QyroSessionState.completed ? 0 : 1;
      } on QyroSessionFailure catch (failure) {
        return failure.code;
      } finally {
        session.dispose();
      }
    });

    yield* _drain(samples.stream, outcome, address);
    await samples.close();
    port.close();
  }

  /// Android's half: the descriptors belong to this isolate's table.
  Stream<QyroTransferState> _sendDescriptors(
    String address,
    List<QyroPicked> files,
  ) async* {
    final descriptors = files.whereType<QyroPickedDescriptor>().toList();
    yield const QyroConnecting();
    try {
      final session = QyroSession.sendDescriptors(
        bindings: _bindings,
        to: address,
        descriptors: descriptors.map((f) => f.descriptor).toList(),
        names: descriptors.map((f) => f.name).toList(),
      );
      try {
        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
          final progress = session.progress();
          yield QyroMoving(
            done: progress.done,
            total: progress.total,
            fingerprint: _trust.peerFingerprint(session),
          );
          await Future<void>.delayed(Duration.zero);
        }
        if (state == QyroSessionState.completed) {
          yield QyroDelivered(
            fileCount: descriptors.length,
            destination: address,
          );
        } else {
          yield QyroFailed(
            kind: QyroFailureKind.refusedByPeer,
            reason: _trust.rejection(session),
          );
        }
      } finally {
        session.dispose();
      }
    } on QyroSessionFailure catch (failure) {
      yield QyroFailed(kind: _kindOf(failure.code));
    }
  }

  Stream<QyroTransferState> _drain(
    Stream<_Sample> samples,
    Future<int> outcome,
    String address,
  ) async* {
    var last = const _Sample(0, 0);
    final subscription = samples.listen((sample) => last = sample);
    final code = await outcome;
    await subscription.cancel();

    if (code == 0) {
      yield QyroDelivered(fileCount: 1, destination: address);
      return;
    }
    yield QyroMoving(done: last.done, total: last.total, fingerprint: '');
    yield QyroFailed(kind: _kindOf(code));
  }

  @override
  Stream<QyroTransferState> receive({
    required String bind,
    required String destination,
    required Future<bool> Function(QyroAwaitingDecision offer) decide,
  }) async* {
    final where = destination.isEmpty ? defaultDestination() : destination;
    Directory(where).createSync(recursive: true);
    yield const QyroConnecting();

    try {
      final session = QyroSession.receive(
        bindings: _bindings,
        bind: bind,
        destination: where,
      );
      try {
        // One step takes the offer and the manifest; the decision happens
        // before anything else is accepted (ADR-0036 §1).
        session.stepBlocking();
        final progress = session.progress();
        final offer = QyroAwaitingDecision(
          fingerprint: _trust.peerFingerprint(session),
          trust: QyroPeerTrust.newPeer,
          fileNames: const <String>[],
          totalBytes: progress.total,
        );
        yield offer;

        if (!await decide(offer)) {
          _trust.reject(session, QyroRejectReason.declined);
          yield const QyroFailed(kind: QyroFailureKind.refusedByMe);
          return;
        }

        var state = QyroSessionState.inProgress;
        while (state == QyroSessionState.inProgress) {
          state = session.stepBlocking();
          final now = session.progress();
          yield QyroMoving(
            done: now.done,
            total: now.total,
            fingerprint: offer.fingerprint,
          );
          await Future<void>.delayed(Duration.zero);
        }
        if (state == QyroSessionState.completed) {
          yield QyroDelivered(fileCount: 1, destination: where);
        } else {
          yield const QyroFailed(kind: QyroFailureKind.integrity);
        }
      } finally {
        session.dispose();
      }
    } on QyroSessionFailure catch (failure) {
      yield QyroFailed(kind: _kindOf(failure.code));
    }
  }

  /// The local history.
  ///
  /// `qyro_fs::history` records it and no C symbol reads it yet, so this is
  /// empty rather than wrong: an empty list is a true statement about what this
  /// build can show, and a fabricated one would not be.
  @override
  Future<List<QyroHistoryEntry>> history() async => const <QyroHistoryEntry>[];

  static QyroFailureKind _kindOf(int code) => switch (code) {
        QyroCode.peerUnreachable => QyroFailureKind.unreachable,
        QyroCode.notAuthenticated => QyroFailureKind.keyChanged,
        QyroCode.transferRefused => QyroFailureKind.refusedByPeer,
        QyroCode.storageRefused => QyroFailureKind.noRoom,
        QyroCode.cancelled => QyroFailureKind.cancelled,
        _ => QyroFailureKind.integrity,
      };

  /// The deepest directory every path shares.
  ///
  /// The engine names each item relative to a root (ADR-0026), so two files from
  /// different folders must not both become their last component — that would
  /// make the receiver arbitrate a collision the sender created.
  static String _commonRoot(List<String> paths) {
    if (paths.isEmpty) return '.';
    final separator = Platform.pathSeparator;
    var prefix = paths.first.split(separator)..removeLast();
    for (final path in paths.skip(1)) {
      final parts = path.split(separator)..removeLast();
      var shared = 0;
      while (shared < prefix.length &&
          shared < parts.length &&
          prefix[shared] == parts[shared]) {
        shared++;
      }
      prefix = prefix.sublist(0, shared);
    }
    return prefix.isEmpty ? separator : prefix.join(separator);
  }
}
