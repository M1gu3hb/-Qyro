import 'package:flutter/material.dart';

import '../generated/branding.g.dart';
import '../l10n/generated/app_localizations.dart';
import '../transfer/native_transfer_service.dart';
import '../transfer/transfer_screens.dart';
import '../transfer/transfer_service.dart';

/// The first screen, and the two buttons this project kept switched off.
///
/// They were `onPressed: null` from the first commit with a line of text saying
/// so, because enabling them before the engine existed would have been the one
/// lie this project spent seven months not telling. They are on now, and the
/// text is gone: leaving it would be lying in the other direction.
///
/// The five conditions of ADR-0036 §5 and their evidence are in
/// `docs/reports/fase-05-la-interfaz-y-los-botones.md`.
class HomeScreen extends StatelessWidget {
  const HomeScreen({this.service, super.key});

  /// Injected by tests. Production builds the native one on first use, so a
  /// widget test never opens a dynamic library it does not need.
  final QyroTransferService? service;

  void _open(BuildContext context, int tab) {
    final engine = service ?? NativeTransferService();
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => TransferHome(service: engine, initialTab: tab),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);

    return Scaffold(
      appBar: AppBar(title: Text(strings.appTitle)),
      body: SafeArea(
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (GeneratedBranding.isProvisional) ...[
                    _ProvisionalNotice(
                      label: strings.bootProvisionalBranding,
                    ),
                    const SizedBox(height: 24),
                  ],
                  Text(
                    strings.homeTransferReady,
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 36),
                  _PrimaryAction(
                    icon: Icons.upload_file,
                    label: strings.homeSend,
                    onPressed: () => _open(context, 1),
                  ),
                  const SizedBox(height: 16),
                  _PrimaryAction(
                    icon: Icons.download,
                    label: strings.homeReceive,
                    onPressed: () => _open(context, 2),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _PrimaryAction extends StatelessWidget {
  const _PrimaryAction({
    required this.icon,
    required this.label,
    required this.onPressed,
  });

  final IconData icon;
  final String label;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return FilledButton.icon(
      onPressed: onPressed,
      icon: Icon(icon),
      label: Padding(
        padding: const EdgeInsets.symmetric(vertical: 18),
        child: Text(label),
      ),
    );
  }
}

class _ProvisionalNotice extends StatelessWidget {
  const _ProvisionalNotice({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: Color(0x26168BFF),
        borderRadius: BorderRadius.all(Radius.circular(8)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(10),
        child: Text(
          label,
          textAlign: TextAlign.center,
          style: const TextStyle(
            color: Color(0xFF9FCFFF),
            fontFamily: 'monospace',
            fontSize: 11,
          ),
        ),
      ),
    );
  }
}
