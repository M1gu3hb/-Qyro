// Dónde escribe Qyro lo que recibe, y qué hace cuando no puede saberlo.
//
// QYR-0373. El defecto era que en Android el destino salía `/Qyro` —la raíz del
// sistema— porque `Directory.current.path` es `/` en un proceso de Android y el
// lado Kotlin que iba a pasar la ruta buena nunca se escribió. Recibir fallaba
// al crear la carpeta, **antes de emitir un solo estado**, así que pulsar
// Recibir en el teléfono no hacía nada visible.
//
// Lo que se prueba aquí es lo que se puede probar sin un teléfono: que el puente
// pide lo que tiene que pedir, y que **ninguno de sus fallos lanza** — porque un
// arreglo que revienta cuando el canal no está sería peor que el defecto.

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/transfer/qyro_paths.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  /// Un canal que contesta lo que el caso quiera, o lanza lo que quiera.
  MethodChannel channelThat(Future<Object?> Function(MethodCall call) answer) {
    const channel = MethodChannel('dev.qyro/paths.test');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, answer);
    addTearDown(
      () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null),
    );
    return channel;
  }

  test('pide `destination` y devuelve lo que Android conteste', () async {
    var asked = '';
    final channel = channelThat((call) async {
      asked = call.method;
      return '/sdcard/Android/data/dev.qyro.app/files/Qyro';
    });

    final where = await androidDestination(channel: channel, isAndroid: true);

    expect(asked, 'destination');
    expect(where, '/sdcard/Android/data/dev.qyro.app/files/Qyro');
  });

  test('en Windows no pregunta nada, porque allí el de siempre acierta',
      () async {
    // El control de la prueba de arriba: si preguntara siempre, la de arriba
    // pasaría igual y esto no distinguiría nada.
    var asked = false;
    final channel = channelThat((call) async {
      asked = true;
      return '/no debería llegar aquí';
    });

    expect(
      await androidDestination(channel: channel, isAndroid: false),
      isNull,
    );
    expect(asked, isFalse);
  });

  test('ningún fallo del canal lanza: se contesta null y se sigue', () async {
    // Los tres fallos reales, uno a uno. Un arreglo que revienta cuando el canal
    // no está sería peor que el defecto que arregla: una build vieja, o una
    // prueba de widgets sin plataforma debajo, dejarían la aplicación sin
    // pantalla de recibir en vez de con una carpeta rara.
    final missing = channelThat((call) async {
      throw MissingPluginException('no such channel');
    });
    expect(await androidDestination(channel: missing, isAndroid: true), isNull);

    final refused = channelThat((call) async {
      throw PlatformException(code: 'no-external-storage');
    });
    expect(await androidDestination(channel: refused, isAndroid: true), isNull);

    // Y `null` del propio Kotlin, que es lo que contesta cuando el
    // almacenamiento externo no está montado. Y la cadena vacía, que es la misma
    // respuesta escrita de otra forma y que sin este caso se propagaría como una
    // ruta relativa al directorio de trabajo -- que es exactamente el defecto.
    final nothing = channelThat((call) async => null);
    expect(await androidDestination(channel: nothing, isAndroid: true), isNull);

    final empty = channelThat((call) async => '');
    expect(await androidDestination(channel: empty, isAndroid: true), isNull);
  });
}
