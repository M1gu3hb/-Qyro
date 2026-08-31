// Dónde escribe Qyro lo que recibe.
//
// **QYR-0373, y era un P0 en el teléfono.** `defaultDestination()` devolvía, en
// Android, `Directory.current.path + '/Qyro'`, bajo un comentario que decía «el
// lado Kotlin la pasa; hasta que lo haga, el directorio de trabajo del proceso
// es la respuesta honesta».
//
// **En Android el directorio de trabajo de un proceso es `/`.** Así que la
// respuesta era `/Qyro`: la raíz del sistema, que ninguna aplicación puede
// escribir. `Directory('/Qyro').createSync(recursive: true)` lanza, y lanza
// dentro de `receive()` **antes de emitir un solo estado** — o sea, pulsar
// Recibir en el teléfono no hacía nada visible. El lado Kotlin que iba a pasar
// la ruta nunca se escribió, y el comentario que lo decía llevaba desde
// entonces sonando a nota temporal.
//
// Esto es ese lado, por fin, del lado de Dart.

import 'dart:io';

import 'package:flutter/services.dart';

/// El canal que contesta dónde escribir. Sólo Android lo implementa.
const MethodChannel qyroPathsChannel = MethodChannel('dev.qyro/paths');

/// La carpeta donde dejar lo que llegue, o `null` para usar la de siempre.
///
/// Devuelve `null` —y no lanza— en tres casos, y los tres son reales:
///
/// - **No es Android.** En Windows `defaultDestination()` ya acierta:
///   `%USERPROFILE%\Downloads\Qyro`, que existe y es escribible.
/// - **El canal no está registrado.** Una build vieja, o una prueba de widgets
///   sin plataforma debajo. Preferir lo de antes a reventar es lo correcto:
///   este archivo arregla un destino, no es el destino.
/// - **Android no pudo dar la carpeta.** El almacenamiento externo puede no
///   estar montado. Contestar `null` deja que el llamante decida, en vez de
///   inventar una ruta que fallaría al escribir el primer byte.
///
/// Nunca lanza. Un fallo aquí no debe ser peor que el defecto que arregla.
Future<String?> androidDestination({
  MethodChannel? channel,
  bool? isAndroid,
}) async {
  if (!(isAndroid ?? Platform.isAndroid)) return null;
  try {
    final answer = await (channel ?? qyroPathsChannel)
        .invokeMethod<String>('destination');
    if (answer == null || answer.isEmpty) return null;
    return answer;
  } on MissingPluginException {
    return null;
  } on PlatformException {
    return null;
  }
}
