package com.owner.qyro

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The Android half of the secret wrapper.
 *
 * Specification: `docs/adr/ADR-0037-android-keystore-bridge.md`.
 *
 * # Why this is here and not in Rust
 *
 * **There is no Keystore API in the NDK.** That is why QYR-0064 stayed open: a
 * binary pushed to `/data/local/tmp` has no JVM, no `Context` and no application
 * process, so it could not reach Keystore no matter how it was written. Saying
 * so was correct; this is the other half of the answer.
 *
 * Rust never calls Keystore. Rust hands over bytes and gets bytes back.
 *
 * # The blob
 *
 * `IV ‖ ciphertext ‖ tag`, AES-256-GCM, twelve-byte IV chosen by the platform.
 * The same class writes it and reads it, so there is no format to negotiate.
 *
 * `setRandomizedEncryptionRequired(true)` is the default and is written anyway:
 * an IV a caller could choose is an IV a caller can repeat, and a repeated IV in
 * GCM breaks confidentiality and authentication at once.
 */
object KeystoreWrapper {

    /** Where the key lives. */
    private const val PROVIDER = "AndroidKeyStore"

    /** The alias. Versioned, so a future format is a different key. */
    const val ALIAS = "qyro.identity.v1"

    /** GCM's IV, in bytes. */
    private const val IV_LENGTH = 12

    /** GCM's tag, in bits. */
    private const val TAG_BITS = 128

    /**
     * Wraps [plain]. The result is `IV ‖ ciphertext ‖ tag`.
     *
     * @throws java.security.GeneralSecurityException if the platform refuses.
     */
    fun wrap(plain: ByteArray): ByteArray {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val iv = cipher.iv
        require(iv.size == IV_LENGTH) { "the platform chose a ${iv.size}-byte IV" }
        val sealed = cipher.doFinal(plain)
        return iv + sealed
    }

    /**
     * Reverses [wrap]. Any tampering fails here, because GCM authenticates.
     *
     * @throws java.security.GeneralSecurityException if the tag does not verify.
     */
    fun unwrap(wrapped: ByteArray): ByteArray {
        require(wrapped.size > IV_LENGTH) { "too short to carry an IV and a tag" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(
            Cipher.DECRYPT_MODE,
            key(),
            GCMParameterSpec(TAG_BITS, wrapped, 0, IV_LENGTH),
        )
        return cipher.doFinal(wrapped, IV_LENGTH, wrapped.size - IV_LENGTH)
    }

    /** Forgets the key. Every blob wrapped under it becomes unopenable. */
    fun forget() {
        val store = KeyStore.getInstance(PROVIDER).apply { load(null) }
        if (store.containsAlias(ALIAS)) store.deleteEntry(ALIAS)
    }

    /** Whether a key exists, without creating one. */
    fun exists(): Boolean {
        val store = KeyStore.getInstance(PROVIDER).apply { load(null) }
        return store.containsAlias(ALIAS)
    }

    private fun key(): SecretKey {
        val store = KeyStore.getInstance(PROVIDER).apply { load(null) }
        (store.getEntry(ALIAS, null) as? KeyStore.SecretKeyEntry)?.let { return it.secretKey }

        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        generator.init(
            KeyGenParameterSpec.Builder(
                ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                // The identity has to exist when the application starts, and
                // demanding a fingerprint for it to *exist* would turn "this
                // device has an identity" into "the user is looking at it".
                // The trust decision is still the person's; the device's own
                // identity is not a secret they custody.
                .setUserAuthenticationRequired(false)
                // The default, written out: an IV the caller picks is an IV the
                // caller can repeat.
                .setRandomizedEncryptionRequired(true)
                .build(),
        )
        return generator.generateKey()
    }
}
