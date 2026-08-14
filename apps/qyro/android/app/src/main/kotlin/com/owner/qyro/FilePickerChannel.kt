package com.owner.qyro

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.OpenableColumns
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

/**
 * The file picker, written here rather than taken from a package.
 *
 * Specification: `docs/adr/ADR-0034-file-selection.md`.
 *
 * `file_selector_android` was the obvious choice and it copies: its main path
 * runs `getPathFromCopyOfFileFromUri`, which streams the whole content URI into
 * `{cacheDir}/{uuid}/{fileName}` before Dart ever sees it. A four-gigabyte file
 * is duplicated on disk before a byte moves, and on a phone with tight storage
 * that is not slow, it is impossible (QYR-0323).
 *
 * So this hands Dart the descriptor itself. Sixty lines, no package, and the
 * bytes are never copied.
 */
class FilePickerChannel(private val activity: Activity) : MethodChannel.MethodCallHandler {

    companion object {
        const val CHANNEL = "dev.qyro/file_picker"
        private const val REQUEST_OPEN = 0x51_59_01
    }

    private var pending: MethodChannel.Result? = null

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "pickFiles" -> pick(result)
            else -> result.notImplemented()
        }
    }

    private fun pick(result: MethodChannel.Result) {
        if (pending != null) {
            // A second picker while one is open would leave the first result
            // never completed, which in Dart is a Future that never resolves --
            // the worst kind of failure to debug because nothing reports it.
            result.error("busy", "a picker is already open", null)
            return
        }
        pending = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true)
            // Read is all a sender needs from the user's file. The "rw" below is
            // about the descriptor's seekability, not about writing to it.
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        activity.startActivityForResult(intent, REQUEST_OPEN)
    }

    /** Returns true when this handled the result. */
    fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?): Boolean {
        if (requestCode != REQUEST_OPEN) return false
        val result = pending ?: return true
        pending = null

        if (resultCode != Activity.RESULT_OK || data == null) {
            // A cancelled picker is not an error. It is a person changing their
            // mind, and Dart gets an empty list rather than an exception.
            result.success(emptyList<Map<String, Any>>())
            return true
        }

        val uris = mutableListOf<Uri>()
        data.clipData?.let { clip ->
            for (index in 0 until clip.itemCount) uris.add(clip.getItemAt(index).uri)
        }
        if (uris.isEmpty()) data.data?.let { uris.add(it) }

        val picked = mutableListOf<Map<String, Any>>()
        for (uri in uris) {
            val entry = describe(uri) ?: continue
            picked.add(entry)
        }
        result.success(picked)
        return true
    }

    private fun describe(uri: Uri): Map<String, Any>? {
        // "rw", never "r". The AOSP javadoc says the exclusive modes may return a
        // pipe or a socket pair; "rw" implies a file on disk that supports
        // seeking. With "r", seek() fails with ESPIPE and breaks the resume that
        // already exists in the engine.
        val descriptor = try {
            activity.contentResolver.openFileDescriptor(uri, "rw")
        } catch (_: Exception) {
            // A provider that refuses "rw" -- a read-only document tree, say --
            // is skipped rather than crashing the pick. The person keeps the
            // files that did open.
            null
        } ?: return null

        val length = descriptor.statSize
        // detachFd, never getFd. getFd leaves ownership with the
        // ParcelFileDescriptor, which also closes when it is collected: a double
        // close, and in a threaded process the second one can close a descriptor
        // that was reassigned in between -- the transfer socket, for instance.
        // Silent corruption rather than a loud failure. detachFd gives the
        // integer away, and Rust's File::from_raw_fd becomes its only owner.
        val fd = descriptor.detachFd()

        return mapOf(
            "fd" to fd,
            "name" to displayName(uri),
            "size" to length,
        )
    }

    private fun displayName(uri: Uri): String {
        val cursor = activity.contentResolver.query(uri, null, null, null, null)
        cursor?.use {
            val column = it.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (column >= 0 && it.moveToFirst()) {
                val name = it.getString(column)
                if (!name.isNullOrEmpty()) return sanitise(name)
            }
        }
        return "file"
    }

    /**
     * Keeps a provider-supplied name from becoming a path.
     *
     * A content provider chooses this string and nothing stops it containing
     * separators or `..`. The manifest layer refuses those too, but a name that
     * only fails three layers down is a name that reached three layers.
     */
    private fun sanitise(raw: String): String {
        val leaf = raw.substringAfterLast('/').substringAfterLast('\\')
        val cleaned = leaf.replace("..", "_")
        return cleaned.ifEmpty { "file" }
    }
}
