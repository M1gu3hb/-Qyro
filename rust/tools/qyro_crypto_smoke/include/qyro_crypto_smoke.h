/*
 * Qyro cryptographic smoke test — TEST HARNESS ONLY, NEVER SHIPPED.
 *
 * Declares the whole C surface of `qyro_crypto_smoke`. It is one function that
 * takes nothing and returns an integer, and that is deliberate: no pointer
 * crosses this boundary in either direction, so there is nothing to own, free
 * or leak, and no way for a key, a seed, a nonce, a traffic secret or plaintext
 * to reach the caller.
 *
 * Do not link this into the Qyro application. `qyro_ffi` — the library Dart
 * loads — cannot reach `qyro_crypto` at all, and a guard in CI searches the
 * APK, Runner.app and the Windows portable ZIP for `qyro_crypto_smoke_run` and
 * fails the build if it appears in any of them.
 *
 * See docs/adr/ADR-0023-crypto-platform-test-harness.md.
 */

#ifndef QYRO_CRYPTO_SMOKE_H
#define QYRO_CRYPTO_SMOKE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Runs one complete session — two fresh identities, the four-message
 * handshake, sealing, a wire round trip, opening, a replay attempt and a
 * tamper attempt — and reports the first step that failed.
 *
 * Returns 0 on success. Non-zero values are stable across releases because a
 * CI runner reads them out of a process exit status:
 *
 *    0  success
 *    1  device identity generation failed (system CSPRNG unavailable)
 *    2  the four-message handshake did not complete
 *    3  the two sides disagree on the session identifier
 *    4  frame key derivation failed
 *    5  sealing a frame failed
 *    6  a sealed frame did not survive encode and decode
 *    7  opening a sealed frame failed
 *    8  the plaintext or the authenticated metadata came back wrong
 *    9  a replayed frame was not rejected as a replay
 *   10  a frame with an altered tag authenticated
 *   11  the responder-to-initiator direction failed
 *
 * Entropy comes from the system CSPRNG, the same source production uses. There
 * is no deterministic mode: a harness with a fixed key would show that the
 * platform can reproduce a constant, which is not the question being asked.
 */
int32_t qyro_crypto_smoke_run(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* QYRO_CRYPTO_SMOKE_H */
