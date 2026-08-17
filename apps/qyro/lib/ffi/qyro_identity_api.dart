// The device identity, from Dart.
//
// Specification: docs/adr/ADR-0040-identity-persistence.md.
//
// Until phase 11 the engine minted a fresh keypair for every session, so the
// fingerprint a person was asked to compare out loud changed between one
// transfer and the next, and this application could not show its own pairing
// code because it had no stable identity to build one from. These three calls
// are what closes that.
//
// Nothing here crosses a type: a path in, a code back, text out through a
// buffer this side lends to Rust (ADR-0038). **No key material ever reaches
// Dart** — the wrapped blob and the seed both stay below this boundary, and
// that is the one rule of this project that has never been amended.

import 'dart:convert';
import 'dart:ffi';

import 'qyro_session_api.dart';

/// How the seed on disk is protected. ADR-0040 amendment 1.
enum QyroProtection {
  /// The platform's own wrapper: DPAPI on Windows.
  ///
  /// **Refuses when there is none.** It never quietly degrades to [sandbox]:
  /// nobody receives less protection than they asked for because a wrapper
  /// happened to be missing.
  platform,

  /// The filesystem sandbox, and nothing else.
  ///
  /// Android's stage A. Honest sentence, and it is in THREAT_MODEL.md rather
  /// than a footnote: *with Keystore an attacker with root would still need the
  /// TEE; with the sandbox, root is enough.*
  sandbox;

  int get code => switch (this) {
        QyroProtection.platform => 0,
        QyroProtection.sandbox => 1,
      };
}

typedef _OpenNative = Int32 Function(Pointer<Uint8>, IntPtr, Uint32);
typedef _OpenDart = int Function(Pointer<Uint8>, int, int);
typedef _TextOutNative = Int32 Function(
    Pointer<Uint8>, IntPtr, Pointer<UintPtr>);
typedef _TextOutDart = int Function(Pointer<Uint8>, int, Pointer<UintPtr>);

/// The three identity calls.
final class QyroIdentityBindings {
  QyroIdentityBindings(this._session, DynamicLibrary library)
      : _open = library.lookupFunction<_OpenNative, _OpenDart>(
          'qyro_identity_open_blocking',
        ),
        _fingerprint = library.lookupFunction<_TextOutNative, _TextOutDart>(
          'qyro_identity_fingerprint',
        );

  /// Opens the same library the session half already opened.
  factory QyroIdentityBindings.open(QyroSessionBindings session) =>
      QyroIdentityBindings(session, session.library);

  final QyroSessionBindings _session;
  final _OpenDart _open;
  final _TextOutDart _fingerprint;

  /// Loads the identity at [path], creating one if the store is empty.
  ///
  /// Must succeed before any session is opened; otherwise every `open`
  /// answers [QyroCode.identityUnreadable]. Blocking — it touches a disk — so
  /// callers on the UI isolate should run it off the frame.
  ///
  /// Throws [QyroSessionFailure] on refusal. **A refusal is never a reason to
  /// carry on without an identity**: continuing would give the person a
  /// different fingerprint on every transfer, which is the defect this exists
  /// to remove.
  void open(String path, QyroProtection protection) {
    final bytes = utf8.encode(path);
    final borrowed = QyroBorrowed.ofBytes(_session, bytes);
    try {
      final code = _open(borrowed.pointer, bytes.length, protection.code);
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'opening the identity at $path');
      }
    } finally {
      borrowed.release();
    }
  }

  /// This device's own fingerprint, grouped for reading aloud.
  ///
  /// Ask-length, allocate, ask-again — the same two-call protocol every text
  /// value on this boundary uses, and nothing is written when it does not fit,
  /// because half a fingerprint that matches proves nothing.
  String fingerprint() {
    final lengthCell = QyroBorrowed.ofBytes(_session, List<int>.filled(8, 0));
    try {
      _fingerprint(nullptr, 0, lengthCell.pointer.cast<UintPtr>());
      final needed = lengthCell.pointer.cast<UintPtr>().value;
      if (needed == 0) {
        return '';
      }
      final out = QyroBorrowed.ofBytes(_session, List<int>.filled(needed, 0));
      try {
        final code = _fingerprint(
            out.pointer, needed, lengthCell.pointer.cast<UintPtr>());
        if (code != QyroCode.ok) {
          throw QyroSessionFailure(code, 'reading this device\'s fingerprint');
        }
        final wrote = lengthCell.pointer.cast<UintPtr>().value;
        return utf8.decode(out.pointer.asTypedList(wrote));
      } finally {
        out.release();
      }
    } finally {
      lengthCell.release();
    }
  }
}
