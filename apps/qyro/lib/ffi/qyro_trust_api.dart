// Trust, fingerprints, refusals and pairing strings, from Dart.
//
// Specification: docs/adr/ADR-0032-engine-ffi.md amendment 1 for the shape, and
// docs/adr/ADR-0035-discovery-and-pairing.md for what the values mean.
//
// Nothing here crosses a type. Every call returns an `int` code and writes its
// answer into a buffer this side lent to Rust (ADR-0038), which is why the text
// helper below is the only place that knows the two-call protocol: ask for the
// length, allocate, ask again.

import 'dart:convert';
import 'dart:ffi';

import 'qyro_session_api.dart';

/// What the peer store says. Three words and no boolean.
///
/// ADR-0031 refused a boolean on purpose: «new» and «known» are not two values
/// of one question, and a `bool` erases which one you were told.
enum QyroPeerTrust {
  /// A record exists under this name and the whole public identity matches.
  known,

  /// A record exists and the identity **changed**. In SSH this is a shouted
  /// warning; it is one here, and it must never render as a softer `newPeer`.
  changed,

  /// No record under this name. Not an error, and not permission either.
  newPeer;

  static QyroPeerTrust fromCode(int code) => switch (code) {
        0 => QyroPeerTrust.known,
        1 => QyroPeerTrust.changed,
        2 => QyroPeerTrust.newPeer,
        _ => throw QyroSessionFailure(code, 'unknown trust verdict'),
      };
}

/// Why a receiver refused.
enum QyroRejectReason {
  declined,
  noRoom,
  unacceptableManifest,
  unspecified;

  int get code => switch (this) {
        QyroRejectReason.declined => 0,
        QyroRejectReason.noRoom => 1,
        QyroRejectReason.unacceptableManifest => 2,
        QyroRejectReason.unspecified => 3,
      };

  static QyroRejectReason? fromCode(int code) => switch (code) {
        0 => QyroRejectReason.declined,
        1 => QyroRejectReason.noRoom,
        2 => QyroRejectReason.unacceptableManifest,
        3 => QyroRejectReason.unspecified,
        // `-1` is «it did not refuse», which is not a reason and must not
        // become one.
        _ => null,
      };
}

// --------------------------------------------------------------- native types

typedef _TextOutNative = Int32 Function(
    Uint64 handle, Pointer<Uint8> out, UintPtr capacity, Pointer<UintPtr> len);
typedef _TextOutDart = int Function(int, Pointer<Uint8>, int, Pointer<UintPtr>);

typedef _NameQueryNative = Int32 Function(
    Uint64 handle, Pointer<Uint8> name, UintPtr nameLen, Pointer<Int32> out);
typedef _NameQueryDart = int Function(int, Pointer<Uint8>, int, Pointer<Int32>);

typedef _NameOnlyNative = Int32 Function(
    Uint64 handle, Pointer<Uint8> name, UintPtr nameLen);
typedef _NameOnlyDart = int Function(int, Pointer<Uint8>, int);

typedef _ForgetNative = Int32 Function(
    Pointer<Uint8> name, UintPtr nameLen, Pointer<Int32> out);
typedef _ForgetDart = int Function(Pointer<Uint8>, int, Pointer<Int32>);

typedef _ListNative = Int32 Function(
    Pointer<Uint8> out, UintPtr capacity, Pointer<UintPtr> len);
typedef _ListDart = int Function(Pointer<Uint8>, int, Pointer<UintPtr>);

typedef _HandleOnlyNative = Int32 Function(Uint64 handle);
typedef _HandleOnlyDart = int Function(int);

typedef _RejectNative = Int32 Function(Uint64 handle, Int32 reason);
typedef _RejectDart = int Function(int, int);

typedef _RejectionNative = Int32 Function(Uint64 handle, Pointer<Int32> out);
typedef _RejectionDart = int Function(int, Pointer<Int32>);

typedef _PairingNative = Int32 Function(Pointer<Uint8> text, UintPtr textLen,
    Pointer<Uint8> out, UintPtr capacity, Pointer<UintPtr> len);
typedef _PairingDart = int Function(
    Pointer<Uint8>, int, Pointer<Uint8>, int, Pointer<UintPtr>);

/// Lo que el par está ofreciendo, ya emparejado nombre con tamaño.
///
/// Una clase y no un registro para que el tipo tenga nombre en los mensajes de
/// error del análisis: esto cruza un `Isolate` y un tipo anónimo allí se lee
/// como una tupla sin contexto.
final class QyroOffered {
  const QyroOffered({required this.names, required this.totalBytes});

  final List<String> names;
  final int totalBytes;
}

/// Los símbolos de ADR-0032 enmienda 1, más los dos de la enmienda 6, buscados
/// una sola vez.
final class QyroTrustBindings {
  QyroTrustBindings(this._session, DynamicLibrary library)
      : _peerFingerprint = library.lookupFunction<_TextOutNative, _TextOutDart>(
          'qyro_session_peer_fingerprint',
        ),
        _localAddress = library.lookupFunction<_TextOutNative, _TextOutDart>(
          'qyro_session_local_address',
        ),
        _peerTrust = library.lookupFunction<_NameQueryNative, _NameQueryDart>(
          'qyro_session_peer_trust',
        ),
        _rememberPeer = library.lookupFunction<_NameOnlyNative, _NameOnlyDart>(
          'qyro_session_remember_peer',
        ),
        _forgetPeer = library.lookupFunction<_ForgetNative, _ForgetDart>(
          'qyro_trust_forget_peer',
        ),
        _listPeers = library.lookupFunction<_ListNative, _ListDart>(
          'qyro_trust_list_peers',
        ),
        _reject = library.lookupFunction<_RejectNative, _RejectDart>(
          'qyro_session_reject',
        ),
        _rejection = library.lookupFunction<_RejectionNative, _RejectionDart>(
          'qyro_session_rejection',
        ),
        _awaitOffer =
            library.lookupFunction<_HandleOnlyNative, _HandleOnlyDart>(
          'qyro_session_await_offer_blocking',
        ),
        _offeredFiles = library.lookupFunction<_TextOutNative, _TextOutDart>(
          'qyro_session_offered_files',
        ),
        _pairingParse = library.lookupFunction<_PairingNative, _PairingDart>(
          'qyro_pairing_parse',
        ),
        // QYR-0392. La otra mitad de la cadena, y la que faltaba: sin ella
        // esta cara podia marcar la direccion y no podia comparar nada.
        _pairingFingerprint =
            library.lookupFunction<_PairingNative, _PairingDart>(
          'qyro_pairing_fingerprint',
        );

  /// Opens the trust half of the same library the session half already opened.
  factory QyroTrustBindings.openDefault(QyroSessionBindings session) =>
      QyroTrustBindings(session, session.library);

  final QyroSessionBindings _session;
  final _TextOutDart _peerFingerprint;
  final _TextOutDart _localAddress;
  final _NameQueryDart _peerTrust;
  final _NameOnlyDart _rememberPeer;
  final _ForgetDart _forgetPeer;
  final _ListDart _listPeers;
  final _RejectDart _reject;
  final _RejectionDart _rejection;
  final _HandleOnlyDart _awaitOffer;
  final _TextOutDart _offeredFiles;
  final _PairingDart _pairingParse;
  final _PairingDart _pairingFingerprint;

  /// Runs the two-call text protocol: ask the length, allocate, ask again.
  ///
  /// The protocol lives here once. Five operations share it, and five copies of
  /// «ask, allocate, ask» is five places for an off-by-one to become half a
  /// fingerprint.
  String _readText(
    String operation,
    int Function(Pointer<Uint8> out, int capacity, Pointer<UintPtr> len) call,
  ) {
    final lengthCell = QyroBorrowed.ofBytes(_session, List<int>.filled(8, 0));
    try {
      // First call: no buffer at all. Rust writes the length it needed and
      // reports `bad_argument`, which here means «not yet», not «wrong».
      call(nullptr, 0, lengthCell.pointer.cast<UintPtr>());
      final needed = lengthCell.pointer.cast<UintPtr>().value;
      if (needed == 0) {
        return '';
      }

      final out = QyroBorrowed.ofBytes(_session, List<int>.filled(needed, 0));
      try {
        final code =
            call(out.pointer, needed, lengthCell.pointer.cast<UintPtr>());
        if (code != QyroCode.ok) {
          throw QyroSessionFailure(code, operation);
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

  /// The peer's fingerprint, **formatted by the core**.
  ///
  /// ADR-0035 §4. Never reformatted here: two devices rendering it differently
  /// makes comparing it out loud worthless, and that is all a fingerprint is for.
  String peerFingerprint(QyroSession session) => _readText(
        'qyro_session_peer_fingerprint',
        (out, capacity, len) =>
            _peerFingerprint(session.handle, out, capacity, len),
      );

  /// The address this end bound, ready to put in a pairing string.
  String localAddress(QyroSession session) => _readText(
        'qyro_session_local_address',
        (out, capacity, len) =>
            _localAddress(session.handle, out, capacity, len),
      );

  /// What the book says about the peer this handshake authenticated.
  QyroPeerTrust peerTrust(QyroSession session, String name) {
    final buffer = QyroBorrowed.ofUtf8(_session, name);
    final out = QyroBorrowed.ofBytes(_session, List<int>.filled(4, 0));
    try {
      final code = _peerTrust(
        session.handle,
        buffer.pointer,
        buffer.length,
        out.pointer.cast<Int32>(),
      );
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_session_peer_trust');
      }
      return QyroPeerTrust.fromCode(out.pointer.cast<Int32>().value);
    } finally {
      buffer.release();
      out.release();
    }
  }

  /// Records this peer under [name]. **Only a person may cause this.**
  void rememberPeer(QyroSession session, String name) {
    final buffer = QyroBorrowed.ofUtf8(_session, name);
    try {
      final code = _rememberPeer(session.handle, buffer.pointer, buffer.length);
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_session_remember_peer');
      }
    } finally {
      buffer.release();
    }
  }

  /// Forgets [name]. Returns whether there was anything to forget.
  bool forgetPeer(String name) {
    final buffer = QyroBorrowed.ofUtf8(_session, name);
    final out = QyroBorrowed.ofBytes(_session, List<int>.filled(4, 0));
    try {
      final code =
          _forgetPeer(buffer.pointer, buffer.length, out.pointer.cast<Int32>());
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_trust_forget_peer');
      }
      return out.pointer.cast<Int32>().value != 0;
    } finally {
      buffer.release();
      out.release();
    }
  }

  /// Every remembered name.
  List<String> listPeers() {
    final joined = _readText(
      'qyro_trust_list_peers',
      (out, capacity, len) => _listPeers(out, capacity, len),
    );
    if (joined.isEmpty) {
      return const <String>[];
    }
    return joined.split('\x00');
  }

  /// Waits until the offer and its manifest have arrived. **Blocks.**
  ///
  /// ADR-0032 enmienda 6, y el número es el hallazgo: hacen falta **dos** pasos,
  /// no uno. `QyroSession.receive` vuelve en cuanto termina el handshake, y este
  /// worker daba un `stepBlocking()` y preguntaba — con `progress().total` a
  /// cero y sin un solo nombre. La tarjeta ofrecía «0 archivos, 0 B».
  ///
  /// El límite vive en `Session::await_offer`, en Rust, y no aquí: un número del
  /// protocolo escrito en los dos lados es el defecto del puerto otra vez.
  ///
  /// Sólo desde un worker: lleva `_blocking` porque bloquea (ADR-0032 §7).
  void awaitOffer(QyroSession session) {
    final code = _awaitOffer(session.handle);
    if (code != QyroCode.ok) {
      throw QyroSessionFailure(code, 'qyro_session_await_offer_blocking');
    }
  }

  /// Qué se está ofreciendo: los nombres y cuántos bytes suman.
  ///
  /// Vacío cuando el manifiesto todavía no ha llegado, que es un estado real:
  /// llama a [awaitOffer] antes. Los nombres llegan **tal como el par los
  /// mandó** y se dibujan con `safeDisplayName`, que es donde ADR-0047 §6 pone
  /// esa regla.
  QyroOffered offeredFiles(QyroSession session) {
    final joined = _readText(
      'qyro_session_offered_files',
      (out, capacity, len) => _offeredFiles(session.handle, out, capacity, len),
    );
    if (joined.isEmpty) {
      return const QyroOffered(names: <String>[], totalBytes: 0);
    }
    // `nombre\0tamaño\0nombre\0tamaño…`. NUL porque es el único byte que un
    // nombre no puede contener, igual que `qyro_trust_list_peers`.
    final fields = joined.split('\x00');
    final names = <String>[];
    var total = 0;
    // Un campo suelto al final significaría que la codificación cambió de un
    // lado y no del otro; se ignora en vez de emparejar un nombre con el tamaño
    // del siguiente, que es como un tamaño acaba dibujado como nombre.
    for (var i = 0; i + 1 < fields.length; i += 2) {
      names.add(fields[i]);
      total += int.tryParse(fields[i + 1]) ?? 0;
    }
    return QyroOffered(names: names, totalBytes: total);
  }

  /// Refuses the offered transfer, with a reason the sender will see.
  void reject(QyroSession session, QyroRejectReason reason) {
    final code = _reject(session.handle, reason.code);
    if (code != QyroCode.ok) {
      throw QyroSessionFailure(code, 'qyro_session_reject');
    }
  }

  /// Why the receiver refused, or null if it did not.
  QyroRejectReason? rejection(QyroSession session) {
    final out = QyroBorrowed.ofBytes(_session, List<int>.filled(4, 0));
    try {
      final code = _rejection(session.handle, out.pointer.cast<Int32>());
      if (code != QyroCode.ok) {
        throw QyroSessionFailure(code, 'qyro_session_rejection');
      }
      return QyroRejectReason.fromCode(out.pointer.cast<Int32>().value);
    } finally {
      out.release();
    }
  }

  /// Validates a pairing string and returns the address to dial.
  ///
  /// Returns null when the string is not one of ours.
  String? addressOfPairingString(String text) =>
      _pairingHalf(_pairingParse, text);

  /// La **expectativa** que lleva la misma cadena: treinta y dos hex.
  ///
  /// **No es una huella autenticada y no establece nada** (ADR-0035 §2.1). Es
  /// lo que hay que comparar contra [peerFingerprint] cuando el apreton termine;
  /// si no coincide, la sesion se rechaza **sin preguntar a nadie**, porque
  /// quien escaneo ya contesto esa pregunta.
  ///
  /// **QYR-0392: hasta aqui la comparacion era imposible desde esta cara.** El
  /// unico simbolo de emparejamiento devolvia la direccion y tiraba la huella,
  /// asi que escanear un QR ataba la sesion a una direccion y a ninguna clave.
  String? fingerprintOfPairingString(String text) =>
      _pairingHalf(_pairingFingerprint, text);

  /// El protocolo de dos llamadas de ADR-0032 enmienda 1, sobre una cadena.
  ///
  /// Una funcion y no dos copias: las dos mitades de una cadena de
  /// emparejamiento tienen que aceptar y rechazar **lo mismo**, y dos copias de
  /// «pregunta, reserva, pregunta» son dos sitios donde eso deja de ser cierto.
  String? _pairingHalf(_PairingDart half, String text) {
    final input = QyroBorrowed.ofUtf8(_session, text);
    final lengthCell = QyroBorrowed.ofBytes(_session, List<int>.filled(8, 0));
    try {
      half(
        input.pointer,
        input.length,
        nullptr,
        0,
        lengthCell.pointer.cast<UintPtr>(),
      );
      final needed = lengthCell.pointer.cast<UintPtr>().value;
      if (needed == 0) {
        return null;
      }
      final out = QyroBorrowed.ofBytes(_session, List<int>.filled(needed, 0));
      try {
        final code = half(
          input.pointer,
          input.length,
          out.pointer,
          needed,
          lengthCell.pointer.cast<UintPtr>(),
        );
        if (code != QyroCode.ok) {
          return null;
        }
        final wrote = lengthCell.pointer.cast<UintPtr>().value;
        return utf8.decode(out.pointer.asTypedList(wrote));
      } finally {
        out.release();
      }
    } finally {
      input.release();
      lengthCell.release();
    }
  }
}
