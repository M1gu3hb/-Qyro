package com.owner.qyro

import android.content.Intent
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {

    private var picker: FilePickerChannel? = null
    private var discovery: DiscoveryChannel? = null
    private var scanner: ScannerChannel? = null
    private var paths: PathsChannel? = null

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

        val eye = ScannerChannel(this)
        scanner = eye
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            ScannerChannel.CHANNEL,
        ).setMethodCallHandler(eye)

        // QYR-0373. Sin esto, el destino en Android era `/Qyro` -- la raiz del
        // sistema-- y recibir fallaba antes de emitir un solo estado.
        val where = PathsChannel(this)
        paths = where
        MethodChannel(
            flutterEngine.dartExecutor.binaryMessenger,
            PathsChannel.CHANNEL,
        ).setMethodCallHandler(where)
    }

    override fun onDestroy() {
        // La cámara se suelta al morir la actividad. Sin esto, un aparato que
        // vuelve a la aplicación se encuentra la cámara ocupada por su propio
        // proceso anterior.
        scanner?.stop()
        super.onDestroy()
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        // The picker answers first. If it did not own this request code the
        // result still has to reach the embedding, or any other consumer of
        // startActivityForResult silently stops receiving anything.
        if (picker?.onActivityResult(requestCode, resultCode, data) == true) return
        super.onActivityResult(requestCode, resultCode, data)
    }
}
