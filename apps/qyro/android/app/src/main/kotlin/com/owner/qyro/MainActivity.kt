package com.owner.qyro

import android.content.Intent
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {

    private var picker: FilePickerChannel? = null
    private var discovery: DiscoveryChannel? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        val handler = FilePickerChannel(this)
        picker = handler
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            FilePickerChannel.CHANNEL,
        ).setMethodCallHandler(handler)

        val finder = DiscoveryChannel(this)
        discovery = finder
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            DiscoveryChannel.CHANNEL,
        ).setMethodCallHandler(finder)
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        // The picker answers first. If it did not own this request code the
        // result still has to reach the embedding, or any other consumer of
        // startActivityForResult silently stops receiving anything.
        if (picker?.onActivityResult(requestCode, resultCode, data) == true) return
        super.onActivityResult(requestCode, resultCode, data)
    }
}
