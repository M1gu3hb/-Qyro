// The four screens and their ugly states, driven without a socket.
//
// ADR-0036 enumerates the states because they are what decides whether this is
// a product or a demo. Each one is reachable here through a fake service, which
// is the reason the screens talk to an interface and not to the FFI.

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:qyro/discovery/qyro_discovery.dart';
import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/ffi/qyro_trust_api.dart';
import 'package:qyro/l10n/generated/app_localizations.dart';
import 'package:qyro/transfer/transfer_screens.dart';
import 'package:qyro/transfer/transfer_service.dart';

/// A service that answers whatever a test tells it to.
///
/// **What this fake can and cannot show, because it already fooled this
/// project once.** It drives every ugly state of the four screens without a
/// socket, which is what it is for and what it does well. What it cannot do is
/// tell a *measured* value from a constant a test wrote next to the assertion:
/// `ownPairingString()` here returns whatever `ownCode` says, so
/// "the screen shows the code" passed for months while the production service
/// returned null for every transfer the product ever attempted (QYR-0322).
///
/// Anything that has to be **measured** rather than displayed belongs in
/// `test/transfer/native_transfer_service_test.dart` against the production
/// class, or in the two-process test. This file tests rendering.
final class FakeService implements QyroTransferService {
  FakeService({
    this.peers = const <QyroPeerEntry>[],
    this.pairingAddress,
    this.ownCode,
    this.candidates = const <QyroListenAddress>[],
    this.picked = const <QyroPicked>[],
    this.states = const <QyroTransferState>[],
    this.entries = const <QyroHistoryEntry>[],
  });

  List<QyroPeerEntry> peers;
  String? pairingAddress;
  String? ownCode;
  List<QyroListenAddress> candidates;
  List<QyroPicked> picked;
  List<QyroTransferState> states;
  List<QyroHistoryEntry> entries;
  final forgotten = <String>[];

  @override
  Future<List<QyroPeerEntry>> knownPeers() async => peers;

  @override
  Future<bool> forgetPeer(String name) async {
    forgotten.add(name);
    peers = peers.where((peer) => peer.name != name).toList();
    return true;
  }

  @override
  Future<String?> addressOfPairingString(String text) async => pairingAddress;

  @override
  Future<String?> ownPairingString() async => ownCode;

  @override
  Future<List<QyroListenAddress>> listenCandidates() async => candidates;

  @override
  Future<List<QyroPicked>> pickFiles() async => picked;

  @override
  Stream<QyroTransferState> send({
    required String address,
    required List<QyroPicked> files,
    String? expectedFingerprint,
  }) =>
      Stream<QyroTransferState>.fromIterable(states);

  @override
  Stream<QyroTransferState> receive({
    required String bind,
    required String destination,
    required Future<bool> Function(QyroAwaitingDecision offer) decide,
  }) =>
      Stream<QyroTransferState>.fromIterable(states);

  @override
  Future<List<QyroHistoryEntry>> history() async => entries;
}

Widget _wrap(Widget child, {Locale locale = const Locale('en')}) => MaterialApp(
      locale: locale,
      localizationsDelegates: const <LocalizationsDelegate<Object>>[
        AppLocalizations.delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ],
      supportedLocales: AppLocalizations.supportedLocales,
      // A Scaffold, because TextField and Card need a Material ancestor and a
      // bare home: has none. The screens are always inside one in the app.
      home: Scaffold(body: child),
    );

/// Discovery that answers with whatever the test put in it.
///
/// Hand-written rather than mocked: the screen's contract with discovery is
/// three methods, and a fake that implements them is easier to read than a
/// framework that describes them.
final class FakeDiscovery implements QyroDiscovery {
  FakeDiscovery({this.found = const <QyroFoundPeer>[], this.refuse});

  final List<QyroFoundPeer> found;
  final String? refuse;
  int advertised = 0;

  @override
  Future<void> advertise({
    required int port,
    required String fingerprint,
  }) async {
    if (refuse != null) throw QyroDiscoveryUnavailable(refuse!);
    advertised++;
  }

  @override
  Future<List<QyroFoundPeer>> browse() async {
    if (refuse != null) throw QyroDiscoveryUnavailable(refuse!);
    return found;
  }

  @override
  Future<void> stop() async {}
}

/// The same fingerprint, in the compact spelling a pairing code carries.
const _compact = 'ab12cd34ab12cd34ab12cd34ab12cd34';

const _fingerprint = '49eff48e-89bf12b0-1122aabb-33445566-'
    '778899aa-bbccddee-ff001122-0bff77f7';

void main() {
  group('peers', () {
    testWidgets('an empty book says so, and the manual path is still there',
        (tester) async {
      await tester.pumpWidget(_wrap(PeersScreen(service: FakeService())));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('peers-empty')), findsOneWidget);
      // Never behind "advanced": it is the only path that works in every
      // network (ADR-0036 §3).
      expect(find.byKey(const Key('pairing-field')), findsOneWidget);
      expect(find.byKey(const Key('pairing-scan')), findsOneWidget);
    });

    testWidgets('en escritorio no se ofrece un escaner que no existe',
        (tester) async {
      // ADR-0044 §6: el escritorio DIBUJA los QR, el telefono los lee. Ofrecer
      // aqui un boton de escanear seria prometer lo que no hay -- que es el
      // defecto exacto de QYR-0348, un icono de camara sobre un parser de texto.
      //
      // Las pruebas corren en escritorio, asi que `scannerAvailableOn()` es
      // falso y el boton no debe existir.
      await tester.pumpWidget(
        _wrap(PeersScreen(service: FakeService(), discovery: FakeDiscovery())),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('scan-open')), findsNothing);
      // Y el control: la pantalla si se dibujo, asi que el `findsNothing` de
      // arriba dice algo. Sin esto, una pantalla que no montara pasaria igual.
      expect(find.byKey(const Key('pairing-field')), findsOneWidget);
    });

    testWidgets('a device found on the network is offered, not dialled',
        (tester) async {
      // ADR-0043: what a discovered device offers is a *code*. The person still
      // decides, and the trust check that follows is the same one a typed code
      // goes through -- a device that announced itself has proved nothing.
      final discovery = FakeDiscovery(
        found: const <QyroFoundPeer>[
          QyroFoundPeer(
            address: '192.168.1.9:49517',
            fingerprint: 'ab12cd34ab12cd34ab12cd34ab12cd34',
          ),
        ],
      );
      await tester.pumpWidget(
        _wrap(
          PeersScreen(
            // With an identity, because a device with none has nothing to
            // announce -- and that is a real state, not a test convenience:
            // `ownPairingString` returns null before the identity exists.
            service: FakeService(ownCode: 'QYRO1|10.0.0.1:49517|$_compact'),
            discovery: discovery,
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key('nearby-ab12cd34ab12cd34ab12cd34ab12cd34')),
        findsOneWidget,
      );
      expect(find.text('192.168.1.9:49517'), findsOneWidget);
      // And it announced this device too: discovery that only listens finds
      // nobody, because the other side is also only listening.
      expect(discovery.advertised, 1);
    });

    testWidgets('a device that cannot look says so instead of looking empty',
        (tester) async {
      // The distinction that cost this project a phase: an empty list from a
      // platform that cannot ask is indistinguishable from a quiet network.
      final discovery = FakeDiscovery(refuse: 'this device has no NsdManager');
      await tester.pumpWidget(
        _wrap(PeersScreen(service: FakeService(), discovery: discovery)),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('nearby-unavailable')), findsOneWidget);
      expect(
        find.byKey(const Key('nearby-empty')),
        findsNothing,
        reason: 'a device that cannot look was shown as a quiet network',
      );
      // The manual path is untouched, which is the whole point of it.
      expect(find.byKey(const Key('pairing-field')), findsOneWidget);
    });

    testWidgets('and a quiet network says the other thing', (tester) async {
      // The control for the test above. A screen that showed "cannot look"
      // whatever happened would pass it and be useless.
      await tester.pumpWidget(
        _wrap(PeersScreen(service: FakeService(), discovery: FakeDiscovery())),
      );
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('nearby-empty')), findsOneWidget);
      expect(find.byKey(const Key('nearby-unavailable')), findsNothing);
    });

    testWidgets('a pairing code that is not ours says so and resolves nothing',
        (tester) async {
      final service = FakeService();
      await tester.pumpWidget(_wrap(PeersScreen(service: service)));
      await tester.pumpAndSettle();

      await tester.enterText(
          find.byKey(const Key('pairing-field')), 'nonsense');
      await tester.tap(find.byKey(const Key('pairing-scan')));
      await tester.pumpAndSettle();

      expect(find.text('That is not a Qyro pairing code.'), findsOneWidget);
      expect(find.byKey(const Key('pairing-address')), findsNothing);

      // And the control: a code that *is* ours resolves and clears the error,
      // so the refusal above is about the code and not about the screen.
      service.pairingAddress = '192.168.1.7:47001';
      await tester.tap(find.byKey(const Key('pairing-scan')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('pairing-address')), findsOneWidget);
      expect(find.text('That is not a Qyro pairing code.'), findsNothing);
    });

    testWidgets('a peer whose key changed does not look like the others',
        (tester) async {
      final service = FakeService(peers: <QyroPeerEntry>[
        const QyroPeerEntry(
          name: 'laptop',
          fingerprint: _fingerprint,
          trust: QyroPeerTrust.known,
        ),
        const QyroPeerEntry(
          name: 'phone',
          fingerprint: _fingerprint,
          trust: QyroPeerTrust.changed,
        ),
      ]);
      await tester.pumpWidget(_wrap(PeersScreen(service: service)));
      await tester.pumpAndSettle();

      final calm = tester.widget<Card>(find.byKey(const Key('peer-laptop')));
      final alarming = tester.widget<Card>(find.byKey(const Key('peer-phone')));

      // Different colour, and different *from each other*: asserting only that
      // the alarming one has a colour would pass if both did.
      expect(alarming.color, isNotNull);
      expect(alarming.color, isNot(calm.color));
      // A sentence that says what it means, not a code.
      expect(
        find.byKey(const Key('peer-changed-explain-phone')),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key('peer-changed-explain-laptop')),
        findsNothing,
      );
      expect(find.text("This device's key has changed"), findsOneWidget);
      // And the fingerprint is shown as the core formatted it.
      expect(find.text(_fingerprint), findsNWidgets(2));
    });

    testWidgets('forgetting a peer removes it and asks the engine',
        (tester) async {
      final service = FakeService(peers: <QyroPeerEntry>[
        const QyroPeerEntry(
          name: 'phone',
          fingerprint: _fingerprint,
          trust: QyroPeerTrust.changed,
        ),
      ]);
      await tester.pumpWidget(_wrap(PeersScreen(service: service)));
      await tester.pumpAndSettle();

      // The nearby section sits above this list now, so on a test-sized
      // viewport the button is below the fold -- exactly as it is on a phone.
      // Scrolled to rather than asserted away.
      await tester.ensureVisible(find.byKey(const Key('peer-forget-phone')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('peer-forget-phone')));
      await tester.pumpAndSettle();

      expect(service.forgotten, <String>['phone']);
      expect(find.byKey(const Key('peers-empty')), findsOneWidget);
    });
  });

  group('send', () {
    testWidgets('nothing chosen means the send action is not offered',
        (tester) async {
      await tester.pumpWidget(_wrap(SendScreen(service: FakeService())));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('send-no-files')), findsOneWidget);
      final button =
          tester.widget<FilledButton>(find.byKey(const Key('send-start')));
      expect(button.onPressed, isNull);
    });

    testWidgets('a refusal shows the reason the far end gave', (tester) async {
      final service = FakeService(
        states: const <QyroTransferState>[
          QyroFailed(
            kind: QyroFailureKind.refusedByPeer,
            reason: QyroRejectReason.noRoom,
          ),
        ],
        picked: <QyroPicked>[
          const QyroPickedPath(path: '/tmp/a.bin', name: 'a.bin', size: 4096),
        ],
      );
      await tester.pumpWidget(_wrap(SendScreen(service: service)));
      await tester.pumpAndSettle();

      await tester.enterText(
          find.byKey(const Key('send-address')), '127.0.0.1:1');
      await tester.tap(find.byKey(const Key('send-pick')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('send-start')));
      await tester.pumpAndSettle();

      // The reason, not just «it failed»: «they said no» and «no room on the
      // device» are different sentences and QYR-0089 exists so the sender knows
      // which one.
      expect(find.textContaining('no room on the device'), findsOneWidget);
    });

    testWidgets('each failure kind has its own sentence', (tester) async {
      const cases = <(QyroFailureKind, String)>[
        (QyroFailureKind.unreachable, 'That address answered nothing.'),
        (
          QyroFailureKind.keyChanged,
          'Refused: this device\'s key is not the one saved under that name.'
        ),
        (QyroFailureKind.refusedByMe, 'Refused. Nothing was written.'),
        (
          QyroFailureKind.integrity,
          'Something arrived that did not verify. Nothing was kept.'
        ),
        (QyroFailureKind.cancelled, 'Stopped.'),
        (QyroFailureKind.noRoom, 'There is no room for this.'),
      ];

      final seen = <String>{};
      for (final (kind, expected) in cases) {
        await tester
            .pumpWidget(_wrap(TransferStatus(state: QyroFailed(kind: kind))));
        await tester.pumpAndSettle();
        expect(find.text(expected), findsOneWidget, reason: '$kind');
        seen.add(expected);
      }
      // Distinct, not merely present: six kinds sharing one sentence would
      // satisfy every assertion above.
      expect(seen.length, cases.length);
    });

    testWidgets('progress renders bytes and not a bare number', (tester) async {
      await tester.pumpWidget(
        _wrap(
          const TransferStatus(
            state: QyroMoving(done: 1536, total: 4096, fingerprint: ''),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('1.5 KiB of 4.0 KiB'), findsOneWidget);
    });
  });

  group('receive', () {
    testWidgets(
        'an offer shows who, how many, how much and what they are named',
        (tester) async {
      // ADR-0036 §2: all four before the first byte. A screen that only asked
      // «accept transfer?» would be asking permission for something it had not
      // described.
      var accepted = false;
      await tester.pumpWidget(
        _wrap(
          OfferCard(
            offer: const QyroAwaitingDecision(
              fingerprint: _fingerprint,
              trust: QyroPeerTrust.newPeer,
              fileNames: <String>['holiday.jpg', 'notes.txt'],
              totalBytes: 5120,
            ),
            onAccept: () => accepted = true,
            onRefuse: () {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('2 file(s)'), findsOneWidget);
      expect(find.textContaining('5.0 KiB'), findsOneWidget);
      expect(find.text(_fingerprint), findsOneWidget);
      expect(find.text('holiday.jpg'), findsOneWidget);
      expect(find.text('notes.txt'), findsOneWidget);
      expect(
        find.text('You have never accepted this device before.'),
        findsOneWidget,
      );

      // And nothing is accepted on its own: it takes a tap.
      expect(accepted, isFalse);
      await tester.tap(find.byKey(const Key('offer-accept')));
      expect(accepted, isTrue);
    });

    testWidgets('an offer from a changed key offers no way to accept it',
        (tester) async {
      await tester.pumpWidget(
        _wrap(
          OfferCard(
            offer: const QyroAwaitingDecision(
              fingerprint: _fingerprint,
              trust: QyroPeerTrust.changed,
              fileNames: <String>['a.bin'],
              totalBytes: 10,
            ),
            onAccept: () {},
            onRefuse: () {},
          ),
        ),
      );
      await tester.pumpAndSettle();

      // Absent, not disabled. There is no "continue anyway" to find and press
      // (ADR-0036 §1 and §4).
      expect(find.byKey(const Key('offer-accept')), findsNothing);
      expect(find.byKey(const Key('offer-refuse')), findsOneWidget);
      expect(find.text("This device's key has changed"), findsOneWidget);
    });
  });

  group('history', () {
    testWidgets('an empty history says so', (tester) async {
      await tester.pumpWidget(_wrap(HistoryScreen(service: FakeService())));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('history-empty')), findsOneWidget);
    });

    testWidgets('a failed entry reads differently from a delivered one',
        (tester) async {
      final service = FakeService(entries: const <QyroHistoryEntry>[
        QyroHistoryEntry(
          name: 'a.bin',
          peer: 'laptop',
          bytes: 2048,
          succeeded: true,
          outgoing: true,
        ),
        QyroHistoryEntry(
          name: 'b.bin',
          peer: 'phone',
          bytes: 4096,
          succeeded: false,
          outgoing: false,
        ),
      ]);
      await tester.pumpWidget(_wrap(HistoryScreen(service: service)));
      await tester.pumpAndSettle();

      expect(find.text('Delivered'), findsOneWidget);
      expect(find.text('Failed'), findsOneWidget);
      expect(find.textContaining('2.0 KiB'), findsOneWidget);
    });
  });

  group('a hostile name cannot reorder the line it is drawn in', () {
    // Every control below is written as an escape and never as the character
    // itself. That is not style: the analyser refuses a raw U+202E in a
    // literal, for exactly the reason this test exists — the source line would
    // render differently from what the compiler reads, which is the attack.
    const rightToLeftOverride = '\u202E';
    const rightToLeftMark = '\u200F';
    const firstStrongIsolate = '\u2066';
    const arabicLetterMark = '\u061C';
    const nul = '\u0000';
    const del = '\u007F';

    test('bidirectional overrides and controls are dropped', () {
      // `invoice<RLO>fdp.exe` renders as `invoiceexe.pdf` in any widget that
      // honours the override. The manifest layer refuses names that would
      // escape the destination; that protects the disk, and the screen is a
      // separate problem that this project already closed once for the
      // filesystem.
      const hostile = 'invoice${rightToLeftOverride}fdp.exe';
      final safe = safeDisplayName(hostile);

      expect(safe, 'invoicefdp.exe');
      expect(safe, isNot(contains(rightToLeftOverride)));
      // The measurement can see what it is for: the input really does carry the
      // override, so a `safeDisplayName` that returned its argument unchanged
      // would fail here rather than pass.
      expect(hostile, contains(rightToLeftOverride));

      for (final control in <String>[
        rightToLeftMark,
        firstStrongIsolate,
        arabicLetterMark,
        nul,
        del,
      ]) {
        expect(
          safeDisplayName('a${control}b'),
          'ab',
          reason: control.codeUnits.toString(),
        );
      }
    });

    test('a name that was only controls does not become an empty row', () {
      // An empty row is a row nobody can point at, and the name is how a person
      // decides whether to accept.
      expect(safeDisplayName('$rightToLeftOverride$rightToLeftMark'), '—');
      expect(safeDisplayName('   '), '—');
      // And an ordinary name survives untouched, so the stripping above is
      // about the controls and not about every character.
      expect(safeDisplayName('holiday.jpg'), 'holiday.jpg');
      expect(safeDisplayName('vacaciones año 2026.png'),
          'vacaciones año 2026.png');
    });
  });
}
