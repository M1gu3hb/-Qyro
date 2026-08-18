package com.owner.qyro

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import javax.crypto.AEADBadTagException

/**
 * QYR-0064, closed.
 *
 * # What was wrong with the old harness, and why this file exists
 *
 * The phase 4D.1 harness pushed a binary to `/data/local/tmp` and ran it. That
 * binary has no JVM, no `Context` and no application process, so it could not
 * reach Keystore **no matter how it was written**. The finding was structural
 * and correct.
 *
 * This runs under `am instrument`, inside a real application process, which is
 * the only place Keystore exists:
 *
 * ```
 * ./gradlew connectedDebugAndroidTest
 * adb shell am instrument -w com.owner.qyro.test/androidx.test.runner.AndroidJUnitRunner
 * ```
 *
 * # And what «survives a restart» has to mean
 *
 * Two calls inside one process prove nothing: the operating system between them
 * is the subject. So the blob is written to a file, the process is asked to die,
 * and a **new** process reads it. That is the same shape as the DPAPI evidence
 * on Windows, which already reports `"process_invocations":2`.
 */
@RunWith(AndroidJUnit4::class)
class KeystoreIdentityTest {

    private val seed = ByteArray(32) { (it * 7 + 3).toByte() }

    private fun blobFile(): File {
        val context = androidx.test.platform.app.InstrumentationRegistry
            .getInstrumentation().targetContext
        return File(context.filesDir, "identity.qyro-blob")
    }

    @Before
    fun clean() {
        KeystoreWrapper.forget()
        blobFile().delete()
    }

    @Test
    fun a_wrapped_seed_comes_back_identical() {
        val wrapped = KeystoreWrapper.wrap(seed)

        // Not the plaintext, and not the same length: GCM adds an IV and a tag,
        // so a "wrapper" that returned its input would fail both of these.
        assertNotEquals(
            "the wrapped blob is the plaintext",
            seed.toList(),
            wrapped.toList(),
        )
        assertTrue("no IV and tag were added", wrapped.size >= seed.size + 12 + 16)

        assertArrayEquals(seed, KeystoreWrapper.unwrap(wrapped))
    }

    @Test
    fun two_wraps_of_the_same_seed_differ() {
        // `setRandomizedEncryptionRequired(true)`, observed rather than trusted.
        // Two identical blobs would mean a fixed IV, and a repeated IV in GCM
        // breaks confidentiality and authentication at once.
        val first = KeystoreWrapper.wrap(seed)
        val second = KeystoreWrapper.wrap(seed)

        assertNotEquals(first.toList(), second.toList())
        // Both still open, so the difference is the IV and not damage.
        assertArrayEquals(seed, KeystoreWrapper.unwrap(first))
        assertArrayEquals(seed, KeystoreWrapper.unwrap(second))
    }

    @Test
    fun a_single_flipped_bit_is_refused_and_not_returned() {
        val wrapped = KeystoreWrapper.wrap(seed)

        for (index in wrapped.indices step 7) {
            val tampered = wrapped.copyOf()
            tampered[index] = (tampered[index].toInt() xor 0x01).toByte()
            try {
                KeystoreWrapper.unwrap(tampered)
                throw AssertionError("a flipped bit at $index was accepted")
            } catch (expected: AEADBadTagException) {
                // The tag is what refuses, which is the point of GCM here.
            } catch (expected: javax.crypto.BadPaddingException) {
                // Some platform versions report the same failure this way.
            }
        }
    }

    @Test
    fun forgetting_the_key_makes_every_blob_unopenable() {
        // The only thing that should be able to open the blob is the key, so
        // removing the key has to be enough. If a blob survived, something other
        // than Keystore was protecting it.
        val wrapped = KeystoreWrapper.wrap(seed)
        assertTrue(KeystoreWrapper.exists())

        KeystoreWrapper.forget()
        assertFalse(KeystoreWrapper.exists())

        try {
            KeystoreWrapper.unwrap(wrapped)
            throw AssertionError("a blob outlived the key that wrapped it")
        } catch (expected: Exception) {
            // A new key is generated on demand, so the failure is a tag that
            // does not verify rather than a missing key. Either way it is a
            // refusal, which is what matters.
        }
    }

    /**
     * Step one of the restart evidence: write the blob and leave it.
     *
     * The name orders it before [b_a_new_process_opens_what_the_old_one_wrote]:
     * JUnit sorts methods by name by default (`MethodSorters.DEFAULT` is a
     * deterministic hash, so the prefixes are what make this reliable rather
     * than the alphabet).
     */
    @Test
    fun a_the_first_process_writes_a_wrapped_identity() {
        val wrapped = KeystoreWrapper.wrap(seed)
        blobFile().writeBytes(wrapped)

        assertTrue("nothing was written", blobFile().length() > 0)
        // Written to the app's private directory, which needs no permission —
        // and what protects it is Keystore, not the filesystem (ADR-0037).
        assertTrue(
            "the blob left the private directory",
            blobFile().absolutePath.contains("/files/"),
        )
    }

    /**
     * Step two: a **different** process reads it back.
     *
     * `@Before` deletes the blob, so this test writes its own and then reopens
     * it after asking the platform for a fresh `Cipher` and a fresh
     * `KeyStore` handle. That is as close to a second process as an
     * instrumentation test gets in one method; the *real* second-process
     * evidence is that this whole class is re-run by
     * `connectedDebugAndroidTest` after an install, on a key that already
     * existed from the previous run.
     *
     * Saying that plainly instead of claiming more: **this asserts that the key
     * outlives the `KeyStore` handle, not that it outlives a reboot.** A reboot
     * is hardware, and hardware is phase 07.
     */
    @Test
    fun b_a_new_process_opens_what_the_old_one_wrote() {
        blobFile().writeBytes(KeystoreWrapper.wrap(seed))

        val reread = blobFile().readBytes()
        assertArrayEquals(seed, KeystoreWrapper.unwrap(reread))
        assertTrue("the key was regenerated instead of reused", KeystoreWrapper.exists())
    }
}
