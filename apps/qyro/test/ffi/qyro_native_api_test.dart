import 'dart:io';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_native_api.dart';

void main() {
  group('library names', () {
    test('maps supported operating systems', () {
      expect(
        QyroNativeApi.libraryNameForOperatingSystem('android'),
        'libqyro_ffi.so',
      );
      expect(
        QyroNativeApi.libraryNameForOperatingSystem('linux'),
        'libqyro_ffi.so',
      );
      expect(
        QyroNativeApi.libraryNameForOperatingSystem('windows'),
        'qyro_ffi.dll',
      );
      expect(
        QyroNativeApi.libraryNameForOperatingSystem('macos'),
        'libqyro_ffi.dylib',
      );
    });

    test('missing library becomes a typed sanitized failure', () {
      expect(
        () => QyroNativeApi.open('missing-qyro-native-library.invalid'),
        throwsA(
          isA<QyroLibraryNotFoundFailure>()
              .having(
                (failure) => failure.diagnostic,
                'diagnostic',
                contains('missing-qyro-native-library.invalid'),
              )
              .having(
                (failure) => failure.diagnostic,
                'diagnostic',
                isNot(contains('DynamicLibrary')),
              ),
        ),
      );
    });
  });

  group('typed ABI failures', () {
    test('missing pointer symbol is typed and sanitized', () {
      final resolver = _FakeResolver()
        ..failureBySymbol['qyro_protocol_version_ptr'] =
            StateError('SECRET raw loader output');

      expect(
        () => QyroNativeApi.fromResolver(resolver, libraryContext: 'fake.dll'),
        throwsA(
          isA<QyroSymbolNotFoundFailure>()
              .having(
                (failure) => failure.symbol,
                'symbol',
                'qyro_protocol_version_ptr',
              )
              .having(
                (failure) => failure.diagnostic,
                'diagnostic',
                isNot(contains('SECRET')),
              ),
        ),
      );
    });

    test('missing length symbol is typed', () {
      final resolver = _FakeResolver()
        ..failureBySymbol['qyro_protocol_version_len'] = ArgumentError();

      expect(
        () => QyroNativeApi.fromResolver(resolver),
        throwsA(
          isA<QyroSymbolNotFoundFailure>().having(
            (failure) => failure.symbol,
            'symbol',
            'qyro_protocol_version_len',
          ),
        ),
      );
    });

    test('null pointer is typed', () {
      final api = QyroNativeApi.fromResolver(_FakeResolver(nullPointer: true));

      expect(
        api.protocolVersion,
        throwsA(isA<QyroNullPointerFailure>()),
      );
    });

    test('invalid lengths are typed', () {
      for (final length in <int>[0, 65]) {
        final api = QyroNativeApi.fromResolver(
          _FakeResolver(length: length),
        );

        expect(
          api.protocolVersion,
          throwsA(
            isA<QyroInvalidLengthFailure>().having(
              (failure) => failure.length,
              'length',
              length,
            ),
          ),
        );
      }
    });

    test('invalid UTF-8 is typed', () {
      final api = QyroNativeApi.fromResolver(
        _FakeResolver(bytes: Uint8List.fromList(<int>[0xC3, 0x28])),
      );

      expect(
        api.protocolVersion,
        throwsA(isA<QyroInvalidUtf8Failure>()),
      );
    });

    test('incompatible protocol version is typed', () {
      final api = QyroNativeApi.fromResolver(
        _FakeResolver(bytes: Uint8List.fromList('QYRO/2'.codeUnits)),
      );

      expect(
        api.protocolVersion,
        throwsA(
          isA<QyroIncompatibleVersionFailure>()
              .having((failure) => failure.actual, 'actual', 'QYRO/2')
              .having((failure) => failure.expected, 'expected', 'QYRO/1'),
        ),
      );
    });

    test('valid bindings return QYRO/1', () {
      final api = QyroNativeApi.fromResolver(_FakeResolver());

      expect(api.protocolVersion(), 'QYRO/1');
    });
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

final class _FakeResolver implements QyroNativeSymbolResolver {
  _FakeResolver({
    this.length = 6,
    this.nullPointer = false,
    Uint8List? bytes,
  }) : bytes = bytes ?? Uint8List.fromList('QYRO/1'.codeUnits);

  final int length;
  final bool nullPointer;
  final Uint8List bytes;
  final Map<String, Object> failureBySymbol = <String, Object>{};

  @override
  ProtocolVersionBytesReader lookupBytes(String symbol) {
    _throwFor(symbol);
    return (length) => nullPointer ? null : bytes;
  }

  @override
  ProtocolVersionLengthReader lookupLength(String symbol) {
    _throwFor(symbol);
    return () => length;
  }

  void _throwFor(String symbol) {
    final failure = failureBySymbol[symbol];
    if (failure != null) {
      throw failure;
    }
  }
}
