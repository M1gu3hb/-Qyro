import 'package:flutter/material.dart';

import '../generated/branding.g.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text(GeneratedBranding.appName)),
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
                    const _ProvisionalNotice(),
                    const SizedBox(height: 24),
                  ],
                  Text(
                    'Base de interfaz local en preparación.',
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 36),
                  const _PrimaryAction(
                    icon: Icons.upload_file,
                    label: 'Enviar',
                  ),
                  const SizedBox(height: 16),
                  const _PrimaryAction(
                    icon: Icons.download,
                    label: 'Recibir',
                  ),
                  const SizedBox(height: 24),
                  const Text(
                    'Funciones de transferencia aún no implementadas.',
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
  const _ProvisionalNotice();

  @override
  Widget build(BuildContext context) {
    return const DecoratedBox(
      decoration: BoxDecoration(
        color: Color(0x26168BFF),
        borderRadius: BorderRadius.all(Radius.circular(8)),
      ),
      child: Padding(
        padding: EdgeInsets.all(10),
        child: Text(
          'DATOS DE MARCA PROVISIONALES',
          textAlign: TextAlign.center,
          style: TextStyle(
            color: Color(0xFF9FCFFF),
            fontFamily: 'monospace',
            fontSize: 11,
          ),
        ),
      ),
    );
  }
}
