import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

typedef _ProtocolVersionPointerNative = Pointer<Uint8> Function();
typedef _ProtocolVersionPointerDart = Pointer<Uint8> Function();
typedef _ProtocolVersionLengthNative = IntPtr Function();
typedef _ProtocolVersionLengthDart = int Function();

class QyroNativeApi {
  QyroNativeApi._(DynamicLibrary library)
    : _protocolVersionPointer = library
          .lookupFunction<
            _ProtocolVersionPointerNative,
            _ProtocolVersionPointerDart
          >('qyro_protocol_version_ptr'),
      _protocolVersionLength = library
          .lookupFunction<
            _ProtocolVersionLengthNative,
            _ProtocolVersionLengthDart
          >('qyro_protocol_version_len');

  factory QyroNativeApi.open(String path) {
    return QyroNativeApi._(DynamicLibrary.open(path));
  }

  factory QyroNativeApi.openDefault() {
    final override = Platform.environment['QYRO_FFI_LIBRARY_PATH'];
    if (override != null && override.isNotEmpty) {
      return QyroNativeApi.open(override);
    }
    if (Platform.isIOS) {
      return QyroNativeApi._(DynamicLibrary.process());
    }
    return QyroNativeApi.open(
      libraryNameForOperatingSystem(Platform.operatingSystem),
    );
  }

  static const _maximumProtocolVersionBytes = 64;

  final _ProtocolVersionPointerDart _protocolVersionPointer;
  final _ProtocolVersionLengthDart _protocolVersionLength;

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

  String protocolVersion() {
    final length = _protocolVersionLength();
    if (length <= 0 || length > _maximumProtocolVersionBytes) {
      throw StateError('qyro_ffi returned an invalid protocol version length.');
    }

    final pointer = _protocolVersionPointer();
    if (pointer.address == 0) {
      throw StateError('qyro_ffi returned a null protocol version pointer.');
    }

    return utf8.decode(pointer.asTypedList(length), allowMalformed: false);
  }
}
