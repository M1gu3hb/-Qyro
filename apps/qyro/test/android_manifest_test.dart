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
  // ADR-0035 section 7 and phase 04b criterion 7. `NsdManager` with
  // `FLAG_SHOW_PICKER` exists precisely so this one is never needed: the person
  // picking a service *is* the grant, and it survives a reboot. If this ever
  // appears, the discovery design lost its whole advantage and became a runtime
  // permission dialog like everyone else's.
  'android.permission.ACCESS_LOCAL_NETWORK',
];

/// Removes XML comments before looking for a permission.
///
/// The source manifest *names* `ACCESS_LOCAL_NETWORK` in a comment explaining
/// why it is absent, and a substring search over the raw file would read that
/// explanation as the thing it explains. Stripping comments is what makes the
/// check about declarations rather than about prose.
String _withoutComments(String xml) => xml.replaceAll(
      RegExp(r'<!--.*?-->', dotAll: true),
      '',
    );

/// Where the intermediates live, most likely first.
///
/// **`build/app/`, not `android/app/build/`.** Flutter's Gradle plugin moves the
/// build directory out of the Android project — `rootProject.buildDir` becomes
/// `../build` and each module gets `build/<module>` — so nothing is ever written
/// under `android/app/build/`. This list used to name only that path, which is
/// why the assertion below had never once read a file. The Android path stays
/// second because a plain Gradle invocation, without Flutter, does use it.
const _intermediateRoots = <String>[
  'build/app/intermediates',
  'android/app/build/intermediates',
];

/// The exact paths, newest Android Gradle Plugin layout first.
const _mergedCandidates = <String>[
  'merged_manifests/debug/processDebugMainManifest/AndroidManifest.xml',
  'merged_manifests/debug/processDebugManifest/AndroidManifest.xml',
  'merged_manifests/debug/AndroidManifest.xml',
  'merged_manifest/debug/AndroidManifest.xml',
];

File? _mergedManifest() {
  for (final root in _intermediateRoots) {
    for (final candidate in _mergedCandidates) {
      final file = File('$root/$candidate');
      if (file.existsSync()) return file;
    }
  }
  // A wider sweep, because the intermediates path has moved between Android
  // Gradle Plugin versions and pinning one is how this test would quietly stop
  // reading anything.
  for (final root in _intermediateRoots) {
    final directory = Directory(root);
    if (!directory.existsSync()) continue;
    for (final entry in directory.listSync(recursive: true)) {
      if (entry is File &&
          entry.path.endsWith('AndroidManifest.xml') &&
          entry.path.contains('merged_manifest')) {
        return entry;
      }
    }
  }
  return null;
}

/// Everything the sweep looked at, for a failure message that can be acted on.
///
/// Without this, «none was found» is a dead end: it does not say whether the
/// build directory is missing, empty, or full of manifests under a name this
/// test does not recognise.
String _whatTheSweepSaw() {
  final report = StringBuffer();
  for (final root in _intermediateRoots) {
    final directory = Directory(root);
    if (!directory.existsSync()) {
      report.writeln('  $root: does not exist');
      continue;
    }
    final manifests = directory
        .listSync(recursive: true)
        .whereType<File>()
        .where((file) => file.path.endsWith('AndroidManifest.xml'))
        .map((file) => file.path)
        .take(20)
        .toList();
    report.writeln('  $root: ${manifests.length} manifest(s)');
    for (final path in manifests) {
      report.writeln('    $path');
    }
  }
  return report.toString();
}

void main() {
  test('the source manifest declares no storage permission', () {
    final source = File('android/app/src/main/AndroidManifest.xml');
    expect(
      source.existsSync(),
      isTrue,
      reason: 'the manifest this repository writes must exist to be checked',
    );
    final text = _withoutComments(source.readAsStringSync());
    // The stripping works: the raw file does mention one of these and the
    // stripped one does not, so a comment can never satisfy or break this.
    expect(
      source.readAsStringSync(),
      contains('ACCESS_LOCAL_NETWORK'),
      reason: 'the manifest no longer explains why the permission is absent; '
          'if the explanation went, check that the permission did not arrive',
    );
    expect(text, isNot(contains('ACCESS_LOCAL_NETWORK')));
    for (final permission in _forbidden) {
      expect(
        text.contains(permission),
        isFalse,
        reason: '$permission is declared. SAF needs none of these; if one is '
            'genuinely required the design took a wrong turn (ADR-0034 §4)',
      );
    }
  });

  test(
      'the manifest declares exactly one permission, and it is the multicast one',
      () {
    // Not «no permissions»: discovery needs `CHANGE_WIFI_MULTICAST_STATE`,
    // which is a normal permission granted at install and is what stops the
    // Wi-Fi stack filtering multicast beneath the socket. Asserting the exact
    // set rather than an absence, because a list that only forbids is a list a
    // new permission slips past.
    final source = File('android/app/src/main/AndroidManifest.xml');
    final text = _withoutComments(source.readAsStringSync());
    final declared = RegExp(r'<uses-permission android:name="([^"]+)"')
        .allMatches(text)
        .map((match) => match.group(1))
        .toList();

    expect(
        declared, <String>['android.permission.CHANGE_WIFI_MULTICAST_STATE']);
  });

  test('nothing of this application is backed up or transferred', () {
    // QYR-0349. ADR-0025 §3.4 decided `allowBackup=false` and the attribute was
    // never written, so the application shipped with Android's default of
    // **true**: Auto Backup would have copied the wrapped identity blob to
    // Google Drive. A decision written in an ADR and absent from the file it
    // decides is worth nothing, which is why this is asserted and not trusted.
    final source = File('android/app/src/main/AndroidManifest.xml');
    final text = _withoutComments(source.readAsStringSync());

    expect(text, contains('android:allowBackup="false"'));
    expect(text, contains('android:fullBackupContent="false"'));
    // API 31+ stopped letting `allowBackup` govern device-to-device transfer,
    // so the rules file is not optional above that level.
    expect(text, contains('android:dataExtractionRules='));

    final rules =
        File('android/app/src/main/res/xml/data_extraction_rules.xml');
    expect(
      rules.existsSync(),
      isTrue,
      reason: 'the manifest points at a rules file that does not exist, which '
          'fails the build -- and would fail it at package time, not here',
    );
    final body = _withoutComments(rules.readAsStringSync());
    // Empty sections mean "include nothing". An <include> of any kind would
    // mean something does leave.
    expect(body, isNot(contains('<include')));
    expect(body, contains('<cloud-backup'));
    expect(body, contains('<device-transfer'));
  });

  test('the merged manifest declares no storage permission', () {
    final merged = _mergedManifest();
    if (merged == null) {
      // Skipped rather than passed, and said out loud. A green tick for a file
      // that was never read is the failure mode this whole test exists to
      // avoid, so it must not be able to look like success.
      //
      // And where the manifest is *supposed* to exist, a skip is a failure.
      // Until phase 03 nothing in CI ran this test after `flutter build apk`,
      // so it skipped everywhere and the criterion it defends had never once
      // been checked. The job that builds the APK now sets this variable, and a
      // moved intermediates path fails the run instead of quietly skipping it.
      if (Platform.environment['QYRO_REQUIRE_MERGED_MANIFEST'] == '1') {
        fail(
          'QYRO_REQUIRE_MERGED_MANIFEST is set, so an APK was built and a '
          'merged manifest must exist. None was found. Update '
          '_intermediateRoots or _mergedCandidates rather than letting this '
          'skip. What the sweep saw:\n${_whatTheSweepSaw()}',
        );
      }
      markTestSkipped(
        'no merged manifest found under ${_intermediateRoots.join(" or ")}. '
        'Run `flutter build apk --debug` first; on this machine that needs '
        'Developer Mode for plugin symlinks (QYR-0324), so CI is where this '
        'assertion actually runs.',
      );
      return;
    }

    final text = _withoutComments(merged.readAsStringSync());
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
