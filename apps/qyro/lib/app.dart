import 'package:flutter/material.dart';

import 'boot/ascii_logo_model.dart';
import 'boot/boot_screen.dart';
import 'generated/branding.g.dart';
import 'home/home_screen.dart';
import 'l10n/generated/app_localizations.dart';
import 'startup/production_startup.dart';
import 'startup/startup_coordinator.dart';

class QyroApp extends StatefulWidget {
  const QyroApp({
    this.startupCoordinator,
    this.bootLogoModel,
    this.locale,
    super.key,
  });

  final StartupCoordinator? startupCoordinator;
  final AsciiLogoModel? bootLogoModel;
  final Locale? locale;

  @override
  State<QyroApp> createState() => _QyroAppState();
}

class _QyroAppState extends State<QyroApp> with WidgetsBindingObserver {
  late final StartupCoordinator _coordinator;
  late final bool _ownsCoordinator;
  var _showBoot = true;

  @override
  void initState() {
    super.initState();
    _ownsCoordinator = widget.startupCoordinator == null;
    _coordinator =
        widget.startupCoordinator ?? createProductionStartupCoordinator();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _coordinator.handleAppLifecycleState(state);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    if (_ownsCoordinator) {
      _coordinator.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    const colors = ColorScheme.dark(
      primary: Color(GeneratedBranding.primaryColorValue),
      secondary: Color(GeneratedBranding.secondaryColorValue),
      surface: Color(0xFF07111D),
    );

    return MaterialApp(
      debugShowCheckedModeBanner: false,
      locale: widget.locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
      theme: ThemeData(
        colorScheme: colors,
        scaffoldBackgroundColor: const Color(
          GeneratedBranding.backgroundColorValue,
        ),
        useMaterial3: true,
      ),
      home: _showBoot
          ? BootScreen(
              coordinator: _coordinator,
              logoModel: widget.bootLogoModel,
              onFinished: () {
                if (mounted) {
                  setState(() => _showBoot = false);
                }
              },
            )
          : const HomeScreen(),
    );
  }
}
