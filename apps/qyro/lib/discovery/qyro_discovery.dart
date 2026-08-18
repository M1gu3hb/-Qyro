// The Dart side of `dev.qyro/discovery`.
//
// `DiscoveryChannel.kt` has existed since phase 04b, registered, working, with
// `NsdManager` and a multicast lock and a fingerprint check -- and **no Dart has
// ever opened its channel**. It is the fourth capability this project has found
// written, tested and unreachable from the product, after `KeystoreWrapper`,
// `qyro_session_local_address` and `Session::finish`.
//
// ADR-0043 §5: connect what exists, do not rewrite it. So this file is thin on
// purpose. Everything it could have re-decided -- the service type, deriving the
// instance name from the fingerprint so a device name never leaks, dropping a
// service whose TXT record has no valid fingerprint -- is already decided on the
// Kotlin side and stays there.

import 'dart:io';

import 'package:flutter/services.dart';

/// A device that answered, and everything needed to reach it.
///
/// The same two fields `qyro_session::FoundPeer` carries, because they are the
/// same fact arriving by a different road.
final class QyroFoundPeer {
  const QyroFoundPeer({required this.address, required this.fingerprint});

  /// `host:port`, exactly as the platform resolved it.
  final String address;

  /// Thirty-two lowercase hex characters.
  final String fingerprint;

  /// The pairing code this peer would show.
  ///
  /// Assembled from the announced fields rather than carried over the channel:
  /// the Kotlin side has no business knowing the code format, and a second
  /// place that builds this string is a second place that drifts from
  /// ADR-0035.
  String get pairingCode => 'QYRO1|$address|$fingerprint';

  @override
  bool operator ==(Object other) =>
      other is QyroFoundPeer &&
      other.address == address &&
      other.fingerprint == fingerprint;

  @override
  int get hashCode => Object.hash(address, fingerprint);

  @override
  String toString() => 'QyroFoundPeer($address, $fingerprint)';
}

/// Why a browse produced nothing, when the reason is not "nobody is there".
///
/// **The distinction the file picker taught this codebase** (ADR-0034
/// amendment 1): an empty list from a platform that cannot ask is
/// indistinguishable from an empty list from a quiet network, and a person
/// shown the second when it was the first concludes the other device is off.
final class QyroDiscoveryUnavailable implements Exception {
  const QyroDiscoveryUnavailable(this.reason);

  final String reason;

  @override
  String toString() => 'QyroDiscoveryUnavailable: $reason';
}

/// Announcing this device and finding others, per platform.
abstract interface class QyroDiscovery {
  /// Publishes this device so others can find it.
  ///
  /// Throws [QyroDiscoveryUnavailable] when the platform cannot announce.
  Future<void> advertise({required int port, required String fingerprint});

  /// Returns whoever has answered so far.
  ///
  /// **A snapshot, not a stream.** The platform keeps browsing after this
  /// returns, so calling it again a second later returns more; that polling
  /// shape is the Kotlin side's decision (a callback from an arbitrary NSD
  /// thread would need its own lifecycle) and this does not second-guess it.
  ///
  /// An empty list means **nobody answered yet**, which is a true statement
  /// about this network. Anything else throws.
  Future<List<QyroFoundPeer>> browse();

  /// Stops announcing and browsing, and releases the multicast lock.
  Future<void> stop();
}

/// Android: `NsdManager` over our own channel.
final class QyroAndroidDiscovery implements QyroDiscovery {
  const QyroAndroidDiscovery([this._channel = _defaultChannel]);

  static const _defaultChannel = MethodChannel('dev.qyro/discovery');
  final MethodChannel _channel;

  @override
  Future<void> advertise({
    required int port,
    required String fingerprint,
  }) async {
    try {
      await _channel.invokeMethod<void>('advertise', <String, Object>{
        'port': port,
        'fingerprint': fingerprint,
      });
    } on PlatformException catch (error) {
      // `unavailable` is the device saying it has no NsdManager, and
      // `bad_argument` is the channel refusing to announce a malformed record.
      // Both are answers, not crashes -- the typed pairing code still works,
      // which is why it was built first.
      throw QyroDiscoveryUnavailable(error.message ?? error.code);
    }
  }

  @override
  Future<List<QyroFoundPeer>> browse() async {
    final List<Map<Object?, Object?>>? raw;
    try {
      raw = await _channel.invokeListMethod<Map<Object?, Object?>>('browse');
    } on PlatformException catch (error) {
      throw QyroDiscoveryUnavailable(error.message ?? error.code);
    }
    if (raw == null) {
      return const <QyroFoundPeer>[];
    }

    final peers = <QyroFoundPeer>[];
    final seen = <String>{};
    for (final entry in raw) {
      final address = entry['address'];
      final fingerprint = entry['fingerprint'];
      if (address is! String || fingerprint is! String) {
        // A malformed entry is dropped rather than shown blank. The Kotlin side
        // already refuses services without a valid fingerprint; this is the
        // second wall, because the channel is a boundary and a boundary that
        // trusts its input is not a boundary.
        continue;
      }
      // Deduplicated by fingerprint, never by address (ADR-0043 §5). A device
      // reachable on two interfaces announces twice, and the same machine
      // listed twice reads as "someone is impersonating them" rather than "it
      // has Wi-Fi and a cable".
      if (!seen.add(fingerprint)) {
        continue;
      }
      peers.add(
        QyroFoundPeer(address: address, fingerprint: fingerprint),
      );
    }
    return List<QyroFoundPeer>.unmodifiable(peers);
  }

  @override
  Future<void> stop() async {
    try {
      await _channel.invokeMethod<void>('stop');
    } on PlatformException {
      // Stopping something that was never started, or on a device with no
      // NsdManager, is not a failure worth propagating: the caller asked for
      // the browse to be over and it is over.
    }
  }
}

/// A platform with no discovery, which says so instead of returning nothing.
///
/// Windows is this today. `qyro_ffi` exposes no discovery symbol — the engine
/// has `qyro_session::browse` and the C frontier does not carry it, which
/// `deuda-de-calidad.md` records as declared outside v1.x rather than missing.
/// Until a symbol exists, the desktop GUI's honest answer is that it cannot
/// ask, and the `qyro` CLI is the consumer that has discovery on this platform.
final class QyroNoDiscovery implements QyroDiscovery {
  const QyroNoDiscovery(this.reason);

  final String reason;

  // `async` is load-bearing, not decoration. Written as `=> throw ...` these
  // threw **synchronously** from a method declared to return a `Future`, so a
  // caller that handled failures with `.catchError` — the shape an interface
  // like this invites — would never see it, and the exception would escape as
  // an unhandled error somewhere else entirely. The test caught it before any
  // caller existed.
  @override
  Future<void> advertise({
    required int port,
    required String fingerprint,
  }) async =>
      throw QyroDiscoveryUnavailable(reason);

  @override
  Future<List<QyroFoundPeer>> browse() async =>
      throw QyroDiscoveryUnavailable(reason);

  @override
  Future<void> stop() async {}
}

/// The discovery this platform has.
///
/// [operatingSystem] defaults to the host's and exists so the routing itself is
/// testable from any machine — a test that only runs on the platform it asserts
/// about is a test that never runs.
QyroDiscovery discoveryForPlatform({String? operatingSystem}) {
  final os = operatingSystem ?? Platform.operatingSystem;
  return switch (os) {
    'android' => const QyroAndroidDiscovery(),
    'windows' => const QyroNoDiscovery(
        'the desktop build has no discovery symbol at the C frontier yet; '
        'use the pairing code, or the qyro command line',
      ),
    _ => QyroNoDiscovery('$os is not a Qyro platform'),
  };
}
