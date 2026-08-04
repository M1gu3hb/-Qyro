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
    expect(
      english.homeTransferUnavailable,
      'Transfer features are not implemented yet.',
    );
    expect(
      spanish.homeTransferUnavailable,
      'Funciones de transferencia aún no implementadas.',
    );
    expect(
      spanish.bootDiagnostic('asset_invalid'),
      'DIAGNÓSTICO: asset_invalid',
    );
  });

  test('catalog support is intentionally limited to English and Spanish', () {
    expect(AppLocalizations.delegate.isSupported(const Locale('en')), isTrue);
    expect(AppLocalizations.delegate.isSupported(const Locale('es')), isTrue);
    expect(AppLocalizations.delegate.isSupported(const Locale('fr')), isFalse);
  });
}
