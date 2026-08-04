import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_native_api.dart';

void main() {
  test('maps supported operating systems to native library names', () {
    expect(QyroNativeApi.libraryNameForOperatingSystem('android'), 'libqyro_ffi.so');
    expect(QyroNativeApi.libraryNameForOperatingSystem('linux'), 'libqyro_ffi.so');
    expect(QyroNativeApi.libraryNameForOperatingSystem('windows'), 'qyro_ffi.dll');
    expect(QyroNativeApi.libraryNameForOperatingSystem('macos'), 'libqyro_ffi.dylib');
  });

  final libraryPath = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
  test(
    'reads QYRO/1 from the compiled Rust library',
    () {
      final api = QyroNativeApi.open(libraryPath!);
      expect(api.protocolVersion(), 'QYRO/1');
    },
    skip: libraryPath == null
        ? 'Set QYRO_FFI_LIBRARY_PATH to run the native ABI integration test.'
        : false,
  );
}
