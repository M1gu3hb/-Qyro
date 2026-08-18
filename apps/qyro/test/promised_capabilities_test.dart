// The interface may not promise a capability the application does not have.
//
// QYR-0348, found in phase 10. A button carried `Icons.qr_code_scanner` and the
// label "Scan a code", and what it did was parse the text field above it. There
// is no camera in this application: no camera plugin in `pubspec.yaml`, no
// camera permission in the manifest, and no QR decoder anywhere in the tree. A
// person who taps a scanner icon and gets a text parser has been told something
// untrue by the same interface that phase 05 spent a day making honest.
//
// This is a textual guard and this project knows what those are worth
// (QYR-0304): it loses to any spelling it does not enumerate. It is here anyway
// because the failure it catches is cheap to make by accident -- reaching for
// the icon that *looks* right -- and because the second half of it is not
// textual at all: it asserts the absence of the dependency that would be needed
// to make the promise true. An icon can be renamed; a camera cannot be
// smuggled in without a package.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Icons that claim a camera.
///
/// Not every camera-ish icon Material ships -- that list would rot. These are
/// the ones somebody reaches for when they want to suggest "point this at
/// something", which is the promise the application cannot keep.
const iconsThatPromiseACamera = <String>[
  'Icons.qr_code_scanner',
  'Icons.qr_code_2', // still reads as "scan me" next to a pairing field
  'Icons.photo_camera',
  'Icons.camera_alt',
  'Icons.document_scanner',
  'Icons.center_focus_strong',
];

/// Packages that would make a camera real.
const packagesThatWouldProvideOne = <String>[
  'camera',
  'mobile_scanner',
  'qr_code_scanner',
  'flutter_barcode_scanner',
  'google_mlkit_barcode_scanning',
];

/// The source with its comments removed, so the guard reads code, not prose.
///
/// It failed on its first run against the comment that explains why it exists,
/// which is QYR-0328 in the other language: a check that cannot tell a mention
/// from a use. The Rust side walks the source and skips literals
/// (`production_source`); this is that walk in Dart. String literals are copied
/// through rather than dropped -- keeping them is the strict choice, and a `//`
/// inside one can no longer swallow the rest of its line.
String codeOnly(String source) {
  final out = StringBuffer();
  var i = 0;
  while (i < source.length) {
    if (source.startsWith('//', i)) {
      final end = source.indexOf('\n', i);
      i = end == -1 ? source.length : end;
      continue;
    }
    if (source.startsWith('/*', i)) {
      final end = source.indexOf('*/', i + 2);
      i = end == -1 ? source.length : end + 2;
      continue;
    }
    final quote = _quoteAt(source, i);
    if (quote != null) {
      final end = _endOfLiteral(source, i, quote);
      out.write(source.substring(i, end));
      i = end;
      continue;
    }
    out.write(source[i]);
    i++;
  }
  return out.toString();
}

/// The literal delimiter starting at [i], longest first so `'''` wins over `'`.
String? _quoteAt(String source, int i) {
  for (final quote in const ["'''", '"""', "'", '"']) {
    if (source.startsWith(quote, i)) return quote;
  }
  return null;
}

int _endOfLiteral(String source, int start, String quote) {
  var j = start + quote.length;
  while (j < source.length) {
    if (source[j] == r'\') {
      j += 2;
      continue;
    }
    if (source.startsWith(quote, j)) return j + quote.length;
    j++;
  }
  return source.length;
}

void main() {
  test('a scanner icon only appears where the camera is real', () {
    // **QYR-0348 al reves, y por eso esta guarda cambia en vez de irse.**
    //
    // Nacio prohibiendo el icono de escaner porque no habia camara: un boton con
    // `Icons.qr_code_scanner` sobre un parser de texto le dice a alguien algo
    // que no es verdad. Su propia razon decia como terminaba: *«o se va la
    // promesa, o llega la camara con su plugin, su permiso, su ADR y su fila en
    // el modelo de amenazas»*.
    //
    // En la fase 24B llegaron las cuatro. Asi que la guarda **no se debilita**:
    // deja de prohibir el icono y pasa a exigir que, si aparece, las cuatro
    // cosas esten. Atada a la capacidad es mas fuerte que atada a una lista.
    final offenders = <String>[];
    for (final file in Directory('lib')
        .listSync(recursive: true)
        .whereType<File>()
        .where((entry) => entry.path.endsWith('.dart'))
        // The generated localisation classes are data, not widgets.
        .where((entry) => !entry.path.contains('generated'))) {
      final source = codeOnly(file.readAsStringSync());
      for (final icon in iconsThatPromiseACamera) {
        if (source.contains(icon)) offenders.add('${file.path}: $icon');
      }
    }

    if (offenders.isEmpty) return;

    // Hay promesa. Entonces las cuatro piezas tienen que estar, y cada
    // `expect` nombra la que falta en vez de decir «algo falta».
    final manifest =
        File('android/app/src/main/AndroidManifest.xml').readAsStringSync();
    expect(
      manifest,
      contains('android.permission.CAMERA'),
      reason:
          'hay un icono de escaner y el manifest no pide CAMERA: $offenders',
    );

    final gradle = File('android/app/build.gradle.kts').readAsStringSync();
    expect(
      gradle,
      contains('androidx.camera:camera-core'),
      reason: 'hay un icono de escaner y nada en el grafo puede capturar',
    );

    expect(
      File('../../docs/adr/ADR-0048-el-ojo.md').existsSync(),
      isTrue,
      reason: 'hay un icono de escaner y ninguna ADR lo decidio',
    );

    final threats = File('../../THREAT_MODEL.md').readAsStringSync();
    expect(
      threats.toLowerCase(),
      contains('camera'),
      reason: 'hay un icono de escaner y el modelo de amenazas no menciona la '
          'camara. Una superficie de entrada nueva sin fila es una superficie '
          'que nadie ha pensado.',
    );
  });

  test('and no pub.dev camera package sneaks in', () {
    final pubspec = File('pubspec.yaml').readAsStringSync();
    final lock = File('pubspec.lock').readAsStringSync();

    for (final package in packagesThatWouldProvideOne) {
      expect(
        codeOnly(pubspec).contains('\n  $package:'),
        isFalse,
        reason:
            '$package is a direct dependency now. If the application really '
            'grew a camera, this test is the wrong thing to delete: update the '
            'threat model first, because a camera is a new adversary.',
      );
      expect(
        lock.contains('\n  $package:\n'),
        isFalse,
        reason:
            '$package arrived transitively. Nothing here should pull one in, '
            'so find out what did.',
      );
    }
  });

  test('the guard tells a use from a mention', () {
    // The control, in both directions. A guard nobody has watched fail is a
    // guard nobody knows the shape of, and three of this project's defects
    // were checks that could not see the thing they were for.
    const mention = '// once carried Icons.qr_code_scanner, and does not now\n';
    const use = 'IconButton(icon: Icon(Icons.qr_code_scanner))';

    expect(codeOnly(mention).contains('Icons.qr_code_scanner'), isFalse);
    expect(codeOnly(use).contains('Icons.qr_code_scanner'), isTrue);

    // And a `//` inside a string literal is not the start of a comment.
    const inString = "final url = 'https://example.invalid'; Icons.camera_alt;";
    expect(codeOnly(inString).contains('Icons.camera_alt'), isTrue);
  });
}
