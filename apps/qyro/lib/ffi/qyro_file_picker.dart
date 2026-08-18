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
//
// The Windows half is `file_selector_windows`, the endorsed Windows
// implementation of that same federated plugin, depended on directly rather
// than through the `file_selector` umbrella -- the umbrella would drag
// `file_selector_android` into the APK, which is the one package this design
// exists to avoid. ADR-0034, amendment 1.

import 'dart:io';

import 'package:file_selector_windows/file_selector_windows.dart';
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
/// Two platforms ship in v1.0 and both are named here. Everything else is
/// refused **by name**, including Linux and macOS: an unsupported platform must
/// not return an empty list, because an empty list is what a cancelled picker
/// returns and the caller cannot tell the two apart. iOS is deferred by
/// ADR-0039; Linux and macOS never had a picker and are not v1.0 targets
/// (ADR-0034, amendment 1).
///
/// [operatingSystem] defaults to the host's, and exists so the routing itself
/// can be tested from any machine. A test that can only run on the platform it
/// is asserting about is a test that never runs.
QyroFilePicker pickerForPlatform({String? operatingSystem}) {
  final os = operatingSystem ?? Platform.operatingSystem;
  return switch (os) {
    'android' => const QyroAndroidFilePicker(),
    'windows' => const QyroWindowsFilePicker(),
    _ => throw UnsupportedError(
        'Qyro has no file picker for $os. v1.0 ships Android and Windows; iOS '
        'is deferred by ADR-0039 and the other desktops were never targets.',
      ),
  };
}

/// Windows: a real path, from the system dialog.
///
/// The dialog is `file_selector_windows` (flutter.dev, BSD-3). Hand-written
/// `IFileOpenDialog` is a ~29-slot vtable whose order Microsoft does not publish
/// on the web, and a shifted slot is silent undefined behaviour rather than a
/// link error (ADR-0034 §4.2).
///
/// [openPaths] is the seam. It defaults to the real dialog, so production needs
/// no wiring; a test replaces it, because a modal dialog cannot be driven from
/// `flutter test` and a picker that can only be exercised by hand is a picker
/// with no tests at all.
final class QyroWindowsFilePicker implements QyroFilePicker {
  const QyroWindowsFilePicker({this.openPaths = _systemDialog});

  /// Returns absolute paths, or an empty list if the person cancelled.
  final Future<List<String>> Function() openPaths;

  static Future<List<String>> _systemDialog() async {
    final chosen = await FileSelectorWindows().openFiles();
    return chosen.map((file) => file.path).toList(growable: false);
  }

  @override
  Future<List<QyroPicked>> pickFiles() async {
    final paths = await openPaths();
    return paths.map((path) {
      final file = File(path);
      return QyroPickedPath(
        path: path,
        name: leafName(path),
        size: file.existsSync() ? file.lengthSync() : -1,
      );
    }).toList(growable: false);
  }
}

/// The last segment of [path], cutting on either separator.
///
/// Both, and not `Platform.pathSeparator`, for two reasons that are really one:
/// a Windows path handled on a Linux CI host would come back whole, and the
/// name is what travels to the receiver — a name that is still a path is how a
/// receiver ends up writing outside its destination. The manifest layer refuses
/// that too; this is the layer that should never have produced it.
String leafName(String path) {
  final cut = <int>[path.lastIndexOf('/'), path.lastIndexOf(r'\')]
      .reduce((a, b) => a > b ? a : b);
  final leaf = cut < 0 ? path : path.substring(cut + 1);
  return leaf.isEmpty ? 'file' : leaf;
}
