// El cerrojo del puerto fijo, compartido por las pruebas que lo ligan.
//
// No termina en `_test.dart` a propósito: el corredor sólo recoge esos, así que
// esto es una biblioteca de pruebas y no una prueba vacía.

import 'dart:io';

import 'package:qyro/transfer/transfer_service.dart';

/// Toma en exclusiva el **puerto fijo** antes de ligarlo.
///
/// ADR-0041 §3 fija el 49517 a propósito: un puerto que se mueve pierde el
/// permiso del cortafuegos y la predicción del código, y los pierde **sin
/// avisar**. Eso, que es correcto para el producto, tiene una consecuencia en
/// las pruebas: **dos no pueden tenerlo a la vez**.
///
/// Y `flutter test` corre los archivos **en paralelo**, así que
/// `two_process_pairing_test` y `native_transfer_service_test` se lo quitaban el
/// uno al otro. Quien perdía no fallaba diciendo «el puerto está ocupado»: el
/// receptor emitía `QyroFailed(portUnavailable)`, nadie miraba ese estado, y lo
/// que se veía era al segundo proceso saliendo con `ConnectionRefused` — un
/// síntoma a dos saltos de su causa.
///
/// Un cerrojo de archivo, que es el mecanismo que sí cruza procesos. El sistema
/// lo suelta solo si la prueba se cae, así que no hay forma de dejarlo tomado.
RandomAccessFile lockTheFixedPort() {
  final path = '${Directory.systemTemp.path}${Platform.pathSeparator}'
      'qyro-puerto-$qyroDefaultPort.lock';
  final handle = File(path).openSync(mode: FileMode.write);
  handle.lockSync(FileLock.blockingExclusive);
  return handle;
}
