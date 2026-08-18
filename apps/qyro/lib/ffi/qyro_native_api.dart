import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import '../startup/native_bridge.dart';

typedef ProtocolVersionBytesReader = Uint8List? Function(int length);
typedef ProtocolVersionLengthReader = int Function();

abstract interface class QyroNativeSymbolResolver {
  ProtocolVersionBytesReader lookupBytes(String symbol);

  ProtocolVersionLengthReader lookupLength(String symbol);
}

sealed class QyroNativeFailure implements Exception {
  const QyroNativeFailure({
    required this.code,
    required this.diagnostic,
  });

  final String code;
  final String diagnostic;

  String get userMessageKey => 'nativeBridgeUnavailable';

  @override
  String toString() => '$runtimeType($code): $diagnostic';
}

final class QyroLibraryNotFoundFailure extends QyroNativeFailure {
  const QyroLibraryNotFoundFailure(String library)
      : super(
          code: 'library_not_found',
          diagnostic: 'Native library not found: $library',
        );
}

final class QyroSymbolNotFoundFailure extends QyroNativeFailure {
  const QyroSymbolNotFoundFailure({
    required this.symbol,
    required String library,
  }) : super(
          code: 'symbol_not_found',
          diagnostic: 'Required symbol $symbol was not found in $library',
        );

  final String symbol;
}

final class QyroNullPointerFailure extends QyroNativeFailure {
  const QyroNullPointerFailure()
      : super(
          code: 'null_pointer',
          diagnostic: 'Native protocol version pointer was null',
        );
}

final class QyroInvalidLengthFailure extends QyroNativeFailure {
  const QyroInvalidLengthFailure(this.length)
      : super(
          code: 'invalid_length',
          diagnostic: 'Native protocol version length was $length',
        );

  final int length;
}

final class QyroInvalidUtf8Failure extends QyroNativeFailure {
  const QyroInvalidUtf8Failure()
      : super(
          code: 'invalid_utf8',
          diagnostic: 'Native protocol version was not valid UTF-8',
        );
}

final class QyroIncompatibleVersionFailure extends QyroNativeFailure {
  const QyroIncompatibleVersionFailure({
    required this.actual,
    this.expected = QyroNativeApi.supportedProtocolVersion,
  }) : super(
          code: 'incompatible_version',
          diagnostic: 'Expected $expected but native bridge returned $actual',
        );

  final String actual;
  final String expected;
}

typedef _ProtocolVersionPointerNative = Pointer<Uint8> Function();
typedef _ProtocolVersionPointerDart = Pointer<Uint8> Function();
typedef _ProtocolVersionLengthNative = IntPtr Function();
typedef _ProtocolVersionLengthDart = int Function();

final class _DynamicLibraryResolver implements QyroNativeSymbolResolver {
  const _DynamicLibraryResolver(this.library);

  final DynamicLibrary library;

  @override
  ProtocolVersionBytesReader lookupBytes(String symbol) {
    final readPointer = library.lookupFunction<_ProtocolVersionPointerNative,
        _ProtocolVersionPointerDart>(symbol);
    return (length) {
      final pointer = readPointer();
      if (pointer.address == 0) {
        return null;
      }
      return Uint8List.fromList(pointer.asTypedList(length));
    };
  }

  @override
  ProtocolVersionLengthReader lookupLength(String symbol) {
    return library.lookupFunction<_ProtocolVersionLengthNative,
        _ProtocolVersionLengthDart>(symbol);
  }
}

final class QyroNativeApi implements NativeBridge {
  QyroNativeApi._({
    required ProtocolVersionBytesReader readBytes,
    required ProtocolVersionLengthReader readLength,
  })  : _readBytes = readBytes,
        _readLength = readLength;

  factory QyroNativeApi.fromResolver(
    QyroNativeSymbolResolver resolver, {
    String libraryContext = 'native process',
  }) {
    final context = _sanitizeLibraryContext(libraryContext);
    late final ProtocolVersionBytesReader readBytes;
    late final ProtocolVersionLengthReader readLength;

    try {
      readBytes = resolver.lookupBytes('qyro_protocol_version_ptr');
    } catch (_) {
      throw QyroSymbolNotFoundFailure(
        symbol: 'qyro_protocol_version_ptr',
        library: context,
      );
    }
    try {
      readLength = resolver.lookupLength('qyro_protocol_version_len');
    } catch (_) {
      throw QyroSymbolNotFoundFailure(
        symbol: 'qyro_protocol_version_len',
        library: context,
      );
    }

    return QyroNativeApi._(
      readBytes: readBytes,
      readLength: readLength,
    );
  }

  factory QyroNativeApi.open(String path) {
    final context = _sanitizeLibraryContext(path);
    try {
      final library = DynamicLibrary.open(path);
      return QyroNativeApi.fromResolver(
        _DynamicLibraryResolver(library),
        libraryContext: context,
      );
    } on QyroNativeFailure {
      rethrow;
    } catch (_) {
      throw QyroLibraryNotFoundFailure(context);
    }
  }

  factory QyroNativeApi.openDefault() {
    final override = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
    if (override != null && override.isNotEmpty) {
      return QyroNativeApi.open(override);
    }
    if (Platform.isIOS) {
      return QyroNativeApi.fromResolver(
        _DynamicLibraryResolver(DynamicLibrary.process()),
      );
    }
    return QyroNativeApi.open(
      libraryNameForOperatingSystem(Platform.operatingSystem),
    );
  }

  static const supportedProtocolVersion = 'QYRO/1';
  static const _maximumProtocolVersionBytes = 64;

  final ProtocolVersionBytesReader _readBytes;
  final ProtocolVersionLengthReader _readLength;

  static String libraryNameForOperatingSystem(String operatingSystem) {
    return switch (operatingSystem) {
      'android' || 'linux' => 'libqyro_ffi.so',
      'windows' => 'qyro_ffi.dll',
      'macos' => 'libqyro_ffi.dylib',
      _ => throw UnsupportedError(
          'qyro_ffi does not support $operatingSystem as a dynamic library.',
        ),
    };
  }

  @override
  String protocolVersion() {
    final length = _readLength();
    if (length <= 0 || length > _maximumProtocolVersionBytes) {
      throw QyroInvalidLengthFailure(length);
    }

    final bytes = _readBytes(length);
    if (bytes == null) {
      throw const QyroNullPointerFailure();
    }

    late final String version;
    try {
      version = utf8.decode(bytes, allowMalformed: false);
    } on FormatException {
      throw const QyroInvalidUtf8Failure();
    }
    if (version != supportedProtocolVersion) {
      throw QyroIncompatibleVersionFailure(actual: version);
    }
    return version;
  }

  static String _sanitizeLibraryContext(String value) {
    final normalized = value.replaceAll(r'\', '/');
    final segments = normalized.split('/');
    final basename = segments.isEmpty ? '' : segments.last;
    return basename.isEmpty ? 'native process' : basename;
  }
}
