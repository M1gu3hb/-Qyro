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
///
/// **Release as well as debug.** Phase 10 added a workflow that builds the
/// release APK and runs this assertion on it, and the release APK is the one
/// people install. Listing only `debug` would have made this test fall through
/// to the wide sweep on the build that matters -- or, if the sweep also missed,
/// skip on the artifact whose permissions are the whole point.
const _mergedCandidates = <String>[
  'merged_manifests/release/processReleaseMainManifest/AndroidManifest.xml',
  'merged_manifests/release/processReleaseManifest/AndroidManifest.xml',
  'merged_manifests/release/AndroidManifest.xml',
  'merged_manifest/release/AndroidManifest.xml',
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

  test('the manifest declares exactly three permissions, and they are these',
      () {
    // Not «no permissions»: discovery needs `CHANGE_WIFI_MULTICAST_STATE`,
    // which is a normal permission granted at install and is what stops the
    // Wi-Fi stack filtering multicast beneath the socket. Since phase 24B,
    // `CAMERA`: the optical channel is the one that works with no network of
    // any kind, and without a camera there is no optical channel. And since
    // QYR-0368, `INTERNET` — see the test below for why its absence was a P0.
    //
    // **The exact set, and the exact count.** Not «at least these» and not «not
    // these»: a list that only forbids is a list a new permission slips past,
    // and «at least» would let a fourth arrive without anybody noticing. Three,
    // named, in order.
    final source = File('android/app/src/main/AndroidManifest.xml');
    final text = _withoutComments(source.readAsStringSync());
    final declared = RegExp(r'<uses-permission android:name="([^"]+)"')
        .allMatches(text)
        .map((match) => match.group(1))
        .toList();

    expect(declared, <String>[
      'android.permission.CHANGE_WIFI_MULTICAST_STATE',
      'android.permission.CAMERA',
      'android.permission.INTERNET',
    ]);
    expect(declared.length, 3,
        reason: 'a fourth permission arrived without an argument next to it');
  });

  test('the source manifest declares INTERNET, because everything here is TCP',
      () {
    // QYR-0368, P0. `android.permission.INTERNET` lived **only** in
    // `app/src/debug/AndroidManifest.xml` and `app/src/profile/`, and neither
    // source set reaches a release build: Gradle merges `main` plus the source
    // set of the variant being built. So `flutter build apk --release`
    // produced an APK with no `INTERNET` at all, while `flutter run` and every
    // emulator run had it — the only build that failed was the only build
    // anybody installs.
    //
    // What the person would have seen on the phone: a socket call inside the
    // native library failing with `Permission denied (errno = 13)`, a message
    // that names neither Qyro nor a permission.
    //
    // Asserted separately from the exact-set test above so that a failure says
    // *which* permission and *why*, rather than printing a list diff.
    final source = File('android/app/src/main/AndroidManifest.xml');
    final text = _withoutComments(source.readAsStringSync());
    expect(
      text,
      contains('android.permission.INTERNET'),
      reason: 'every channel Qyro has except the optical one is a TCP socket. '
          'Without INTERNET the release APK cannot bind or connect, and the '
          'error the user sees mentions neither Qyro nor a permission.',
    );
  });

  test('the camera is declared optional, so a phone without one still runs',
      () {
    // `required="false"` on purpose: a device with no camera **is still a Qyro
    // device** — it has the network, the direct cable and the serial line. A
    // required feature would take it out of the store over one of four
    // channels.
    final source = File('android/app/src/main/AndroidManifest.xml');
    final text = _withoutComments(source.readAsStringSync());
    final features = RegExp(
            r'<uses-feature android:name="([^"]+)" android:required="([^"]+)"')
        .allMatches(text)
        .map((match) => '${match.group(1)}=${match.group(2)}')
        .toList();

    expect(features, contains('android.hardware.camera=false'));
    expect(features, contains('android.hardware.camera.autofocus=false'));
    expect(features.where((entry) => entry.endsWith('=true')), isEmpty,
        reason: 'a required feature narrows which devices can install this');
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

  test('no Android XML has a comment that XML cannot parse', () {
    // QYR-0351. `data_extraction_rules.xml` carried `-- which is exactly` in a
    // comment, and `--` inside a comment is not well-formed XML. The build died
    // at `:app:parseDebugLocalResources` with "Failed to parse XML file", two
    // minutes into a Gradle run in CI, on a file that had been added by the
    // commit before.
    //
    // Narrow on purpose: this is not an XML validator, it is the one mistake
    // that is easy to make while writing prose in a comment and impossible to
    // see by reading it. Everything else about these files is already checked
    // by the tool that consumes them.
    final offenders = <String>[];
    for (final file in Directory('android/app/src/main')
        .listSync(recursive: true)
        .whereType<File>()
        .where((entry) => entry.path.endsWith('.xml'))) {
      for (final comment in RegExp(r'<!--(.*?)-->', dotAll: true).allMatches(
        file.readAsStringSync(),
      )) {
        if ((comment.group(1) ?? '').contains('--')) {
          offenders.add(file.path);
        }
      }
    }

    expect(
      offenders,
      isEmpty,
      reason: 'An XML comment may not contain a double hyphen. Use a comma, a '
          'dash, or a new sentence.',
    );

    // The control, both ways: the pattern finds the mistake, and does not
    // report a comment that merely ends in one.
    const bad = '<!-- a thing -- and another -->';
    const good = '<!-- a thing, and another -->';
    String? inner(String xml) =>
        RegExp(r'<!--(.*?)-->', dotAll: true).firstMatch(xml)?.group(1);
    expect(inner(bad)!.contains('--'), isTrue);
    expect(inner(good)!.contains('--'), isFalse);
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
    // **The half this test did not have, and the half that mattered.** It only
    // ever asserted that permissions were *absent*. A permission that has to be
    // *present* could therefore disappear and every check stayed green — which
    // is exactly what happened to `INTERNET` (QYR-0368): declared in the debug
    // and profile source sets, absent from `main`, and so absent from every
    // release APK this project has ever built. A measurement that cannot see
    // the failure it is for is the defect this file was written to avoid, and
    // it had it.
    //
    // Release only, when the path says so. The debug merged manifest *does*
    // carry `INTERNET` from `app/src/debug/`, so asserting it there would pass
    // while the release APK stayed broken.
    final isRelease = merged.path.contains('release');
    // And where the caller *says* it built a release APK, reading the debug
    // manifest instead is a failure and not a pass. Without this, a moved
    // intermediates path would send the release job to the debug manifest and
    // the one assertion the release job exists to make would evaporate — the
    // same way the whole INTERNET check evaporated in the first place.
    if (Platform.environment['QYRO_REQUIRE_RELEASE_MANIFEST'] == '1') {
      expect(
        isRelease,
        isTrue,
        reason: 'QYRO_REQUIRE_RELEASE_MANIFEST is set, so a release APK was '
            'built and its merged manifest must be the one read. What was '
            'read instead: ${merged.path}',
      );
    }
    if (isRelease) {
      expect(
        text.contains('android.permission.INTERNET'),
        isTrue,
        reason:
            'the merged RELEASE manifest at ${merged.path} does not declare '
            'android.permission.INTERNET. This is the manifest of the APK '
            'people install, and every Qyro channel except the optical one is '
            'a TCP socket. Check that app/src/main/AndroidManifest.xml still '
            'declares it: app/src/debug/ and app/src/profile/ do not reach a '
            'release build.',
      );
    }
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
