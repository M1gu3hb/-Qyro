// What the picker does, exercised rather than read.
//
// Phase 03 shipped the Android channel and the Windows dialog with no Dart test
// of either. This is that half: the routing, the decoding of what each platform
// sends back, and the two properties that are about safety rather than
// plumbing — that an unsupported platform is refused *by name*, and that the
// name which travels to the receiver is never a path.
//
// The dialogs themselves are not driven here and cannot be: `flutter test` runs
// on the Dart VM with no window, and a modal Win32 dialog needs one. That is
// stated rather than papered over — see the phase report, §8.

import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/ffi/qyro_file_picker.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('the platform decides the shape, and says so', () {
    test('android picks by descriptor and windows by path', () {
      expect(
        pickerForPlatform(operatingSystem: 'android'),
        isA<QyroAndroidFilePicker>(),
      );
      expect(
        pickerForPlatform(operatingSystem: 'windows'),
        isA<QyroWindowsFilePicker>(),
      );
      // The two are genuinely different types, so the assertions above cannot
      // both be satisfied by one picker that answers to everything.
      expect(
        pickerForPlatform(operatingSystem: 'android').runtimeType,
        isNot(pickerForPlatform(operatingSystem: 'windows').runtimeType),
      );
    });

    test('an unsupported platform is refused by name and never by silence', () {
      for (final os in <String>['ios', 'linux', 'macos', 'fuchsia']) {
        expect(
          () => pickerForPlatform(operatingSystem: os),
          throwsA(
            isA<UnsupportedError>().having(
              (error) => error.message,
              'message',
              contains(os),
            ),
          ),
          reason: '$os must be refused. Returning an empty list would be '
              'indistinguishable from the person cancelling, which is the one '
              'wrong answer here',
        );
      }
    });
  });

  group('android: what the channel says becomes what Rust is handed', () {
    const channel = MethodChannel('dev.qyro/file_picker');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

    tearDown(() => messenger.setMockMethodCallHandler(channel, null));

    test('descriptors, names and sizes survive the crossing', () async {
      messenger.setMockMethodCallHandler(channel, (call) async {
        expect(call.method, 'pickFiles');
        return <Map<Object?, Object?>>[
          <Object?, Object?>{'fd': 7, 'name': 'holiday.jpg', 'size': 4096},
          <Object?, Object?>{'fd': 11, 'name': 'notes.txt', 'size': 12},
        ];
      });

      final picked = await const QyroAndroidFilePicker().pickFiles();

      expect(picked, hasLength(2));
      final first = picked.first as QyroPickedDescriptor;
      final second = picked.last as QyroPickedDescriptor;
      expect(first.descriptor, 7);
      expect(first.name, 'holiday.jpg');
      expect(first.size, 4096);
      expect(second.descriptor, 11);
      expect(second.name, 'notes.txt');
      // Two different descriptors and two different sizes on purpose: an
      // implementation that read the first entry twice satisfies every
      // assertion above except these.
      expect(first.descriptor, isNot(second.descriptor));
      expect(first.size, isNot(second.size));
    });

    test('a cancelled pick is an empty list and not an error', () async {
      messenger.setMockMethodCallHandler(channel, (call) async => null);
      expect(await const QyroAndroidFilePicker().pickFiles(), isEmpty);

      messenger.setMockMethodCallHandler(
        channel,
        (call) async => <Map<Object?, Object?>>[],
      );
      expect(await const QyroAndroidFilePicker().pickFiles(), isEmpty);
    });

    test('a name the provider withheld becomes a name and never null',
        () async {
      messenger.setMockMethodCallHandler(channel, (call) async {
        return <Map<Object?, Object?>>[
          <Object?, Object?>{'fd': 3},
        ];
      });

      final picked = await const QyroAndroidFilePicker().pickFiles();
      final only = picked.single as QyroPickedDescriptor;
      expect(only.name, isNotEmpty);
      // `-1` is the documented "the platform would not say", and it has to be
      // distinguishable from a real zero-byte file.
      expect(only.size, -1);
    });
  });

  group('windows: a path from the dialog becomes a path and a leaf name', () {
    late Directory scratch;

    setUp(() => scratch = Directory.systemTemp.createTempSync('qyro-picker'));
    tearDown(() {
      try {
        scratch.deleteSync(recursive: true);
      } on FileSystemException {
        // A held handle on Windows is not a test failure.
      }
    });

    test('the size is read from disk and the name is the leaf', () async {
      final file = File('${scratch.path}${Platform.pathSeparator}holiday.jpg')
        ..writeAsBytesSync(List<int>.filled(1234, 7));

      final picker = QyroWindowsFilePicker(
        openPaths: () async => <String>[file.path],
      );
      final picked = await picker.pickFiles();

      final only = picked.single as QyroPickedPath;
      expect(only.path, file.path);
      expect(only.name, 'holiday.jpg');
      expect(only.size, 1234);
      // Measured, not assumed: a size that came from a constant would survive a
      // file of any length, so the assertion is tied to what was written.
      expect(only.size, file.lengthSync());
      expect(only.size, greaterThan(0));
    });

    test('a path the dialog gave for a file that is gone reports -1', () async {
      final missing = '${scratch.path}${Platform.pathSeparator}gone.bin';
      final picker = QyroWindowsFilePicker(
        openPaths: () async => <String>[missing],
      );

      final only = (await picker.pickFiles()).single as QyroPickedPath;
      expect(only.size, -1);
      expect(only.name, 'gone.bin');
    });

    test('a cancelled dialog is an empty list', () async {
      final picker = QyroWindowsFilePicker(openPaths: () async => <String>[]);
      expect(await picker.pickFiles(), isEmpty);
    });
  });

  group('the name that travels is a name', () {
    test('both separators are cut, on whichever host runs this', () {
      expect(leafName(r'C:\Users\someone\Pictures\holiday.jpg'), 'holiday.jpg');
      expect(leafName('/home/someone/Pictures/holiday.jpg'), 'holiday.jpg');
      expect(leafName(r'mixed/separators\holiday.jpg'), 'holiday.jpg');
      expect(leafName('holiday.jpg'), 'holiday.jpg');
    });

    test('a name that is only separators does not become empty', () {
      // An empty name would reach the manifest, which refuses it — three layers
      // down. A name that only fails three layers down is a name that reached
      // three layers.
      expect(leafName(r'C:\Users\someone\'), 'file');
      expect(leafName('/'), 'file');
    });

    test('a hostile name keeps no path in it', () {
      for (final hostile in <String>[
        r'..\..\..\windows\system32\evil.dll',
        '../../../etc/passwd',
        r'C:\safe\..\..\evil.exe',
      ]) {
        final leaf = leafName(hostile);
        expect(leaf, isNot(contains('/')));
        expect(leaf, isNot(contains(r'\')));
      }
      // And the measurement can see the failure it is for: the inputs above do
      // contain separators, so a `leafName` that returned its argument unchanged
      // would fail rather than pass.
      expect(r'..\..\evil.dll', contains(r'\'));
      expect('../../evil', contains('/'));
    });
  });

  test('nothing in the app reaches for the file_selector umbrella', () {
    // ADR-0034 amendment 1. The umbrella package pulls `file_selector_android`,
    // which is the implementation that copies the whole file into the app cache
    // (QYR-0323). It is not a dependency, so an import would not compile — but
    // this fails at the moment somebody adds it back, with the reason attached,
    // rather than at the moment a 4 GB file is duplicated on a phone.
    final library = Directory('lib');
    expect(library.existsSync(), isTrue, reason: 'lib/ must exist to be read');

    final offenders = <String>[];
    var scanned = 0;
    for (final entry in library.listSync(recursive: true)) {
      if (entry is! File || !entry.path.endsWith('.dart')) continue;
      scanned++;
      final source = entry.readAsStringSync();
      for (final forbidden in <String>[
        "package:file_selector/",
        "package:file_selector_android/",
        "package:file_selector_ios/",
      ]) {
        if (source.contains(forbidden)) {
          offenders.add('${entry.path}: $forbidden');
        }
      }
    }

    expect(
      scanned,
      greaterThan(10),
      reason: 'only $scanned Dart files were read, so this scan is not seeing '
          'the app and an empty result means nothing',
    );
    expect(offenders, isEmpty);
    // And the scan can see what it is for: the file it is really about does
    // import the Windows implementation, so a scan that found nothing anywhere
    // would be broken rather than reassuring.
    expect(
      File('lib/ffi/qyro_file_picker.dart').readAsStringSync(),
      contains('package:file_selector_windows/'),
    );
  });
}
