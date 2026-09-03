// El puente entre la cámara y el ojo.
//
// ADR-0048 y la fase 24B. Kotlin saca **sólo el plano Y** de cada frame y lo
// pasa por `dev.qyro/scanner`; aquí se copia a un búfer que el motor sabe
// liberar (`qyro_buffer_alloc`, ADR-0038) y se le da a `qyro_scanner_look`.
//
// **Cero `unsafe` nuevo, cero JNI, cero excepción a `forbid(unsafe_code)`.** El
// único `unsafe` que participa es el que esta frontera ya tenía desde ADR-0032.
//
// # El coste, que hay que medir y no suponer
//
// Un plano de luma a 1280×720 son **921 600 bytes por frame**. A 5 por segundo,
// 4,6 MB/s cruzando un MethodChannel y otra vez por FFI. Si el aparato lo
// sostiene, esto está hecho para siempre; si no, el cruce de copia cero por JNI
// tiene entonces su argumento **medido** en vez de supuesto. `framesPerSecond`
// existe para escribir ese número.

import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'package:flutter/services.dart';

/// En qué quedó un frame. Los códigos son los de `ScanState` en Rust.
enum QyroScanState {
  /// No había código legible. El caso más común, y no significa nada malo.
  nothing(0),

  /// Había código y ya se conocía. A 30 fps de cámara contra 5 de pantalla,
  /// cada QR se ve unas seis veces.
  repeat(1),

  /// Código nuevo.
  progress(2),

  /// Con éste ya está.
  complete(3),

  /// Lo leído no era un archivo: era un **código de emparejamiento**.
  ///
  /// **QYR-0381.** Una rama distinta y no un grado de progreso: las cuatro de
  /// arriba dicen «sigue mirando», y ésta dice «para, ya tienes lo que hacía
  /// falta». El código se saca con [QyroScanner.pairing].
  pairing(4);

  const QyroScanState(this.code);
  final int code;

  static QyroScanState? fromCode(int code) {
    for (final state in QyroScanState.values) {
      if (state.code == code) return state;
    }
    return null;
  }
}

/// Por qué no se puede escanear.
final class QyroScannerUnavailable implements Exception {
  const QyroScannerUnavailable(this.reason);
  final String reason;

  @override
  String toString() => 'QyroScannerUnavailable: $reason';
}

// --------------------------------------------------------------- las firmas

typedef _OpenNative = Int32 Function(Pointer<Uint64>);
typedef _OpenDart = int Function(Pointer<Uint64>);
typedef _LookNative = Int32 Function(Uint64, Pointer<Uint8>, IntPtr, IntPtr);
typedef _LookDart = int Function(int, Pointer<Uint8>, int, int);
typedef _TallyNative = Int32 Function(Uint64, Pointer<Uint64>, Pointer<Uint64>);
typedef _TallyDart = int Function(int, Pointer<Uint64>, Pointer<Uint64>);
typedef _ResultLenNative = Int32 Function(Uint64, Pointer<IntPtr>);
typedef _ResultLenDart = int Function(int, Pointer<IntPtr>);
typedef _ResultNative = Int32 Function(Uint64, Pointer<Uint8>, IntPtr);
typedef _ResultDart = int Function(int, Pointer<Uint8>, int);
typedef _PairingNative = Int32 Function(
    Uint64, Pointer<Uint8>, IntPtr, Pointer<IntPtr>);
typedef _PairingDart = int Function(int, Pointer<Uint8>, int, Pointer<IntPtr>);
typedef _CloseNative = Void Function(Uint64);
typedef _CloseDart = void Function(int);
typedef _AllocNative = Pointer<Uint8> Function(IntPtr);
typedef _AllocDart = Pointer<Uint8> Function(int);
typedef _FreeNative = Void Function(Pointer<Uint8>, IntPtr);
typedef _FreeDart = void Function(Pointer<Uint8>, int);

/// Cuántos frames y cuántos traían código.
///
/// **Las dos, siempre juntas.** «300 mirados, 2 leídos» y «300 mirados, 280
/// leídos» son la misma barra de progreso y dos situaciones opuestas: la primera
/// dice que hay que acercar el teléfono.
final class QyroScanTally {
  const QyroScanTally({required this.seen, required this.read});
  final int seen;
  final int read;

  /// Si la cámara está entregando pero casi nada se lee.
  ///
  /// El umbral es deliberadamente flojo: con 30 frames mirados y menos del 10 %
  /// leídos, algo va mal de verdad — enfoque, distancia o brillo.
  bool get looksMisaimed => seen >= 30 && read * 10 < seen;
}

/// El escáner: frames de la cámara entran, un archivo sale.
final class QyroScanner {
  QyroScanner._(this._library, this._channel);

  static const _defaultChannel = MethodChannel('dev.qyro/scanner');

  final DynamicLibrary _library;
  final MethodChannel _channel;

  int _handle = 0;
  int _framesDelivered = 0;
  DateTime? _firstFrame;

  /// Abre el escáner sobre la biblioteca nativa ya cargada.
  ///
  /// [library] se inyecta para que la prueba no necesite un `.so`; en
  /// producción es la misma que ya usa el resto de la aplicación.
  static QyroScanner open(DynamicLibrary library, {MethodChannel? channel}) {
    final scanner = QyroScanner._(library, channel ?? _defaultChannel);
    scanner._openHandle();
    return scanner;
  }

  /// Reserva `bytes` con el asignador del motor.
  ///
  /// **Y no con `package:ffi`, que es la parte que importa.** `calloc` habría
  /// sido una línea más corta y un paquete de pub.dev más en un producto cuyo
  /// presupuesto para eso es cero. `qyro_buffer_alloc` ya existe (ADR-0038), ya
  /// cruza esta frontera, y ya sabe que una longitud llegada de fuera no debe
  /// poder abortar el proceso.
  Pointer<Uint8> _alloc(int bytes) => _library
      .lookupFunction<_AllocNative, _AllocDart>('qyro_buffer_alloc')(bytes);

  void _free(Pointer<Uint8> buffer, int bytes) =>
      _library.lookupFunction<_FreeNative, _FreeDart>('qyro_buffer_free')(
          buffer, bytes);

  void _openHandle() {
    final open = _library.lookupFunction<_OpenNative, _OpenDart>(
      'qyro_scanner_open',
    );
    final raw = _alloc(8);
    if (raw == nullptr) {
      throw const QyroScannerUnavailable('no memory for the scanner handle');
    }
    try {
      final out = raw.cast<Uint64>();
      final code = open(out);
      if (code != 0) {
        throw QyroScannerUnavailable('qyro_scanner_open returned $code');
      }
      _handle = out.value;
    } finally {
      _free(raw, 8);
    }
  }

  /// Pide el permiso de cámara si hace falta. **No espera la respuesta.**
  ///
  /// **QYR-0378.** `CAMERA` es un permiso **peligroso**: declararlo en el
  /// manifiesto no concede nada desde Android 6, hay que pedirlo en ejecución, y
  /// **nada en este repositorio lo pedía**. `bindToLifecycle` con el permiso
  /// denegado lanza `SecurityException`, que llegaba a la pantalla como «este
  /// aparato no puede mirar» — una frase sobre el aparato, cuando lo que faltaba
  /// era una pregunta que nadie hizo.
  ///
  /// - `granted` — ya está.
  /// - `asked` — el diálogo del sistema está en pantalla **ahora**; quien llama
  ///   lo dice y ofrece reintentar.
  /// - `unavailable` — no hay Activity a la que preguntar, o no es Android.
  Future<String> permission() async {
    try {
      return await _channel.invokeMethod<String>('permission') ?? 'unavailable';
    } on MissingPluginException {
      return 'unavailable';
    } on PlatformException catch (error) {
      throw QyroScannerUnavailable(error.message ?? error.code);
    }
  }

  /// Le pide a la cámara que empiece.
  Future<void> start() async {
    try {
      await _channel.invokeMethod<void>('start');
    } on PlatformException catch (error) {
      throw QyroScannerUnavailable(error.message ?? error.code);
    }
  }

  /// Recoge el último frame, si hay uno nuevo, y se lo da al ojo.
  ///
  /// Devuelve `null` cuando la cámara todavía no ha entregado nada desde la
  /// última vez — que no es un error, es el caso normal entre frames.
  Future<QyroScanState?> pump() async {
    final Map<Object?, Object?>? frame;
    try {
      frame = await _channel.invokeMapMethod<Object?, Object?>('latest');
    } on PlatformException catch (error) {
      throw QyroScannerUnavailable(error.message ?? error.code);
    }
    if (frame == null) return null;

    final luma = frame['luma'];
    final width = frame['width'];
    final height = frame['height'];
    if (luma is! Uint8List || width is! int || height is! int) {
      // Un frame mal formado se descarta. Viene de otro proceso, y descartarlo
      // es más barato que fiarse.
      return null;
    }
    if (luma.length != width * height) {
      // El de-padding de Kotlin promete exactamente `width * height`. Si no
      // cuadra, algo cambió allí y darle esto al ojo sería leer basura.
      return null;
    }

    _firstFrame ??= DateTime.now();
    _framesDelivered += 1;
    return _look(luma, width, height);
  }

  QyroScanState? _look(Uint8List luma, int width, int height) {
    final look = _library.lookupFunction<_LookNative, _LookDart>(
      'qyro_scanner_look',
    );

    final buffer = _alloc(luma.length);
    if (buffer == nullptr) {
      // ADR-0038: un puntero nulo es como viaja un fallo de reserva. Un frame
      // perdido cuesta un frame, que es exactamente lo que el fountain absorbe.
      return null;
    }
    try {
      buffer.asTypedList(luma.length).setAll(0, luma);
      return QyroScanState.fromCode(look(_handle, buffer, width, height));
    } finally {
      _free(buffer, luma.length);
    }
  }

  /// Cuántos frames se han mirado y cuántos traían código.
  QyroScanTally tally() {
    final tally = _library.lookupFunction<_TallyNative, _TallyDart>(
      'qyro_scanner_tally',
    );
    final raw = _alloc(16);
    if (raw == nullptr) return const QyroScanTally(seen: 0, read: 0);
    try {
      final seen = raw.cast<Uint64>();
      final read = (raw + 8).cast<Uint64>();
      if (tally(_handle, seen, read) != 0) {
        return const QyroScanTally(seen: 0, read: 0);
      }
      return QyroScanTally(seen: seen.value, read: read.value);
    } finally {
      _free(raw, 16);
    }
  }

  /// Los frames por segundo que este aparato ha sostenido de verdad.
  ///
  /// **La cifra que la fase 24B existe para escribir**, y por eso se mide en vez
  /// de suponerse: 921 600 bytes por frame a 1280×720 cruzan un MethodChannel y
  /// luego una copia por FFI. Si esto se queda por debajo de 5, el cruce de
  /// copia cero por JNI tiene su argumento medido; si llega, no hace falta.
  ///
  /// `null` hasta que haya llegado más de un frame: con uno solo no hay
  /// intervalo que medir, y devolver un número inventado sería peor que no
  /// devolver ninguno.
  double? framesPerSecond() {
    final first = _firstFrame;
    if (first == null || _framesDelivered < 2) return null;
    final elapsed = DateTime.now().difference(first).inMicroseconds;
    if (elapsed <= 0) return null;
    return _framesDelivered * 1000000 / elapsed;
  }

  /// El archivo, cuando está entero. `null` mientras falte algo.
  ///
  /// **Nunca uno a medias**: uno casi correcto falla el hash y nada explica por
  /// qué.
  Uint8List? result() {
    final resultLen = _library.lookupFunction<_ResultLenNative, _ResultLenDart>(
      'qyro_scanner_result_len',
    );
    final resultInto = _library.lookupFunction<_ResultNative, _ResultDart>(
      'qyro_scanner_result',
    );
    final lengthRaw = _alloc(8);
    if (lengthRaw == nullptr) return null;
    try {
      final lengthOut = lengthRaw.cast<IntPtr>();
      if (resultLen(_handle, lengthOut) != 0) return null;
      final length = lengthOut.value;
      if (length <= 0) return null;

      final buffer = _alloc(length);
      if (buffer == nullptr) return null;
      try {
        if (resultInto(_handle, buffer, length) != 0) return null;
        return Uint8List.fromList(buffer.asTypedList(length));
      } finally {
        _free(buffer, length);
      }
    } finally {
      _free(lengthRaw, 8);
    }
  }

  /// El código de emparejamiento leído, entero, o `null` si no hubo ninguno.
  ///
  /// **QYR-0381, y sale entero con su huella.** Devolver sólo la dirección sería
  /// repetir el defecto que QYR-0392 arregló en la otra cara: la huella es lo
  /// que hace que escanear valga más que teclear, y quien la recibe la compara
  /// con la del apretón y se niega si no coincide (ADR-0035 §2.1).
  ///
  /// El contrato de dos llamadas del símbolo 35: se pregunta el tamaño con
  /// capacidad cero, se reserva, se pide.
  String? pairing() {
    final read = _library.lookupFunction<_PairingNative, _PairingDart>(
      'qyro_scanner_pairing',
    );
    final lengthRaw = _alloc(8);
    if (lengthRaw == nullptr) return null;
    try {
      final lengthOut = lengthRaw.cast<IntPtr>();
      // Preguntar. Un código que no existe contesta «todavía no», que aquí
      // significa «no se ha leído ninguno» y no es un error de llamada.
      read(_handle, nullptr, 0, lengthOut);
      final length = lengthOut.value;
      if (length <= 0) return null;

      final buffer = _alloc(length);
      if (buffer == nullptr) return null;
      try {
        if (read(_handle, buffer, length, lengthOut) != 0) return null;
        final wrote = lengthOut.value;
        if (wrote <= 0 || wrote > length) return null;
        return utf8.decode(buffer.asTypedList(wrote));
      } finally {
        _free(buffer, length);
      }
    } finally {
      _free(lengthRaw, 8);
    }
  }

  /// Cierra la cámara y el escaneo. Llamarlo dos veces no hace daño.
  Future<void> close() async {
    if (_handle != 0) {
      final close = _library.lookupFunction<_CloseNative, _CloseDart>(
        'qyro_scanner_close',
      );
      close(_handle);
      _handle = 0;
    }
    try {
      await _channel.invokeMethod<void>('stop');
    } on PlatformException {
      // Parar algo que nunca empezó no es un fallo que propagar.
    }
  }
}

/// Si esta plataforma puede escanear.
///
/// **Android y nada más, y se dice por su nombre.** Windows no tiene el canal:
/// el escritorio es quien **dibuja** los QR (ADR-0044 §6), y devolver «ningún
/// código» allí sería indistinguible de una cámara que no enfoca.
bool scannerAvailableOn({String? operatingSystem}) =>
    (operatingSystem ?? Platform.operatingSystem) == 'android';
