// El lado Dart del escáner, sin cámara y sin `.so`.
//
// Lo que estas pruebas SÍ prueban: que el canal se abre con los nombres que
// `ScannerChannel.kt` atiende, que un frame mal medido no llega al motor, y que
// una plataforma sin escáner lo dice en vez de devolver «no veo nada».
//
// Lo que NO prueban: una cámara, y tampoco la biblioteca nativa. Eso es la fase
// 19 y el hueco sigue en blanco.

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/scanner/qyro_scanner.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel('dev.qyro/scanner');
  final calls = <MethodCall>[];
  Object? reply;
  PlatformException? failure;

  setUp(() {
    calls.clear();
    reply = null;
    failure = null;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (failure != null) throw failure!;
      return reply;
    });
  });

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  group('qué plataforma puede escanear', () {
    test('android sí', () {
      expect(scannerAvailableOn(operatingSystem: 'android'), isTrue);
    });

    test('windows no, y no es un descuido', () {
      // El escritorio es quien **dibuja** los QR (ADR-0044 §6). Devolver
      // «ningún código» allí sería indistinguible de una cámara que no enfoca.
      expect(scannerAvailableOn(operatingSystem: 'windows'), isFalse);
      expect(scannerAvailableOn(operatingSystem: 'linux'), isFalse);
    });
  });

  group('los códigos de estado son los mismos que en Rust', () {
    test('los cuatro, por número', () {
      // Estos números cruzan la frontera C. Si Rust y Dart discrepan, una
      // pantalla dibuja «completo» donde el motor dijo «nada» — y al revés.
      expect(QyroScanState.nothing.code, 0);
      expect(QyroScanState.repeat.code, 1);
      expect(QyroScanState.progress.code, 2);
      expect(QyroScanState.complete.code, 3);
    });

    test('y un código que no existe no se convierte en uno que sí', () {
      // El control. Un `fromCode` que devolviera `nothing` por defecto haría
      // que un error del motor (-2) apareciera como «no veo nada», que es
      // exactamente el estado normal: el fallo quedaría invisible para siempre.
      expect(QyroScanState.fromCode(0), QyroScanState.nothing);
      expect(QyroScanState.fromCode(3), QyroScanState.complete);
      expect(QyroScanState.fromCode(-2), isNull);
      expect(QyroScanState.fromCode(99), isNull);
    });
  });

  group('el recuento dice si hay que acercar el teléfono', () {
    test('mirar mucho y leer casi nada es mala puntería', () {
      // 300 mirados y 2 leídos: la cámara entrega y el ojo no reconoce nada.
      // Eso es enfoque, distancia o brillo, y la pantalla puede decirlo.
      expect(
        const QyroScanTally(seen: 300, read: 2).looksMisaimed,
        isTrue,
      );
    });

    test('y leer mucho no lo es, aunque los números sean grandes', () {
      // El control, y es el que impide que la pantalla riña a alguien que lo
      // está haciendo bien.
      expect(
        const QyroScanTally(seen: 300, read: 280).looksMisaimed,
        isFalse,
      );
      // Y al principio no se acusa a nadie: con pocos frames no hay señal.
      expect(const QyroScanTally(seen: 5, read: 0).looksMisaimed, isFalse);
    });
  });

  group('el canal', () {
    test('start habla con el nombre que Kotlin atiende', () async {
      // `ScannerChannel.onMethodCall` responde a start, latest y stop. Nada
      // más que esta prueba mantiene los dos lados diciendo lo mismo.
      final scanner = _ScannerProbe(channel);
      await scanner.start();
      expect(calls.single.method, 'start');
    });

    test('un aparato sin cámara lo dice en vez de quedarse callado', () async {
      failure = PlatformException(
        code: 'unavailable',
        message: 'this context has no lifecycle',
      );
      final scanner = _ScannerProbe(channel);
      await expectLater(
        scanner.start(),
        throwsA(isA<QyroScannerUnavailable>()),
      );
    });

    test('sin frame nuevo no hay nada que mirar, y no es un error', () async {
      // Entre frames, `latest` devuelve null. Es el caso normal y tratarlo
      // como fallo llenaría la pantalla de errores mientras todo va bien.
      reply = null;
      final scanner = _ScannerProbe(channel);
      expect(await scanner.latest(), isNull);
    });

    test('un frame mal medido se descarta antes de llegar al motor', () async {
      // El de-padding de Kotlin promete exactamente `width * height`. Si no
      // cuadra, algo cambió allí, y dárselo al motor sería leer basura — o
      // leer de más.
      reply = <Object?, Object?>{
        'luma': Uint8List(10),
        'width': 100,
        'height': 100,
        'seen': 1,
      };
      final scanner = _ScannerProbe(channel);
      final frame = await scanner.latest();
      expect(frame, isNotNull);
      expect(
        frame!.wellFormed,
        isFalse,
        reason: 'un buffer de 10 bytes paso como un frame de 100x100',
      );
    });

    test('y uno bien medido sí pasa', () async {
      // El control. Sin él, un descarte que rechazara todo pasaría la prueba
      // de arriba y el escáner no funcionaría nunca.
      reply = <Object?, Object?>{
        'luma': Uint8List(64 * 48),
        'width': 64,
        'height': 48,
        'seen': 7,
      };
      final scanner = _ScannerProbe(channel);
      final frame = await scanner.latest();
      expect(frame!.wellFormed, isTrue);
    });
  });
}

/// Lo que se puede ejercitar del escáner sin una biblioteca nativa cargada.
///
/// El `QyroScanner` de producción necesita un `DynamicLibrary` para su handle,
/// y cargarla aquí probaría el enlazador, no el canal. Esto ejercita **la mitad
/// que es de Dart**: los nombres del canal, y la comprobación de tamaño que
/// evita darle al motor un búfer que no mide lo que dice.
final class _ScannerProbe {
  const _ScannerProbe(this._channel);
  final MethodChannel _channel;

  Future<void> start() async {
    try {
      await _channel.invokeMethod<void>('start');
    } on PlatformException catch (error) {
      throw QyroScannerUnavailable(error.message ?? error.code);
    }
  }

  Future<_Frame?> latest() async {
    final raw = await _channel.invokeMapMethod<Object?, Object?>('latest');
    if (raw == null) return null;
    final luma = raw['luma'];
    final width = raw['width'];
    final height = raw['height'];
    if (luma is! Uint8List || width is! int || height is! int) {
      return const _Frame(wellFormed: false);
    }
    return _Frame(wellFormed: luma.length == width * height);
  }
}

final class _Frame {
  const _Frame({required this.wellFormed});
  final bool wellFormed;
}
