//! Every number ADR-0028 froze, in one place.
//!
//! These are not tuning knobs. Each one is a decision with an argument written
//! next to it in `docs/adr/ADR-0028-network-transport.md`, and a test in this
//! crate that exercises it. A constant here whose value no test can observe is
//! prose, not a limit.

use core::time::Duration;

/// Bytes an established connection stages between `read` and the decoder.
///
/// ADR-0028 §2. It bounds syscalls per byte, **not** memory: the memory ceiling
/// belongs to `FrameDecoder` (`MAX_BUFFER_LEN`, 1 049 664) and stays there.
///
/// 65 536 because it is the order of the frame that dominates this wire — a
/// sealed `DataChunk` is 48 header + 8 body + 65 536 content + 16 tag = 65 608
/// bytes — so a chunk arrives in about two reads instead of the nine that 8 KiB
/// would cost. It is also already the number this codebase uses for
/// `CHUNK_SIZE` and `HASH_BUFFER_LEN`.
///
/// The value is argued, not measured. Nothing in sprint 6A measures throughput
/// against buffer size, and a loopback socket would be the wrong place to try.
pub const READ_BUFFER_LEN: usize = 65_536;

/// Bytes accepted from a peer that has not authenticated yet.
///
/// ADR-0028 §3.1. A legitimate handshake **receives** 295 bytes at either end
/// (two framed messages: 212 + 83 for the dialer, 148 + 147 for the listener),
/// so this is more than an order of magnitude of headroom.
///
/// What makes it a limit rather than a wish is where it is enforced: while the
/// connection is unauthenticated, no read is ever issued with a buffer larger
/// than the remaining allowance. Reading 64 KiB and *then* checking would mean
/// the bytes were already accepted by the time anyone looked.
pub const MAX_PREAUTH_BYTES: usize = 4096;

/// How long the whole handshake may take, from connection to established.
///
/// ADR-0028 §3.2. Two round trips plus two X25519, a signature, a verification,
/// an HKDF and an HMAC: single-digit milliseconds of work, and a round trip
/// under 5 ms on a LAN. Ten seconds is roughly twenty times a badly degraded
/// Wi-Fi.
///
/// **Total, not per message.** A per-message deadline is restarted for ever by
/// a peer that emits one byte before each expiry, which is the classic
/// slowloris; a total deadline is not.
pub const HANDSHAKE_DEADLINE: Duration = Duration::from_secs(10);

/// Connections accepted but not yet authenticated, at one time.
///
/// ADR-0028 §3.3. This is the number that bounds what a **stranger** can make
/// this process hold.
pub const MAX_PENDING_HANDSHAKES: usize = 8;

/// Authenticated sessions alive at one time.
///
/// ADR-0028 §3.3. Product policy rather than defence: Qyro is a one-to-one
/// application. Kept distinct from [`MAX_PENDING_HANDSHAKES`] because merging
/// the two would let unauthenticated peers consume the budget meant for real
/// transfers.
pub const MAX_ESTABLISHED_SESSIONS: usize = 4;

/// How long a dial waits for the far end to answer.
///
/// ADR-0028 §4. Without it, a black-holed address blocks for whatever the
/// operating system's default happens to be, which is around two minutes on
/// Linux and is not a decision anyone made.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How often a thread parked in `read` wakes up.
///
/// ADR-0028 §4.1. **This is not a deadline and its expiry is not an error.** It
/// is the heartbeat that lets a blocked reader notice a cancellation request,
/// and a connection that is merely waiting hits it constantly. Treating it as
/// an ending would kill every transfer with a pause longer than a quarter of a
/// second.
pub const READ_TIMEOUT: Duration = Duration::from_millis(250);

/// How long a connection may deliver nothing at all before it is declared dead.
///
/// ADR-0028 §4.2, and the one number in this file that decides "slow" against
/// "dead". The distinction is **progress, not rate**: there is no total
/// deadline on a transfer, so a 4 GiB file at 100 KiB/s may take eleven and a
/// half hours. What is not allowed is sixty seconds without a single byte.
///
/// Any byte resets it — not a whole frame. A peer delivering a frame slowly is
/// alive; a peer delivering nothing is not.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
