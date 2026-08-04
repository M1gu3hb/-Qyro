import 'package:flutter/material.dart';

import '../generated/branding.g.dart';
import '../l10n/generated/app_localizations.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

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
                    strings.homeBaseline,
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 36),
                  _PrimaryAction(
                    icon: Icons.upload_file,
                    label: strings.homeSend,
                  ),
                  const SizedBox(height: 16),
                  _PrimaryAction(
                    icon: Icons.download,
                    label: strings.homeReceive,
                  ),
                  const SizedBox(height: 24),
                  Text(
                    strings.homeTransferUnavailable,
                    textAlign: TextAlign.center,
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
  const _PrimaryAction({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return FilledButton.icon(
      onPressed: null,
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
