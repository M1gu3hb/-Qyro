import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../boot/ascii_logo_model.dart';
import '../ffi/qyro_native_api.dart';
import '../generated/branding.g.dart';
import 'startup_coordinator.dart';

StartupCoordinator createProductionStartupCoordinator() {
  return StartupCoordinator(
    loadBranding: () async {
      return const StartupBranding(
        isProvisional: GeneratedBranding.isProvisional,
      );
    },
    verifyAssets: () async {
      final source = await rootBundle.loadString(
        'assets/generated/logo_ascii.json',
      );
      AsciiLogoModel.fromJsonString(source);
    },
    loadNativeBridge: () async => QyroNativeApi.openDefault(),
    initializeInterface: () async {
      final binding = WidgetsBinding.instance;
      if (binding.platformDispatcher.views.isEmpty) {
        throw const StartupTaskFailure(
          code: 'interface_unavailable',
          userMessageKey: 'startupInterfaceUnavailable',
          technicalSummary: 'Flutter did not expose an attached platform view',
        );
      }
    },
  );
}
