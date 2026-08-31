import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:qyro/l10n/generated/app_localizations.dart';

void main() {
  test('English and Spanish catalogs cover every visible baseline action',
      () async {
    final english = await AppLocalizations.delegate.load(const Locale('en'));
    final spanish = await AppLocalizations.delegate.load(const Locale('es'));

    expect(english.bootSkip, 'SKIP');
    expect(spanish.bootSkip, 'OMITIR');
    expect(english.bootRetry, 'RETRY');
    expect(spanish.bootRetry, 'REINTENTAR');
    expect(english.homeSend, 'Send');
    expect(spanish.homeSend, 'Enviar');
    expect(english.homeReceive, 'Receive');
    expect(spanish.homeReceive, 'Recibir');
    // The string that explained why the buttons were off is **gone**, and its
    // replacement says what the app does instead. ADR-0036 §5: leaving the old
    // sentence in place once the buttons work would be lying in the other
    // direction.
    expect(
      english.homeTransferReady,
      'Send a file to another device on this network.',
    );
    expect(
      spanish.homeTransferReady,
      'Manda un archivo a otro aparato de esta red.',
    );
    expect(
      spanish.bootDiagnostic('asset_invalid'),
      'DIAGNÓSTICO: asset_invalid',
    );
  });

  test('every transfer string exists in both catalogs and neither is a copy',
      () async {
    // ADR-0036 §6, mechanised. A key that exists in one catalogue and not the
    // other is a blank screen for half the users, and a Spanish string that is
    // byte-identical to the English one is a key somebody forgot to translate.
    final english = await AppLocalizations.delegate.load(const Locale('en'));
    final spanish = await AppLocalizations.delegate.load(const Locale('es'));

    // Read through the getters rather than the .arb files: what ships is the
    // generated class, and a test over the source data would pass while the
    // generator dropped a key.
    final pairs = <String, (String, String)>{
      'homeTransferReady': (
        english.homeTransferReady,
        spanish.homeTransferReady
      ),
      'navPeers': (english.navPeers, spanish.navPeers),
      'navSend': (english.navSend, spanish.navSend),
      'navReceive': (english.navReceive, spanish.navReceive),
      'navHistory': (english.navHistory, spanish.navHistory),
      'peersNone': (english.peersNone, spanish.peersNone),
      'peersManualHint': (english.peersManualHint, spanish.peersManualHint),
      'peersManualLabel': (english.peersManualLabel, spanish.peersManualLabel),
      'peersManualInvalid': (
        english.peersManualInvalid,
        spanish.peersManualInvalid
      ),
      'peersUseCode': (english.peersUseCode, spanish.peersUseCode),
      'peersOwnCode': (english.peersOwnCode, spanish.peersOwnCode),
      'peersOwnCodeUnavailable': (
        english.peersOwnCodeUnavailable,
        spanish.peersOwnCodeUnavailable
      ),
      'peersTrustKnown': (english.peersTrustKnown, spanish.peersTrustKnown),
      'peersTrustChanged': (
        english.peersTrustChanged,
        spanish.peersTrustChanged
      ),
      'peersTrustNew': (english.peersTrustNew, spanish.peersTrustNew),
      'peersChangedExplain': (
        english.peersChangedExplain,
        spanish.peersChangedExplain
      ),
      'peersForget': (english.peersForget, spanish.peersForget),
      'peersNameLabel': (english.peersNameLabel, spanish.peersNameLabel),
      'peersNameInvalid': (english.peersNameInvalid, spanish.peersNameInvalid),
      'peersSave': (english.peersSave, spanish.peersSave),
      'sendChoose': (english.sendChoose, spanish.sendChoose),
      'sendNoFiles': (english.sendNoFiles, spanish.sendNoFiles),
      'sendCancelled': (english.sendCancelled, spanish.sendCancelled),
      'sendUnreachable': (english.sendUnreachable, spanish.sendUnreachable),
      'sendKeyChanged': (english.sendKeyChanged, spanish.sendKeyChanged),
      'sendIntegrity': (english.sendIntegrity, spanish.sendIntegrity),
      'sendCancelledByUser': (
        english.sendCancelledByUser,
        spanish.sendCancelledByUser
      ),
      'reasonDeclined': (english.reasonDeclined, spanish.reasonDeclined),
      'reasonNoRoom': (english.reasonNoRoom, spanish.reasonNoRoom),
      'reasonManifest': (english.reasonManifest, spanish.reasonManifest),
      'reasonUnspecified': (
        english.reasonUnspecified,
        spanish.reasonUnspecified
      ),
      'receiveWaiting': (english.receiveWaiting, spanish.receiveWaiting),
      'receiveStart': (english.receiveStart, spanish.receiveStart),
      'receiveOfferUnknown': (
        english.receiveOfferUnknown,
        spanish.receiveOfferUnknown
      ),
      'receiveAccept': (english.receiveAccept, spanish.receiveAccept),
      'receiveRefuse': (english.receiveRefuse, spanish.receiveRefuse),
      'receiveRefused': (english.receiveRefused, spanish.receiveRefused),
      'receiveNoRoom': (english.receiveNoRoom, spanish.receiveNoRoom),
      'failIdentity': (english.failIdentity, spanish.failIdentity),
      'failBadAddress': (english.failBadAddress, spanish.failBadAddress),
      'failInternal': (english.failInternal, spanish.failInternal),
      'sendAddressLabel': (
        english.sendAddressLabel,
        spanish.sendAddressLabel
      ),
      'receivePortUnavailable': (
        english.receivePortUnavailable,
        spanish.receivePortUnavailable
      ),
      'historyEmpty': (english.historyEmpty, spanish.historyEmpty),
      'historyFailed': (english.historyFailed, spanish.historyFailed),
      'historySucceeded': (english.historySucceeded, spanish.historySucceeded),
      'historySent': (english.historySent, spanish.historySent),
      'historyReceived': (english.historyReceived, spanish.historyReceived),
      'commonCancel': (english.commonCancel, spanish.commonCancel),
      'commonClose': (english.commonClose, spanish.commonClose),
      'commonRetry': (english.commonRetry, spanish.commonRetry),
      'fingerprintCompare': (
        english.fingerprintCompare,
        spanish.fingerprintCompare
      ),
    };

    final untranslated = <String>[];
    for (final entry in pairs.entries) {
      final (left, right) = entry.value;
      expect(left, isNotEmpty, reason: '${entry.key} is empty in English');
      expect(right, isNotEmpty, reason: '${entry.key} is empty in Spanish');
      if (left == right) untranslated.add(entry.key);
    }
    expect(
      untranslated,
      isEmpty,
      reason: 'these read identically in both catalogs, which is what a key '
          'nobody translated looks like',
    );
    expect(
      pairs.length,
      greaterThan(40),
      reason: 'only ${pairs.length} strings were compared, so this test is not '
          'covering the interface it is for',
    );

    // The placeholder strings, exercised rather than read: a key whose
    // substitution is wrong renders the brace instead of the value.
    expect(english.sendChosen('2', '4.0 KiB'), contains('2'));
    expect(spanish.sendChosen('2', '4.0 KiB'), contains('4.0 KiB'));
    expect(spanish.progressOf('1 B', '2 B'), '1 B de 2 B');
    expect(english.progressOf('1 B', '2 B'), '1 B of 2 B');
    expect(english.receiveDelivered('3', '/tmp'), contains('/tmp'));
    expect(spanish.peersForgetConfirm('phone'), contains('phone'));
    for (final rendered in <String>[
      english.sendChosen('2', '4.0 KiB'),
      spanish.receiveOfferFrom('1', '9 B'),
      english.sendRefused('no room'),
      spanish.sendDelivered('1'),
    ]) {
      expect(rendered, isNot(contains('{')), reason: '$rendered kept a brace');
    }
  });

  test('catalog support is intentionally limited to English and Spanish', () {
    expect(AppLocalizations.delegate.isSupported(const Locale('en')), isTrue);
    expect(AppLocalizations.delegate.isSupported(const Locale('es')), isTrue);
    expect(AppLocalizations.delegate.isSupported(const Locale('fr')), isFalse);
  });
}
