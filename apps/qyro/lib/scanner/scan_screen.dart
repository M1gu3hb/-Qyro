// La pantalla de escaneo.
//
// ADR-0048 y la fase 24B. Es la cara del canal que funciona **sin red de ninguna
// clase**: el otro aparato dibuja QR en su terminal y éste los mira.
//
// # Sin visor, y se dice
//
// No hay vista previa de la cámara. `camera-view` es una vista de Android y esta
// aplicación dibuja con Flutter, así que meterla exigiría una `PlatformView` —
// bastante más que una pantalla. La consecuencia es real y no se esconde:
// **quien sostiene el teléfono no ve lo que enfoca.**
//
// Lo que hay en su lugar es el recuento, que es el dato que sirve para actuar:
// «300 mirados, 2 leídos» significa acercar o enfocar; «300 y 280» significa que
// va bien. Una barra de progreso sola no distingue esas dos cosas.

import 'dart:async';
import 'dart:ffi';

import 'package:flutter/material.dart';

import 'package:qyro/scanner/qyro_scanner.dart';

/// Mira códigos hasta que el archivo esté entero.
class ScanScreen extends StatefulWidget {
  const ScanScreen({required this.library, super.key});

  /// La biblioteca nativa ya cargada. Se inyecta para que la pantalla se pueda
  /// probar sin un `.so`.
  final DynamicLibrary library;

  @override
  State<ScanScreen> createState() => _ScanScreenState();
}

class _ScanScreenState extends State<ScanScreen> {
  QyroScanner? _scanner;
  Timer? _pump;
  QyroScanTally _tally = const QyroScanTally(seen: 0, read: 0);
  QyroScanState? _last;
  String? _failure;
  int? _receivedBytes;

  /// Cada cuánto se le pide un frame a la cámara.
  ///
  /// 100 ms, o sea diez veces por segundo, contra los 5 a los que el otro lado
  /// dibuja (ADR-0044 §3). Pedir más a menudo no encuentra códigos nuevos y
  /// gasta batería; pedir menos se salta la mitad.
  static const _interval = Duration(milliseconds: 100);

  @override
  void initState() {
    super.initState();
    _begin();
  }

  @override
  void dispose() {
    _pump?.cancel();
    unawaited(_scanner?.close());
    super.dispose();
  }

  Future<void> _begin() async {
    try {
      final scanner = QyroScanner.open(widget.library);
      _scanner = scanner;
      await scanner.start();
      _pump = Timer.periodic(_interval, (_) => unawaited(_tick()));
    } on QyroScannerUnavailable catch (error) {
      if (!mounted) return;
      setState(() => _failure = error.reason);
    }
  }

  Future<void> _tick() async {
    final scanner = _scanner;
    if (scanner == null) return;
    try {
      final state = await scanner.pump();
      if (!mounted) return;
      setState(() {
        _tally = scanner.tally();
        if (state != null) _last = state;
      });
      if (state == QyroScanState.complete) {
        _pump?.cancel();
        final bytes = scanner.result();
        if (!mounted) return;
        setState(() => _receivedBytes = bytes?.length);
      }
    } on QyroScannerUnavailable catch (error) {
      _pump?.cancel();
      if (!mounted) return;
      setState(() => _failure = error.reason);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Leer códigos')),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (_failure != null)
              Text(
                'Este aparato no puede mirar: $_failure',
                key: const Key('scan-unavailable'),
              )
            else if (_receivedBytes != null)
              Text(
                'Recibido: $_receivedBytes bytes.',
                key: const Key('scan-complete'),
              )
            else ...<Widget>[
              const Text(
                'Apunta la cámara a los códigos de la otra pantalla.\n'
                'No hay vista previa: guíate por las cifras de abajo.',
                key: Key('scan-aiming-hint'),
              ),
              const SizedBox(height: 16),
              Text(
                '${_tally.seen} mirados · ${_tally.read} leídos',
                key: const Key('scan-tally'),
              ),
              if (_tally.looksMisaimed) ...<Widget>[
                const SizedBox(height: 8),
                const Text(
                  'Se ven pocos códigos. Acerca el teléfono, o baja el brillo '
                  'de la otra pantalla.',
                  key: Key('scan-misaimed'),
                ),
              ],
              const SizedBox(height: 16),
              if (_last == QyroScanState.progress ||
                  _last == QyroScanState.repeat)
                const LinearProgressIndicator(key: Key('scan-progress')),
            ],
            const Spacer(),
            OutlinedButton(
              key: const Key('scan-stop'),
              onPressed: () => Navigator.of(context).maybePop(),
              child: const Text('Parar'),
            ),
          ],
        ),
      ),
    );
  }
}
