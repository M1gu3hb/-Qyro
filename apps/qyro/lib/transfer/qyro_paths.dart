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
    final answer =
        await (channel ?? qyroPathsChannel).invokeMethod<String>('destination');
    if (answer == null || answer.isEmpty) return null;
    return answer;
  } on MissingPluginException {
    return null;
  } on PlatformException {
    return null;
  }
}

/// Dónde escribir el blob de identidad de este aparato, o `null` para el de
/// siempre.
///
/// **QYR-0376, y era el P0 más grande.** `defaultIdentityPath()` devolvía en
/// Android `Directory.current.path + '/identity.qyro'`, o sea
/// **`/identity.qyro`**: la raíz del sistema. Escribir ahí falla, así que
/// `openIdentity()` fallaba, así que **toda sesión contestaba
/// `identity_unreadable`** (ADR-0040). Eso no es medio producto como el destino:
/// es el producto entero, porque sin identidad no hay handshake, ni huella que
/// enseñar, ni código de emparejamiento, en ninguna dirección.
///
/// Kotlin contesta con `getNoBackupFilesDir()`, que es almacenamiento interno
/// —privado por el sandbox de UID, que es la protección que `THREAT_MODEL.md`
/// nombra para esta semilla— y que el sistema nunca copia a una nube.
///
/// Los mismos tres fallos que [androidDestination], y por la misma razón:
/// devuelve `null` y nunca lanza.
Future<String?> androidIdentityPath({
  MethodChannel? channel,
  bool? isAndroid,
}) async {
  if (!(isAndroid ?? Platform.isAndroid)) return null;
  try {
    final answer =
        await (channel ?? qyroPathsChannel).invokeMethod<String>('identity');
    if (answer == null || answer.isEmpty) return null;
    return answer;
  } on MissingPluginException {
    return null;
  } on PlatformException {
    return null;
  }
}
