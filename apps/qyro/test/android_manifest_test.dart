// Qyro declares no storage permission, checked on the **merged** manifest.
//
// ADR-0034 §4. The Storage Access Framework's own documentation is the reason
// this can be asserted at all: "Because the user is involved in selecting the
// files or directories that your app can access, this mechanism doesn't require
// any system permissions."
//
// The merged manifest and not ours, because a Flutter plugin can add a
// permission without it ever appearing in the file this repository writes. A
// test over the source manifest would pass while the shipped APK asked for
// storage — which is the exact shape of defect this project keeps finding: a
// measurement that cannot see the failure it is for.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Permissions that would mean the design went wrong.
///
/// Not a complete list of Android permissions — a complete list would rot. These
/// are the ones SAF exists to make unnecessary, so any of them appearing is a
/// signal that somebody reached for the old way.
const _forbidden = <String>[
  'android.permission.READ_EXTERNAL_STORAGE',
  'android.permission.WRITE_EXTERNAL_STORAGE',
  'android.permission.MANAGE_EXTERNAL_STORAGE',
  'android.permission.READ_MEDIA_IMAGES',
  'android.permission.READ_MEDIA_VIDEO',
  'android.permission.READ_MEDIA_AUDIO',
];

/// Where Gradle writes the merged manifest, newest build variant first.
const _mergedCandidates = <String>[
  'android/app/build/intermediates/merged_manifests/debug/processDebugManifest/AndroidManifest.xml',
  'android/app/build/intermediates/merged_manifests/debug/AndroidManifest.xml',
  'android/app/build/intermediates/merged_manifest/debug/AndroidManifest.xml',
];

File? _mergedManifest() {
  for (final candidate in _mergedCandidates) {
    final file = File(candidate);
    if (file.existsSync()) return file;
  }
  // A wider sweep, because the intermediates path has moved between Android
  // Gradle Plugin versions and pinning one is how this test would quietly stop
  // reading anything.
  final root = Directory('android/app/build/intermediates');
  if (!root.existsSync()) return null;
  for (final entry in root.listSync(recursive: true)) {
    if (entry is File &&
        entry.path.endsWith('AndroidManifest.xml') &&
        entry.path.contains('merged_manifest')) {
      return entry;
    }
  }
  return null;
}

void main() {
  test('the source manifest declares no storage permission', () {
    final source = File('android/app/src/main/AndroidManifest.xml');
    expect(
      source.existsSync(),
      isTrue,
      reason: 'the manifest this repository writes must exist to be checked',
    );
    final text = source.readAsStringSync();
    for (final permission in _forbidden) {
      expect(
        text.contains(permission),
        isFalse,
        reason: '$permission is declared. SAF needs none of these; if one is '
            'genuinely required the design took a wrong turn (ADR-0034 §4)',
      );
    }
  });

  test('the merged manifest declares no storage permission', () {
    final merged = _mergedManifest();
    if (merged == null) {
      // Skipped rather than passed, and said out loud. A green tick for a file
      // that was never read is the failure mode this whole test exists to
      // avoid, so it must not be able to look like success.
      markTestSkipped(
        'no merged manifest found under android/app/build/intermediates. '
        'Run `flutter build apk --debug` first; on this machine that needs '
        'Developer Mode for plugin symlinks (QYR-0324), so CI is where this '
        'assertion actually runs.',
      );
      return;
    }

    final text = merged.readAsStringSync();
    expect(
      text.contains('<manifest'),
      isTrue,
      reason: 'the file found at ${merged.path} is not a manifest',
    );
    for (final permission in _forbidden) {
      expect(
        text.contains(permission),
        isFalse,
        reason:
            '$permission reached the merged manifest at ${merged.path}. Some '
            'dependency asked for it; ours does not (ADR-0034 §4)',
      );
    }
  });
}
