// Los códigos de error de Rust y su espejo en Dart, contados uno a uno.
//
// **Un código que Dart no conoce no desaparece: sale con el nombre de otro.**
// `_kindOf` tiene un comodín, así que un error nuevo del motor llega a la
// pantalla disfrazado del último caso de la lista — y una persona lee «error de
// integridad» donde el motor dijo «has elegido demasiados archivos».
//
// Esto pasó de verdad: la fase 22 añadió `QYRO_ERR_TOO_MANY_FILES = -14` con su
// mensaje, y el espejo de Dart se quedó en -13. Lo encontró un barrido leyendo
// los dos lados, no una prueba.
//
// Esta prueba **lee `abi.rs`** en vez de llevar su propia lista: una lista a mano
// se separa del original el día que alguien añade un código, que es exactamente
// el fallo que existe para impedir.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_session_api.dart';

/// Los `pub const QYRO_ERR_* : i32 = -N;` de la frontera C.
Map<String, int> _rustCodes() {
  final source =
      File('../../rust/crates/qyro_ffi/src/abi.rs').readAsStringSync();
  final pattern = RegExp(
    r'pub const (QYRO_ERR_[A-Z_]+): i32 = (-?\d+);',
    multiLine: true,
  );
  return <String, int>{
    for (final match in pattern.allMatches(source))
      match.group(1)!: int.parse(match.group(2)!),
  };
}

void main() {
  test('Dart conoce todos los codigos de error que Rust define', () {
    final rust = _rustCodes();

    // Que el lector funcione, antes de creerle. Un patrón mal escrito
    // encontraría cero códigos y esta prueba pasaría diciendo que todo cuadra.
    expect(
      rust.length,
      greaterThanOrEqualTo(10),
      reason: 'el lector de abi.rs encontro ${rust.length} codigos, asi que se '
          'rompio el patron y no el espejo',
    );
    expect(rust['QYRO_ERR_BAD_ARGUMENT'], -6);

    final faltan = <String>[];
    for (final entry in rust.entries) {
      // `QYRO_ERR_PANIC` y `QYRO_ERR_UNKNOWN` también tienen que estar: los dos
      // llegan a una persona.
      if (!QyroCode.names.containsKey(entry.value)) {
        faltan.add('${entry.key} = ${entry.value}');
      }
    }

    expect(
      faltan,
      isEmpty,
      reason: 'Rust define codigos que Dart no conoce, asi que salen con el '
          'nombre de otro error por el comodin de _kindOf: $faltan',
    );
  });

  test('y no al reves: Dart no inventa codigos que Rust no define', () {
    // El control. Un espejo que anadiera nombres por su cuenta pasaria la
    // prueba de arriba y describiria estados que el motor no produce.
    final rust = _rustCodes().values.toSet()..add(0); // 0 es `ok`, no un error.
    final sobran = QyroCode.names.keys.where((code) => !rust.contains(code));

    expect(
      sobran,
      isEmpty,
      reason: 'Dart nombra codigos que la frontera C no define: $sobran',
    );
  });
}
