// What the person chose, and how it reaches Rust.
//
// Specification: docs/adr/ADR-0034-file-selection.md.
//
// Two shapes, because the platforms genuinely differ and pretending otherwise
// costs a copy of every file:
//
//   Android — the Storage Access Framework hands out a content:// URI and a
//             descriptor, never a path. A descriptor crosses.
//   Windows — a desktop picker hands out a real path. A path crosses.
//
// The Android half is a MethodChannel written in this repository rather than a
// package, because `file_selector_android` copies the whole file into the app
// cache before Dart sees it (QYR-0323).

import 'dart:io';

import 'package:flutter/services.dart';

/// One thing the person picked.
sealed class QyroPicked {
  const QyroPicked({required this.name, required this.size});

  /// The name that travels to the receiver. Never a path.
  final String name;

  /// Bytes, as the platform reported them. `-1` when it would not say.
  final int size;
}

/// A file Rust will open by descriptor. Android.
final class QyroPickedDescriptor extends QyroPicked {
  const QyroPickedDescriptor({
    required this.descriptor,
    required super.name,
    required super.size,
  });

  /// The raw file descriptor, already detached from its `ParcelFileDescriptor`.
  ///
  /// **Dart does not own this.** `detachFd()` gave up the Kotlin side's claim
  /// and Rust's `File::from_raw_fd` takes it on entry. Nothing here may close
  /// it, read it, or use it twice.
  final int descriptor;
}

/// A file Rust will open by path. Windows.
final class QyroPickedPath extends QyroPicked {
  const QyroPickedPath({
    required this.path,
    required super.name,
    required super.size,
  });

  final String path;
}

/// The picker, per platform.
abstract interface class QyroFilePicker {
  /// Opens the system picker. An empty list means the person cancelled, which
  /// is not an error.
  Future<List<QyroPicked>> pickFiles();
}

/// Android: the Storage Access Framework, over our own channel.
final class QyroAndroidFilePicker implements QyroFilePicker {
  const QyroAndroidFilePicker([this._channel = _defaultChannel]);

  static const _defaultChannel = MethodChannel('dev.qyro/file_picker');
  final MethodChannel _channel;

  @override
  Future<List<QyroPicked>> pickFiles() async {
    final raw = await _channel.invokeListMethod<Map<Object?, Object?>>(
      'pickFiles',
    );
    if (raw == null) {
      return const <QyroPicked>[];
    }
    return raw.map((entry) {
      return QyroPickedDescriptor(
        descriptor: (entry['fd'] as num).toInt(),
        name: entry['name'] as String? ?? 'file',
        size: (entry['size'] as num?)?.toInt() ?? -1,
      );
    }).toList(growable: false);
  }
}

/// The picker this platform uses.
///
/// Windows returns a path-shaped picker; the desktop dialog has no content://
/// problem. iOS is out of v1.0 (ADR-0039) and is refused by name rather than
/// silently returning nothing, because an empty list reads as "the person
/// cancelled".
QyroFilePicker pickerForPlatform() {
  if (Platform.isAndroid) {
    return const QyroAndroidFilePicker();
  }
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    return const QyroDesktopFilePicker();
  }
  throw UnsupportedError(
    'Qyro has no file picker for ${Platform.operatingSystem}. iOS is deferred '
    'to v1.1 by ADR-0039.',
  );
}

/// Desktop: a real path, from a real dialog.
///
/// The dialog itself is `file_selector` (flutter.dev, BSD-3). Hand-written
/// `IFileOpenDialog` is a ~29-slot vtable whose order Microsoft does not publish
/// on the web, and a shifted slot is silent undefined behaviour rather than a
/// link error.
///
/// Injected rather than called directly so the transfer path can be tested
/// without a dialog: [openPaths] is what a test replaces.
final class QyroDesktopFilePicker implements QyroFilePicker {
  const QyroDesktopFilePicker({this.openPaths});

  /// Returns absolute paths, or an empty list if the person cancelled.
  final Future<List<String>> Function()? openPaths;

  @override
  Future<List<QyroPicked>> pickFiles() async {
    final open = openPaths;
    if (open == null) {
      throw UnsupportedError(
        'QyroDesktopFilePicker needs an openPaths callback until the '
        'file_selector dependency is wired in ADR-0034 step 3.',
      );
    }
    final paths = await open();
    return paths.map((path) {
      final file = File(path);
      return QyroPickedPath(
        path: path,
        name: path.split(Platform.pathSeparator).last,
        size: file.existsSync() ? file.lengthSync() : -1,
      );
    }).toList(growable: false);
  }
}
