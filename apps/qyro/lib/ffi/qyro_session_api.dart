// The Dart half of the engine boundary.
//
// Specification: docs/adr/ADR-0032-engine-ffi.md (the six operations),
// docs/adr/ADR-0033-progress-bridge.md (the callback), and
// docs/adr/ADR-0038-input-buffers.md (who owns the bytes that cross).
//
// The library is opened with the pattern that already works on three platforms
// -- QYRO_FFI_LIBRARY_PATH, then the per-platform file name, then the process on
// iOS -- reused from QyroNativeApi rather than reinvented.

import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'qyro_native_api.dart';

/// The return codes of `rust/crates/qyro_ffi/src/abi.rs`.
///
/// Transcribed rather than derived: there is no shared header, so the one thing
/// that keeps these honest is a test that asserts the two lists agree.
abstract final class QyroCode {
  static const ok = 0;
  static const invalidHandle = -1;
  static const tableFull = -2;
  static const panic = -3;
  static const nullOut = -4;
  static const poisoned = -5;
  static const badArgument = -6;
  static const peerUnreachable = -7;
  static const notAuthenticated = -8;
  static const transferRefused = -9;
  static const storageRefused = -10;
  static const cancelled = -11;
  static const unknown = -12;

  static const names = <int, String>{
    ok: 'ok',
    invalidHandle: 'invalid_handle',
    tableFull: 'table_full',
    panic: 'panic',
    nullOut: 'null_out',
    poisoned: 'poisoned',
    badArgument: 'bad_argument',
    peerUnreachable: 'peer_unreachable',
    notAuthenticated: 'not_authenticated',
    transferRefused: 'transfer_refused',
    storageRefused: 'storage_refused',
    cancelled: 'cancelled',
    unknown: 'unknown',
  };
}

/// Where a session is. The three states of ADR-0032 §5 -- an error is the
/// return code, never a state.
enum QyroSessionState { inProgress, completed, rejected }

/// How far a session has got. Three integers, nothing to free.
final class QyroProgress {
  const QyroProgress({
    required this.done,
    required this.total,
    required this.item,
  });

  final int done;
  final int total;
  final int item;

  @override
  String toString() => 'QyroProgress($done/$total, item $item)';
}

final class QyroSessionFailure implements Exception {
  const QyroSessionFailure(this.code, this.operation);

  final int code;
  final String operation;

  String get name => QyroCode.names[code] ?? 'code_$code';

  @override
  String toString() => 'QyroSessionFailure($operation: $name / $code)';
}

typedef QyroProgressCallback = void Function(QyroProgress progress);

/// The byte that separates paths in the buffer the sender receives.
const _pathSeparator = '\x00';

// --------------------------------------------------------------- native types

typedef _ProgressNative = Void Function(
    UintPtr context, Uint64 done, Uint64 total, Uint32 item);

typedef _OpenSenderNative = Int32 Function(
  Pointer<Uint8> address,
  UintPtr addressLen,
  Pointer<Uint8> root,
  UintPtr rootLen,
  Pointer<Uint8> paths,
  UintPtr pathsLen,
  Pointer<NativeFunction<_ProgressNative>> onProgress,
  UintPtr context,
  Pointer<Uint64> outHandle,
);
typedef _OpenSenderDart = int Function(
  Pointer<Uint8>,
  int,
  Pointer<Uint8>,
  int,
  Pointer<Uint8>,
  int,
  Pointer<NativeFunction<_ProgressNative>>,
  int,
  Pointer<Uint64>,
);

typedef _OpenReceiverNative = Int32 Function(
  Pointer<Uint8> bind,
  UintPtr bindLen,
  Pointer<Uint8> destination,
  UintPtr destinationLen,
  Pointer<NativeFunction<_ProgressNative>> onProgress,
  UintPtr context,
  Pointer<Uint64> outHandle,
);
typedef _OpenReceiverDart = int Function(
  Pointer<Uint8>,
  int,
  Pointer<Uint8>,
  int,
  Pointer<NativeFunction<_ProgressNative>>,
  int,
  Pointer<Uint64>,
);

typedef _StepNative = Int32 Function(Uint64 handle, Pointer<Int32> outState);
typedef _StepDart = int Function(int, Pointer<Int32>);

typedef _ProgressQueryNative = Int32 Function(
  Uint64 handle,
  Pointer<Uint64> outDone,
  Pointer<Uint64> outTotal,
  Pointer<Uint32> outItem,
);
typedef _ProgressQueryDart = int Function(
    int, Pointer<Uint64>, Pointer<Uint64>, Pointer<Uint32>);

typedef _HandleOnlyNative = Int32 Function(Uint64 handle);
typedef _HandleOnlyDart = int Function(int);

typedef _AllocNative = Pointer<Uint8> Function(UintPtr len);
typedef _AllocDart = Pointer<Uint8> Function(int);

typedef _FreeNative = Void Function(Pointer<Uint8> ptr, UintPtr len);
typedef _FreeDart = void Function(Pointer<Uint8>, int);

// ------------------------------------------------------------------- bindings

/// The ten symbols of the boundary, looked up once.
final class QyroSessionBindings {
  QyroSessionBindings._(DynamicLibrary library)
      : _openSender =
            library.lookupFunction<_OpenSenderNative, _OpenSenderDart>(
          'qyro_session_open_sender_blocking',
        ),
        _openReceiver =
            library.lookupFunction<_OpenReceiverNative, _OpenReceiverDart>(
          'qyro_session_open_receiver_blocking',
        ),
        _step = library.lookupFunction<_StepNative, _StepDart>(
          'qyro_session_step_blocking',
        ),
        _readProgress =
            library.lookupFunction<_ProgressQueryNative, _ProgressQueryDart>(
          'qyro_session_progress',
        ),
        _cancel = library.lookupFunction<_HandleOnlyNative, _HandleOnlyDart>(
          'qyro_session_cancel',
        ),
        _close = library.lookupFunction<_HandleOnlyNative, _HandleOnlyDart>(
          'qyro_session_close',
        ),
        _alloc = library.lookupFunction<_AllocNative, _AllocDart>(
          'qyro_buffer_alloc',
        ),
        _free = library.lookupFunction<_FreeNative, _FreeDart>(
          'qyro_buffer_free',
        );

  /// Opens the library the way `QyroNativeApi` already does on three platforms.
  factory QyroSessionBindings.openDefault() {
    final override = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
    if (override != null && override.isNotEmpty) {
      return QyroSessionBindings._(DynamicLibrary.open(override));
    }
    if (Platform.isIOS) {
      return QyroSessionBindings._(DynamicLibrary.process());
    }
    return QyroSessionBindings._(
      DynamicLibrary.open(
        QyroNativeApi.libraryNameForOperatingSystem(Platform.operatingSystem),
      ),
    );
  }

  factory QyroSessionBindings.open(String path) =>
      QyroSessionBindings._(DynamicLibrary.open(path));

  // Private because their types are private: the eight symbols are an
  // implementation detail of this file, and QyroSession -- which lives here --
  // is the only thing that may reach them. A caller outside gets the session,
  // not the ABI.
  final _OpenSenderDart _openSender;
  final _OpenReceiverDart _openReceiver;
  final _StepDart _step;
  final _ProgressQueryDart _readProgress;
  final _HandleOnlyDart _cancel;
  final _HandleOnlyDart _close;
  final _AllocDart _alloc;
  final _FreeDart _free;
}

// --------------------------------------------------------------------- buffers

/// A borrowed native buffer that carries its own length. ADR-0038.
///
/// The length travels beside the pointer because the free side needs the exact
/// number it was allocated with, and that is the one obligation the boundary
/// cannot check. No caller in this repository has to remember it.
final class _Borrowed {
  _Borrowed._(this._bindings, this.pointer, this.length);

  factory _Borrowed.ofUtf8(QyroSessionBindings bindings, String value) {
    final bytes = utf8.encode(value);
    return _Borrowed.ofBytes(bindings, bytes);
  }

  factory _Borrowed.ofBytes(QyroSessionBindings bindings, List<int> bytes) {
    if (bytes.isEmpty) {
      return _Borrowed._(bindings, nullptr, 0);
    }
    final pointer = bindings._alloc(bytes.length);
    if (pointer == nullptr) {
      throw const QyroSessionFailure(QyroCode.unknown, 'qyro_buffer_alloc');
    }
    final view = pointer.asTypedList(bytes.length);
    view.setAll(0, bytes);
    return _Borrowed._(bindings, pointer, bytes.length);
  }

  final QyroSessionBindings _bindings;
  final Pointer<Uint8> pointer;
  final int length;

  void release() => _bindings._free(pointer, length);
}

/// Runs [body] with every buffer released afterwards, on every path out.
T _withBorrowed<T>(List<_Borrowed> borrowed, T Function() body) {
  try {
    return body();
  } finally {
    for (final buffer in borrowed) {
      buffer.release();
    }
  }
}

// --------------------------------------------------------------------- session

/// One transfer, driven from Dart.
///
/// The handle and the `NativeCallable` are both closed by [dispose], and
/// [dispose] is idempotent, so phase 05 does not have to remember either.
final class QyroSession {
  QyroSession._(this._bindings, this._handle, this._callable, this._context);

  static int _nextContext = 1;
  static final Map<int, QyroProgressCallback> _observers =
      <int, QyroProgressCallback>{};

  /// The listener target. Static because `NativeCallable.listener` cannot close
  /// over anything; the opaque context is what routes an emission back to its
  /// session, which is exactly why ADR-0033 §2 carries one.
  static void _dispatch(int context, int done, int total, int item) {
    final observer = _observers[context];
    if (observer == null) {
      return;
    }
    observer(QyroProgress(done: done, total: total, item: item));
  }

  /// Opens a sending session. **Blocks**: it dials and completes a handshake.
  static QyroSession send({
    required QyroSessionBindings bindings,
    required String to,
    required String root,
    required List<String> files,
    QyroProgressCallback? onProgress,
  }) {
    // NUL-separated, because it is the one byte no path may contain on either
    // platform, so the separator cannot appear inside a name. Written as an
    // escape and not as a literal NUL, because a raw NUL in a source file makes
    // every text tool treat it as binary -- the first draft of this file did
    // exactly that.
    final joined = files.join(_pathSeparator);

    final address = _Borrowed.ofUtf8(bindings, to);
    final rootBuffer = _Borrowed.ofUtf8(bindings, root);
    final paths = _Borrowed.ofUtf8(bindings, joined);
    final out = _Borrowed.ofBytes(bindings, List<int>.filled(8, 0));

    final context = _nextContext++;
    NativeCallable<_ProgressNative>? callable;
    if (onProgress != null) {
      _observers[context] = onProgress;
      callable = NativeCallable<_ProgressNative>.listener(_dispatch);
    }

    final code = _withBorrowed([address, rootBuffer, paths], () {
      return bindings._openSender(
        address.pointer,
        address.length,
        rootBuffer.pointer,
        rootBuffer.length,
        paths.pointer,
        paths.length,
        callable?.nativeFunction ?? nullptr,
        context,
        out.pointer.cast<Uint64>(),
      );
    });

    if (code != QyroCode.ok) {
      out.release();
      callable?.close();
      _observers.remove(context);
      throw QyroSessionFailure(code, 'qyro_session_open_sender_blocking');
    }
    final handle = out.pointer.cast<Uint64>().value;
    out.release();
    return QyroSession._(bindings, handle, callable, context);
  }

  /// Opens a receiving session. **Blocks**: it binds, accepts and handshakes.
  static QyroSession receive({
    required QyroSessionBindings bindings,
    required String bind,
    required String destination,
    QyroProgressCallback? onProgress,
  }) {
    final bindBuffer = _Borrowed.ofUtf8(bindings, bind);
    final destinationBuffer = _Borrowed.ofUtf8(bindings, destination);
    final out = _Borrowed.ofBytes(bindings, List<int>.filled(8, 0));

    final context = _nextContext++;
    NativeCallable<_ProgressNative>? callable;
    if (onProgress != null) {
      _observers[context] = onProgress;
      callable = NativeCallable<_ProgressNative>.listener(_dispatch);
    }

    final code = _withBorrowed([bindBuffer, destinationBuffer], () {
      return bindings._openReceiver(
        bindBuffer.pointer,
        bindBuffer.length,
        destinationBuffer.pointer,
        destinationBuffer.length,
        callable?.nativeFunction ?? nullptr,
        context,
        out.pointer.cast<Uint64>(),
      );
    });

    if (code != QyroCode.ok) {
      out.release();
      callable?.close();
      _observers.remove(context);
      throw QyroSessionFailure(code, 'qyro_session_open_receiver_blocking');
    }
    final handle = out.pointer.cast<Uint64>().value;
    out.release();
    return QyroSession._(bindings, handle, callable, context);
  }

  final QyroSessionBindings _bindings;
  final int _handle;
  final NativeCallable<_ProgressNative>? _callable;
  final int _context;
  var _disposed = false;

  /// Advances the transfer by one step. **Blocks** on the socket.
  ///
  /// The name carries the warning ADR-0032 §7 asks for: this is the one call
  /// that can block without a bound, so it must not run on the UI isolate.
  QyroSessionState stepBlocking() {
    _refuseIfDisposed();
    final out = _Borrowed.ofBytes(_bindings, List<int>.filled(4, 0));
    try {
      final code = _bindings._step(_handle, out.pointer.cast<Int32>());
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_session_step_blocking');
      }
      return switch (out.pointer.cast<Int32>().value) {
        0 => QyroSessionState.inProgress,
        1 => QyroSessionState.completed,
        2 => QyroSessionState.rejected,
        final other => throw QyroSessionFailure(other, 'unknown state'),
      };
    } finally {
      out.release();
    }
  }

  /// Steps until the transfer ends, yielding between steps.
  ///
  /// The yield is not decoration. `NativeCallable.listener` delivers on the
  /// event loop of the isolate that created it, so an isolate that never
  /// returns to its loop receives every emission in one burst at the end.
  Future<QyroSessionState> run() async {
    while (true) {
      final state = stepBlocking();
      await Future<void>.delayed(Duration.zero);
      if (state != QyroSessionState.inProgress) {
        return state;
      }
    }
  }

  QyroProgress progress() {
    _refuseIfDisposed();
    final done = _Borrowed.ofBytes(_bindings, List<int>.filled(8, 0));
    final total = _Borrowed.ofBytes(_bindings, List<int>.filled(8, 0));
    final item = _Borrowed.ofBytes(_bindings, List<int>.filled(4, 0));
    try {
      final code = _bindings._readProgress(
        _handle,
        done.pointer.cast<Uint64>(),
        total.pointer.cast<Uint64>(),
        item.pointer.cast<Uint32>(),
      );
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_session_progress');
      }
      return QyroProgress(
        done: done.pointer.cast<Uint64>().value,
        total: total.pointer.cast<Uint64>().value,
        item: item.pointer.cast<Uint32>().value,
      );
    } finally {
      done.release();
      total.release();
      item.release();
    }
  }

  /// Asks the session to stop. Does not block, and is safe from any isolate.
  void cancel() {
    if (_disposed) {
      return;
    }
    _bindings._cancel(_handle);
  }

  /// Closes the handle and the callback. Idempotent.
  ///
  /// Both halves matter and for different reasons: the handle would leak until
  /// the process ended (ADR-0032 §4), and the `NativeCallable` would keep the
  /// isolate that created it alive for ever (ADR-0033 §3, rule 2), which shows
  /// up as a test binary that finishes its work and never exits.
  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    _bindings._close(_handle);
    _callable?.close();
    _observers.remove(_context);
  }

  void _refuseIfDisposed() {
    if (_disposed) {
      throw const QyroSessionFailure(
        QyroCode.invalidHandle,
        'the session was disposed',
      );
    }
  }
}
