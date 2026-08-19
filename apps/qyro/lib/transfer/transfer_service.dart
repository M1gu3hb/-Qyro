// What the screens are allowed to ask for, and the words they get back.
//
// Specification: docs/adr/ADR-0036-transfer-ui.md.
//
// An interface rather than the FFI directly, for one reason that is not
// tidiness: `QyroSession.stepBlocking` blocks without a bound (ADR-0032 §7), so
// the real implementation runs it on another isolate. A screen that called the
// boundary directly would freeze the frame, and a screen that could only be
// tested with a live socket would not be tested.

import 'package:qyro/ffi/qyro_file_picker.dart';
import 'package:qyro/ffi/qyro_trust_api.dart';

/// One peer this device has been told to remember.
final class QyroPeerEntry {
  const QyroPeerEntry({
    required this.name,
    required this.fingerprint,
    required this.trust,
  });

  final String name;

  /// Grouped hex, **as the core formatted it**. Never reformatted here: two
  /// devices rendering it differently makes comparing it out loud worthless.
  final String fingerprint;

  final QyroPeerTrust trust;

  /// Whether this peer may be sent to.
  ///
  /// A changed key is not a warning with a button behind it (ADR-0036 §4): the
  /// action is absent, not greyed out.
  bool get isSendable => trust != QyroPeerTrust.changed;
}

/// One line of the local history.
final class QyroHistoryEntry {
  const QyroHistoryEntry({
    required this.name,
    required this.peer,
    required this.bytes,
    required this.succeeded,
    required this.outgoing,
  });

  final String name;
  final String peer;
  final int bytes;
  final bool succeeded;
  final bool outgoing;
}

/// Where a transfer is. Sealed, so a screen cannot forget a state.
sealed class QyroTransferState {
  const QyroTransferState();
}

/// Nothing has started.
final class QyroIdle extends QyroTransferState {
  const QyroIdle();
}

/// Dialling, handshaking, or waiting for a peer to connect.
final class QyroConnecting extends QyroTransferState {
  const QyroConnecting();
}

/// A peer has authenticated and is waiting for a decision.
///
/// ADR-0036 §1 and §2: **nothing is accepted on its own**, and the four facts a
/// person needs are all here before the first byte.
final class QyroAwaitingDecision extends QyroTransferState {
  const QyroAwaitingDecision({
    required this.fingerprint,
    required this.trust,
    required this.fileNames,
    required this.totalBytes,
  });

  final String fingerprint;
  final QyroPeerTrust trust;
  final List<String> fileNames;
  final int totalBytes;

  int get fileCount => fileNames.length;
}

/// Bytes are moving.
final class QyroMoving extends QyroTransferState {
  const QyroMoving({
    required this.done,
    required this.total,
    required this.fingerprint,
  });

  final int done;
  final int total;
  final String fingerprint;

  /// `0.0`–`1.0`, or null when the total is not known yet.
  ///
  /// Null rather than zero: a bar sitting at zero says «nothing has happened»,
  /// and «I do not know how much there is» is a different fact.
  double? get fraction => total > 0 ? (done / total).clamp(0.0, 1.0) : null;
}

/// It finished, and everything verified.
final class QyroDelivered extends QyroTransferState {
  const QyroDelivered({required this.fileCount, required this.destination});

  final int fileCount;

  /// Where the files are, because «done» without «where» is not an answer.
  final String destination;
}

/// It ended without delivering, and the reason is carried, not summarised.
final class QyroFailed extends QyroTransferState {
  const QyroFailed({required this.kind, this.reason});

  final QyroFailureKind kind;

  /// Set only when the far end said why (ADR-0035, QYR-0089).
  final QyroRejectReason? reason;
}

/// Why a transfer did not deliver.
enum QyroFailureKind {
  /// The address answered nothing.
  unreachable,

  /// The peer's key is not the one this device remembers under that name.
  keyChanged,

  /// The far end refused. [QyroFailed.reason] says what it said.
  refusedByPeer,

  /// This end refused.
  refusedByMe,

  /// Something arrived that did not verify.
  integrity,

  /// Somebody stopped it.
  cancelled,

  /// Nowhere to put it.
  noRoom,

  /// Se eligieron mas archivos de los que caben en una transferencia.
  ///
  /// ADR-0047 §3 lo fija en 256 y la razon son los descriptores. Es un limite
  /// que una persona puede corregir —mandar menos— asi que merece decirlo en vez
  /// de salir como «algo no verifico».
  tooManyFiles,
}

/// Everything a screen may ask of the engine.
///
/// The port a Qyro receiver listens on, unless a person names another.
///
/// ADR-0041 §3. From IANA's **Dynamic/Private** range, 49152-65535, which IANA
/// states it never assigns to a registered service -- so this cannot collide by
/// registration with anything, only by coincidence with another program that
/// also picked at random.
///
/// **Fixed rather than ephemeral, and the reason is the firewall.** Windows
/// blocks inbound by default and a gateway-less link -- the direct cable of
/// phase 14 -- is classified Public, the most restrictive profile (R8 §9). The
/// permission is granted **once per program and port**: with a fixed port a
/// person authorises Qyro once and never sees the dialog again; with an
/// ephemeral port it returns every session, on the machine where it is least
/// welcome.
///
/// It also makes the pairing string predictable, which is what lets it be
/// composed **before** the socket is bound -- and that is what dissolves
/// QYR-0322 instead of answering it.
const int qyroDefaultPort = 49517;

/// One address this device could be reached at, with the interface it belongs to.
///
/// ADR-0041 §4. A device has several addresses and guessing produces a code
/// that does not work and does not say why, so every candidate is shown with
/// its interface name and a person picks the one whose network they are on.
final class QyroListenAddress {
  const QyroListenAddress({
    required this.interfaceName,
    required this.address,
    required this.pairingString,
  });

  /// What the operating system calls the interface: `Wi-Fi`, `wlan0`, `eth0`.
  final String interfaceName;

  /// `host:port`, the literal that goes on the wire.
  final String address;

  /// The whole `QYRO1|host:port|fingerprint`, ready to read out or type in.
  final String pairingString;
}

/// Every method is `Future` or `Stream` even where a real implementation could
/// answer at once: the real one crosses an isolate, and a signature that is
/// synchronous for the fake and asynchronous for production is a signature that
/// tests a different program.
abstract interface class QyroTransferService {
  /// The peers this device remembers.
  Future<List<QyroPeerEntry>> knownPeers();

  /// Forgets a peer. The only way back from a changed key.
  Future<bool> forgetPeer(String name);

  /// The address inside a pairing string, or null if it is not one of ours.
  Future<String?> addressOfPairingString(String text);

  /// The pairing string this device would show, or null before it has one.
  ///
  /// Null until this device is listening. ADR-0035 §2: a code that names no
  /// listener is a code that does not work, and showing one is worse than
  /// showing none. Until phase 12 this returned null **always**, because the
  /// address half was read from a field nothing ever assigned (QYR-0322).
  Future<String?> ownPairingString();

  /// Every address this device could be reached at, ready to read aloud.
  ///
  /// ADR-0041 §4. Composed from the enumerated interfaces and
  /// [qyroDefaultPort], so it answers **before** anything is bound and before
  /// any peer connects. Empty when this device has no usable address -- which
  /// is a real state on a machine still waiting for APIPA (R8 §8) and is shown
  /// as itself rather than as an empty list of nothing.
  Future<List<QyroListenAddress>> listenCandidates();

  /// Opens the system picker.
  Future<List<QyroPicked>> pickFiles();

  /// Sends [files] to [address]. The stream ends on a terminal state.
  Stream<QyroTransferState> send({
    required String address,
    required List<QyroPicked> files,
    String? expectedFingerprint,
  });

  /// Waits for one transfer. [decide] is asked before a byte is accepted.
  Stream<QyroTransferState> receive({
    required String bind,
    required String destination,
    required Future<bool> Function(QyroAwaitingDecision offer) decide,
  });

  /// The local history, newest first.
  Future<List<QyroHistoryEntry>> history();
}

/// Renders a name that came from somewhere else.
///
/// ADR-0036 §2. The manifest layer refuses a name that would escape the
/// destination; **that protects the disk and not the screen**. A right-to-left
/// override inside a file name reorders the line it is drawn in, so
/// `invoice<RLO>fdp.exe` reads as `invoiceexe.pdf` — the same attack this
/// project already closed once for the filesystem, arriving through the other
/// door.
///
/// What this does, and it is deliberately blunt: drops every Unicode
/// bidirectional control and every C0/C1 control, and refuses to return empty.
/// It does **not** try to detect confusable scripts — homoglyphs are legitimately
/// different names and guessing would be worse than showing them.
String safeDisplayName(String raw) {
  const bidiControls = <int>[
    0x061C, 0x200E, 0x200F, // marks
    0x202A, 0x202B, 0x202C, 0x202D, 0x202E, // embedding and override
    0x2066, 0x2067, 0x2068, 0x2069, // isolates
  ];

  final kept = StringBuffer();
  for (final unit in raw.runes) {
    if (bidiControls.contains(unit)) continue;
    // C0, DEL and C1. A name is text; a control character in it is either a
    // mistake or an attempt.
    if (unit < 0x20 || (unit >= 0x7F && unit <= 0x9F)) continue;
    kept.writeCharCode(unit);
  }
  final trimmed = kept.toString().trim();
  // Never empty: an empty row is a row a person cannot point at, and the name
  // is how they decide whether to accept.
  return trimmed.isEmpty ? '—' : trimmed;
}
