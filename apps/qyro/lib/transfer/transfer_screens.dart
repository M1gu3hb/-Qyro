// The four screens of ADR-0036: peers, send, receive, history.
//
// They talk to `QyroTransferService` and to nothing else, so every state below
// — including the ugly ones, which are the ones that decide whether this is a
// product — can be driven in a widget test without a socket.

import 'dart:async';

import 'package:flutter/material.dart';

import 'package:qyro/discovery/qyro_discovery.dart';
import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/scanner/qyro_scanner.dart';
import 'package:qyro/ffi/qyro_trust_api.dart';
import 'package:qyro/l10n/generated/app_localizations.dart';
import 'package:qyro/transfer/transfer_service.dart';

/// Bytes, in something a person reads.
String humanBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  const units = <String>['KiB', 'MiB', 'GiB', 'TiB'];
  var value = bytes / 1024;
  var unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return '${value.toStringAsFixed(value >= 100 ? 0 : 1)} ${units[unit]}';
}

/// The shell: four destinations, and the transfer service they share.
class TransferHome extends StatefulWidget {
  const TransferHome({required this.service, this.initialTab = 0, super.key});

  final QyroTransferService service;

  /// Which destination opens first. The home buttons land on send or receive.
  final int initialTab;

  @override
  State<TransferHome> createState() => _TransferHomeState();
}

class _TransferHomeState extends State<TransferHome> {
  late int _index = widget.initialTab;

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    // **Three destinations, not four. QYR-0358.**
    //
    // `HistoryScreen` is built and works, and `service.history()` can never
    // return anything: `qyro_fs::history` records to disk and **no C symbol
    // reads it**, so the list is empty by construction. A tab that can never
    // show anything is a promise, and R7 §5 says Qyro does not carry features
    // nobody asked for.
    //
    // The engine keeps recording, so nothing is lost and nothing is deleted:
    // the screen comes back the day a symbol reads the file. Retiring it is
    // cheaper to undo than a fourth tab that lies is to explain.
    final pages = <Widget>[
      PeersScreen(service: widget.service),
      SendScreen(service: widget.service),
      ReceiveScreen(service: widget.service),
    ];

    return Scaffold(
      appBar: AppBar(title: Text(strings.appTitle)),
      body: SafeArea(child: pages[_index]),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _index,
        onDestinationSelected: (next) => setState(() => _index = next),
        destinations: <NavigationDestination>[
          NavigationDestination(
            icon: const Icon(Icons.devices),
            label: strings.navPeers,
          ),
          NavigationDestination(
            icon: const Icon(Icons.upload_file),
            label: strings.navSend,
          ),
          NavigationDestination(
            icon: const Icon(Icons.download),
            label: strings.navReceive,
          ),
        ],
      ),
    );
  }
}

// ------------------------------------------------------------------ peers

class PeersScreen extends StatefulWidget {
  const PeersScreen({
    required this.service,
    this.discovery,
    this.onScan,
    super.key,
  });

  /// Que hacer cuando alguien quiere leer codigos.
  ///
  /// Se inyecta en vez de abrir la pantalla aqui porque abrirla necesita la
  /// biblioteca nativa cargada, y esta pantalla se prueba sin ella.
  final VoidCallback? onScan;

  final QyroTransferService service;

  /// Injected so the screen can be tested without a platform channel, and left
  /// null in production so the routing in [discoveryForPlatform] is the thing
  /// that ships. A screen that only worked with a fake would be a screen nobody
  /// had run.
  final QyroDiscovery? discovery;

  @override
  State<PeersScreen> createState() => _PeersScreenState();
}

class _PeersScreenState extends State<PeersScreen> {
  final TextEditingController _pairing = TextEditingController();
  List<QyroPeerEntry> _peers = const <QyroPeerEntry>[];
  String? _ownCode;
  String? _pairingError;
  String? _resolvedAddress;

  late final QyroDiscovery _discovery =
      widget.discovery ?? discoveryForPlatform();
  List<QyroFoundPeer> _nearby = const <QyroFoundPeer>[];
  bool _looking = false;
  String? _discoveryError;

  @override
  void initState() {
    super.initState();
    _reload();
    _look();
  }

  /// Announces this device and asks who else is here.
  ///
  /// **This is the production caller `dev.qyro/discovery` never had.**
  /// `DiscoveryChannel.kt` has been registered since phase 04b and no Dart had
  /// opened it, which is the same defect this project found four times before.
  ///
  /// Failure here is never fatal to the screen: the pairing code above works on
  /// every network and is the reason it was built first (ADR-0036 §3).
  Future<void> _look() async {
    setState(() {
      _looking = true;
      _discoveryError = null;
    });
    try {
      final own = await widget.service.ownPairingString();
      final fingerprint = own?.split('|').last;
      if (fingerprint != null && fingerprint.isNotEmpty) {
        await _discovery.advertise(
          port: qyroDefaultPort,
          fingerprint: fingerprint,
        );
      }
      final found = await _discovery.browse();
      if (!mounted) return;
      setState(() {
        _nearby = found;
        _looking = false;
      });
    } on QyroDiscoveryUnavailable catch (error) {
      // Said out loud, not swallowed into an empty list. "This device cannot
      // look" and "nobody is there" are different sentences, and a person shown
      // the second when it was the first concludes the other device is off.
      if (!mounted) return;
      setState(() {
        _discoveryError = error.reason;
        _looking = false;
      });
    }
  }

  @override
  void dispose() {
    _pairing.dispose();
    super.dispose();
  }

  Future<void> _reload() async {
    final peers = await widget.service.knownPeers();
    final own = await widget.service.ownPairingString();
    if (!mounted) return;
    setState(() {
      _peers = peers;
      _ownCode = own;
    });
  }

  Future<void> _resolve() async {
    final strings = AppLocalizations.of(context);
    final address =
        await widget.service.addressOfPairingString(_pairing.text.trim());
    if (!mounted) return;
    setState(() {
      _resolvedAddress = address;
      _pairingError = address == null ? strings.peersManualInvalid : null;
    });
  }

  Future<void> _forget(QyroPeerEntry peer) async {
    await widget.service.forgetPeer(peer.name);
    await _reload();
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        // Always visible, never behind "advanced": this is the only path that
        // works in every network (ADR-0036 §3).
        Text(strings.peersManualHint),
        const SizedBox(height: 8),
        TextField(
          key: const Key('pairing-field'),
          controller: _pairing,
          decoration: InputDecoration(
            labelText: strings.peersManualLabel,
            errorText: _pairingError,
          ),
          onSubmitted: (_) => _resolve(),
        ),
        const SizedBox(height: 8),
        Row(
          children: <Widget>[
            // QYR-0348. This button used to carry `Icons.qr_code_scanner` and
            // the label "Scan a code", and what it does is parse the text
            // field above it. There is no camera, no QR decoder and no plugin
            // that could provide either -- an icon that promises a scanner is
            // the same lie as a button that says it works when it does not,
            // which is the thing phase 05 spent a day removing.
            OutlinedButton.icon(
              key: const Key('pairing-scan'),
              onPressed: _resolve,
              icon: const Icon(Icons.link),
              label: Text(strings.peersUseCode),
            ),
            const SizedBox(width: 12),
            if (_resolvedAddress != null)
              Expanded(
                child: Text(
                  _resolvedAddress!,
                  key: const Key('pairing-address'),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
          ],
        ),
        const Divider(height: 32),
        Text(strings.peersOwnCode,
            style: Theme.of(context).textTheme.titleMedium),
        const SizedBox(height: 4),
        SelectableText(
          _ownCode ?? strings.peersOwnCodeUnavailable,
          key: const Key('own-pairing-code'),
          style: const TextStyle(fontFamily: 'monospace'),
        ),
        const Divider(height: 32),
        // **El llamante de produccion del escaner** (ADR-0048, fase 24B). El
        // canal optico es el unico que funciona sin red de ninguna clase, y sin
        // este boton la aplicacion lo tenia entero y sin puerta.
        //
        // Solo en Android: en el escritorio quien DIBUJA los QR es el CLI
        // (ADR-0044 §6), y ofrecer aqui un escaner que no existe seria prometer
        // lo que no hay.
        if (scannerAvailableOn())
          OutlinedButton.icon(
            key: const Key('scan-open'),
            onPressed: widget.onScan,
            icon: const Icon(Icons.qr_code_scanner),
            label: Text(strings.peersScanCodes),
          ),
        if (scannerAvailableOn()) const Divider(height: 32),
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                strings.peersNearbyTitle,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            IconButton(
              key: const Key('nearby-refresh'),
              onPressed: _looking ? null : _look,
              icon: const Icon(Icons.refresh),
              tooltip: strings.peersNearbyLooking,
            ),
          ],
        ),
        const SizedBox(height: 4),
        if (_discoveryError != null)
          Text(
            strings.peersNearbyUnavailable,
            key: const Key('nearby-unavailable'),
          )
        else if (_looking)
          Text(strings.peersNearbyLooking, key: const Key('nearby-looking'))
        else if (_nearby.isEmpty)
          Text(strings.peersNearbyNone, key: const Key('nearby-empty'))
        else
          ..._nearby.map(
            (found) => ListTile(
              key: Key('nearby-${found.fingerprint}'),
              leading: const Icon(Icons.lan_outlined),
              title: Text(found.address),
              subtitle: Text(
                found.fingerprint,
                style: const TextStyle(fontFamily: 'monospace'),
              ),
              trailing: TextButton(
                // It fills the pairing field rather than dialling: what a
                // discovered device offers is a *code*, and the person still
                // decides. The trust check that follows is the same one a typed
                // code goes through, because a device that announced itself has
                // proved nothing yet.
                onPressed: () {
                  _pairing.text = found.pairingCode;
                  _resolve();
                },
                child: Text(strings.peersNearbyUse),
              ),
            ),
          ),
        const Divider(height: 32),
        if (_peers.isEmpty)
          Text(strings.peersNone, key: const Key('peers-empty'))
        else
          ..._peers.map(
            (peer) => PeerTile(peer: peer, onForget: () => _forget(peer)),
          ),
      ],
    );
  }
}

/// One peer. A changed key does not look like the others (ADR-0036 §4).
class PeerTile extends StatelessWidget {
  const PeerTile({required this.peer, required this.onForget, super.key});

  final QyroPeerEntry peer;
  final VoidCallback onForget;

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final alarming = peer.trust == QyroPeerTrust.changed;

    final label = switch (peer.trust) {
      QyroPeerTrust.known => strings.peersTrustKnown,
      QyroPeerTrust.changed => strings.peersTrustChanged,
      QyroPeerTrust.newPeer => strings.peersTrustNew,
    };

    return Card(
      key: Key('peer-${peer.name}'),
      // Error colour, not a warning tint: a danger that looks like the rest of
      // the list is a danger nobody reads.
      color: alarming ? theme.colorScheme.errorContainer : null,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Icon(
                  alarming ? Icons.gpp_bad : Icons.verified_user,
                  color: alarming ? theme.colorScheme.error : null,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    safeDisplayName(peer.name),
                    style: theme.textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              label,
              key: Key('peer-trust-${peer.name}'),
              style: TextStyle(
                color: alarming ? theme.colorScheme.error : null,
                fontWeight: alarming ? FontWeight.bold : null,
              ),
            ),
            if (alarming) ...<Widget>[
              const SizedBox(height: 4),
              Text(
                strings.peersChangedExplain,
                key: Key('peer-changed-explain-${peer.name}'),
              ),
            ],
            const SizedBox(height: 8),
            SelectableText(
              peer.fingerprint,
              key: Key('peer-fingerprint-${peer.name}'),
              style: const TextStyle(fontFamily: 'monospace'),
            ),
            const SizedBox(height: 4),
            Text(strings.fingerprintCompare, style: theme.textTheme.bodySmall),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerRight,
              child: TextButton.icon(
                key: Key('peer-forget-${peer.name}'),
                onPressed: onForget,
                icon: const Icon(Icons.delete_outline),
                label: Text(strings.peersForget),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ------------------------------------------------------------------- send

class SendScreen extends StatefulWidget {
  const SendScreen({required this.service, super.key});

  final QyroTransferService service;

  @override
  State<SendScreen> createState() => _SendScreenState();
}

class _SendScreenState extends State<SendScreen> {
  final TextEditingController _address = TextEditingController();
  List<QyroPicked> _chosen = const <QyroPicked>[];
  QyroTransferState _state = const QyroIdle();

  @override
  void dispose() {
    _address.dispose();
    super.dispose();
  }

  Future<void> _pick() async {
    final picked = await widget.service.pickFiles();
    if (!mounted) return;
    setState(() => _chosen = picked);
  }

  Future<void> _send() async {
    final stream = widget.service.send(
      address: _address.text.trim(),
      files: _chosen,
    );
    await for (final state in stream) {
      if (!mounted) return;
      setState(() => _state = state);
    }
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final totalBytes = _chosen.fold<int>(
      0,
      (sum, file) => sum + (file.size < 0 ? 0 : file.size),
    );

    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        TextField(
          key: const Key('send-address'),
          controller: _address,
          decoration: InputDecoration(labelText: strings.peersManualLabel),
        ),
        const SizedBox(height: 12),
        FilledButton.icon(
          key: const Key('send-pick'),
          onPressed: _pick,
          icon: const Icon(Icons.attach_file),
          label: Text(strings.sendChoose),
        ),
        const SizedBox(height: 12),
        if (_chosen.isEmpty)
          Text(strings.sendNoFiles, key: const Key('send-no-files'))
        else ...<Widget>[
          Text(
            strings.sendChosen(
              '${_chosen.length}',
              humanBytes(totalBytes),
            ),
            key: const Key('send-chosen'),
          ),
          const SizedBox(height: 4),
          ..._chosen.map(
            (file) => Text(
              safeDisplayName(file.name),
              // A name that came from somewhere else is drawn left-to-right,
              // whatever it says it wants (ADR-0036 §2).
              textDirection: TextDirection.ltr,
            ),
          ),
        ],
        const SizedBox(height: 16),
        FilledButton(
          key: const Key('send-start'),
          onPressed:
              _chosen.isEmpty || _address.text.trim().isEmpty ? null : _send,
          child: Text(strings.navSend),
        ),
        const SizedBox(height: 24),
        TransferStatus(state: _state, key: const Key('send-status')),
      ],
    );
  }
}

// ---------------------------------------------------------------- receive

class ReceiveScreen extends StatefulWidget {
  const ReceiveScreen({required this.service, super.key});

  final QyroTransferService service;

  @override
  State<ReceiveScreen> createState() => _ReceiveScreenState();
}

class _ReceiveScreenState extends State<ReceiveScreen> {
  QyroTransferState _state = const QyroIdle();
  QyroAwaitingDecision? _offer;

  /// The codes the other device has to be given.
  ///
  /// QYR-0322, and this is the visible half of the fix. They are loaded when
  /// the screen opens, **before** anything is bound and before any peer
  /// connects, because the port is known in advance (ADR-0041 §3). Until phase
  /// 12 this screen showed nothing at all and the peers screen said "no code to
  /// show", while the other device's screen asked for that very code.
  List<QyroListenAddress> _candidates = const <QyroListenAddress>[];

  @override
  void initState() {
    super.initState();
    unawaited(_loadCandidates());
  }

  Future<void> _loadCandidates() async {
    final found = await widget.service.listenCandidates();
    if (!mounted) return;
    setState(() => _candidates = found);
  }

  /// Asked before a single byte is accepted.
  ///
  /// ADR-0036 §1: **nothing is accepted on its own.** This completer is
  /// resolved by a person tapping and by nothing else — there is no timeout
  /// that accepts out of tiredness, and no "remember this decision" that turns
  /// one yes into a rule.
  Future<bool> _decide(QyroAwaitingDecision offer) {
    final answer = Completer<bool>();
    setState(() {
      _offer = offer;
      _pending = answer;
    });
    return answer.future;
  }

  Completer<bool>? _pending;

  Future<void> _listen() async {
    final stream = widget.service.receive(
      // ADR-0041 §3. Not `:0`: an ephemeral port is a port nobody can compose
      // into a code before the socket exists, and it costs a firewall dialog
      // every session. `0.0.0.0` and not one interface, because the person may
      // be reached on any of them and the code names which one (§4).
      bind: '0.0.0.0:$qyroDefaultPort',
      destination: '',
      decide: _decide,
    );
    await for (final state in stream) {
      if (!mounted) return;
      setState(() => _state = state);
    }
  }

  void _answer({required bool accept}) {
    final pending = _pending;
    if (pending != null && !pending.isCompleted) {
      pending.complete(accept);
    }
    setState(() {
      _pending = null;
      _offer = null;
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final offer = _offer;

    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        FilledButton.icon(
          key: const Key('receive-start'),
          onPressed: _listen,
          icon: const Icon(Icons.download),
          label: Text(strings.receiveStart),
        ),
        const SizedBox(height: 16),
        Text(
          strings.receiveYourCode,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 4),
        if (_candidates.isEmpty)
          // A real state, not an empty list rendered as nothing: a machine
          // still waiting for APIPA has no address for tens of seconds
          // (R8 §8), and saying so beats a blank space.
          Text(
            strings.receiveNoAddress,
            key: const Key('receive-no-address'),
          )
        else ...<Widget>[
          if (_candidates.length > 1)
            // ADR-0041 §4: several addresses means the person picks, because
            // they know which network they are on and this program does not.
            Text(
              strings.receiveSeveralAddresses,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ..._candidates.map(
            (candidate) => Padding(
              key: Key('receive-code-${candidate.interfaceName}'),
              padding: const EdgeInsets.only(top: 8),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    candidate.interfaceName,
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                  SelectableText(
                    candidate.pairingString,
                    style: const TextStyle(fontFamily: 'monospace'),
                  ),
                ],
              ),
            ),
          ),
        ],
        const SizedBox(height: 16),
        if (offer != null)
          OfferCard(
            offer: offer,
            onAccept: () => _answer(accept: true),
            onRefuse: () => _answer(accept: false),
          ),
        const SizedBox(height: 16),
        TransferStatus(state: _state, key: const Key('receive-status')),
      ],
    );
  }
}

/// What the receiver sees before a single byte is accepted.
///
/// ADR-0036 §2: who, how many, how much, and what they are called.
class OfferCard extends StatelessWidget {
  const OfferCard({
    required this.offer,
    required this.onAccept,
    required this.onRefuse,
    super.key,
  });

  final QyroAwaitingDecision offer;
  final VoidCallback onAccept;
  final VoidCallback onRefuse;

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final alarming = offer.trust == QyroPeerTrust.changed;

    return Card(
      key: const Key('offer-card'),
      color: alarming ? theme.colorScheme.errorContainer : null,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              strings.receiveOfferFrom(
                '${offer.fileCount}',
                humanBytes(offer.totalBytes),
              ),
              key: const Key('offer-summary'),
              style: theme.textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            SelectableText(
              offer.fingerprint,
              key: const Key('offer-fingerprint'),
              style: const TextStyle(fontFamily: 'monospace'),
            ),
            const SizedBox(height: 4),
            Text(
              switch (offer.trust) {
                QyroPeerTrust.known => strings.peersTrustKnown,
                QyroPeerTrust.changed => strings.peersTrustChanged,
                QyroPeerTrust.newPeer => strings.receiveOfferUnknown,
              },
              key: const Key('offer-trust'),
              style: TextStyle(
                color: alarming ? theme.colorScheme.error : null,
                fontWeight: alarming ? FontWeight.bold : null,
              ),
            ),
            const SizedBox(height: 8),
            ...offer.fileNames.map(
              (name) => Text(
                safeDisplayName(name),
                textDirection: TextDirection.ltr,
              ),
            ),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: <Widget>[
                TextButton(
                  key: const Key('offer-refuse'),
                  onPressed: onRefuse,
                  child: Text(strings.receiveRefuse),
                ),
                const SizedBox(width: 12),
                // Absent, not disabled, when the key changed: there is no
                // "continue anyway" (ADR-0036 §1).
                if (!alarming)
                  FilledButton(
                    key: const Key('offer-accept'),
                    onPressed: onAccept,
                    child: Text(strings.receiveAccept),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// -------------------------------------------------------------- the status

/// Every transfer state, including the ugly ones, with its own sentence.
class TransferStatus extends StatelessWidget {
  const TransferStatus({required this.state, super.key});

  final QyroTransferState state;

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final theme = Theme.of(context);

    return switch (state) {
      QyroIdle() => const SizedBox.shrink(),
      QyroConnecting() => Row(
          children: <Widget>[
            const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
            const SizedBox(width: 12),
            Text(strings.receiveWaiting),
          ],
        ),
      QyroAwaitingDecision() => Text(strings.receiveWaiting),
      QyroMoving(:final done, :final total, :final fraction) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            LinearProgressIndicator(value: fraction),
            const SizedBox(height: 8),
            Text(
              strings.progressOf(humanBytes(done), humanBytes(total)),
              key: const Key('status-progress'),
            ),
          ],
        ),
      QyroDelivered(:final fileCount, :final destination) => Text(
          strings.receiveDelivered('$fileCount', destination),
          key: const Key('status-delivered'),
        ),
      QyroFailed(:final kind, :final reason) => Text(
          _failureText(strings, kind, reason),
          key: const Key('status-failed'),
          style: TextStyle(color: theme.colorScheme.error),
        ),
    };
  }

  static String _failureText(
    AppLocalizations strings,
    QyroFailureKind kind,
    QyroRejectReason? reason,
  ) =>
      switch (kind) {
        QyroFailureKind.unreachable => strings.sendUnreachable,
        QyroFailureKind.keyChanged => strings.sendKeyChanged,
        QyroFailureKind.refusedByPeer =>
          strings.sendRefused(_reasonText(strings, reason)),
        QyroFailureKind.refusedByMe => strings.receiveRefused,
        QyroFailureKind.integrity => strings.sendIntegrity,
        QyroFailureKind.cancelled => strings.sendCancelledByUser,
        QyroFailureKind.noRoom => strings.receiveNoRoom,
        QyroFailureKind.tooManyFiles => strings.sendTooManyFiles,
      };

  static String _reasonText(
    AppLocalizations strings,
    QyroRejectReason? reason,
  ) =>
      switch (reason) {
        QyroRejectReason.declined => strings.reasonDeclined,
        QyroRejectReason.noRoom => strings.reasonNoRoom,
        QyroRejectReason.unacceptableManifest => strings.reasonManifest,
        QyroRejectReason.unspecified || null => strings.reasonUnspecified,
      };
}

// ---------------------------------------------------------------- history

class HistoryScreen extends StatefulWidget {
  const HistoryScreen({required this.service, super.key});

  final QyroTransferService service;

  @override
  State<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends State<HistoryScreen> {
  List<QyroHistoryEntry>? _entries;

  @override
  void initState() {
    super.initState();
    widget.service.history().then((entries) {
      if (mounted) setState(() => _entries = entries);
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = AppLocalizations.of(context);
    final entries = _entries;
    if (entries == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (entries.isEmpty) {
      return Center(
        child: Text(strings.historyEmpty, key: const Key('history-empty')),
      );
    }
    return ListView.builder(
      itemCount: entries.length,
      itemBuilder: (context, index) {
        final entry = entries[index];
        return ListTile(
          key: Key('history-$index'),
          leading: Icon(entry.outgoing ? Icons.north_east : Icons.south_west),
          title: Text(
            safeDisplayName(entry.name),
            textDirection: TextDirection.ltr,
          ),
          subtitle: Text(
            '${safeDisplayName(entry.peer)} · ${humanBytes(entry.bytes)}',
          ),
          trailing: Text(
            entry.succeeded ? strings.historySucceeded : strings.historyFailed,
            style: TextStyle(
              color:
                  entry.succeeded ? null : Theme.of(context).colorScheme.error,
            ),
          ),
        );
      },
    );
  }
}
