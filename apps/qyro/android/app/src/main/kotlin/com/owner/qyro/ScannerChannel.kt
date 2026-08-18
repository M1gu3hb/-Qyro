package com.owner.qyro

import android.content.Context
import android.util.Size
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.lifecycle.LifecycleOwner
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executors

/**
 * La cámara mira, y sólo el plano Y cruza.
 *
 * Esta clase **no decodifica nada**. Extrae la luma de cada frame a un array
 * compacto y lo pasa a Dart; quien lee QR es `qyro_eye`, en Rust, que ya existe
 * y ya está probado sin cámara. Aquí no hay ni un `if` sobre el contenido.
 *
 * # Por qué sólo el plano Y
 *
 * Un `ImageProxy` en `YUV_420_888` trae tres planos y `qyro_eye` pide uno: un
 * plano de luma de 8 bits. Copiar los tres sería copiar el 50 % de más para
 * tirarlo en la siguiente línea.
 *
 * # El de-padding, que es lo que evita un crash
 *
 * `plane.rowStride` **no** tiene por qué ser `width`: la cámara alinea las filas
 * y deja relleno al final de cada una. Peor: el buffer puede medir exactamente
 * `rowStride * (height - 1) + width`, así que **leer `rowStride * height` bytes
 * se sale por el final del último renglón**. Por eso se copia fila a fila con
 * `width` bytes cada una, y la última no se trata distinto porque nunca se pide
 * de más.
 *
 * Un plano con `rowStride == width` sale igual de correcto por el mismo camino:
 * no hay dos rutas que puedan discrepar.
 */
class ScannerChannel(private val context: Context) : MethodChannel.MethodCallHandler {

    companion object {
        const val CHANNEL = "dev.qyro/scanner"

        /**
         * La resolución que se pide, y no es negociable.
         *
         * `R10` §8 T1: si no se pide nada, CameraX elige **640×480**, y ahí un QR
         * versión 27 da **3,07 píxeles por módulo** — el suelo exacto de `rqrr`,
         * sin margen para desenfoque ni brillo. A 1280×720 da 4,60, dentro de la
         * banda fiable.
         *
         * `FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER`: si el aparato no tiene
         * exactamente 720p, se prefiere **subir** antes que bajar. Bajar es
         * volver al precipicio.
         */
        val TARGET = Size(1280, 720)
    }

    private var provider: ProcessCameraProvider? = null
    private val executor = Executors.newSingleThreadExecutor()

    /** El último frame de luma extraído, esperando a que Dart lo recoja. */
    @Volatile
    private var latest: ByteArray? = null

    @Volatile
    private var latestWidth = 0

    @Volatile
    private var latestHeight = 0

    @Volatile
    private var framesSeen = 0L

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "start" -> start(result)
            "latest" -> result.success(latestFrame())
            "stop" -> {
                stop()
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    private fun start(result: MethodChannel.Result) {
        val owner = context as? LifecycleOwner
        if (owner == null) {
            // Un contexto sin ciclo de vida no puede sostener una cámara. Se
            // dice con un código en vez de reventar: es una respuesta del
            // aparato, y el canal óptico no es el único que existe.
            result.error("unavailable", "this context has no lifecycle", null)
            return
        }

        val future = ProcessCameraProvider.getInstance(context)
        future.addListener({
            try {
                val cameraProvider = future.get()
                provider = cameraProvider

                val selector = ResolutionSelector.Builder()
                    .setResolutionStrategy(
                        ResolutionStrategy(
                            TARGET,
                            ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER,
                        ),
                    )
                    .build()

                val analysis = ImageAnalysis.Builder()
                    .setResolutionSelector(selector)
                    // `STRATEGY_KEEP_ONLY_LATEST` es el default y es el que se
                    // quiere (`R10` §8 T5): con `BLOCK_PRODUCER`, no cerrar una
                    // imagen a tiempo puede **parar también el preview**.
                    .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                    .build()

                analysis.setAnalyzer(executor) { image ->
                    try {
                        capture(image)
                    } finally {
                        // `R10` §8 T5: sin `close()` la cámara deja de producir
                        // imágenes. En un `finally` para que ninguna excepción
                        // del camino de arriba se lleve el preview por delante.
                        image.close()
                    }
                }

                cameraProvider.unbindAll()
                cameraProvider.bindToLifecycle(owner, CameraSelector.DEFAULT_BACK_CAMERA, analysis)
                result.success(null)
            } catch (error: Exception) {
                result.error("unavailable", error.message ?: "camera unavailable", null)
            }
        }, androidx.core.content.ContextCompat.getMainExecutor(context))
    }

    /**
     * Copia el plano de luma a un array compacto, fila a fila.
     *
     * **No se rota nada** (`R10` §8 T2): los tres patrones de posicionamiento son
     * el mecanismo de orientación del propio formato QR, y `rqrr` resuelve la
     * perspectiva desde ellos. Un código a 90° decodifica igual, y
     * `setOutputImageRotationEnabled(true)` cuesta 10–15 ms por frame para nada.
     */
    private fun capture(image: ImageProxy) {
        val plane = image.planes.getOrNull(0) ?: return
        val width = image.width
        val height = image.height
        if (width <= 0 || height <= 0) return

        val buffer = plane.buffer
        val rowStride = plane.rowStride
        val out = ByteArray(width * height)

        if (rowStride == width) {
            // Sin relleno: una copia y ya. Se comprueba y no se supone, porque
            // suponerlo es exactamente el crash que el camino de abajo evita.
            if (buffer.remaining() < out.size) return
            buffer.get(out, 0, out.size)
        } else {
            for (row in 0 until height) {
                val start = row * rowStride
                // **La comprobación que evita el crash.** El buffer puede medir
                // `rowStride * (height - 1) + width`, así que la última fila no
                // tiene relleno detrás y pedir `rowStride` bytes se sale.
                if (start + width > buffer.limit()) return
                buffer.position(start)
                buffer.get(out, row * width, width)
            }
        }

        latest = out
        latestWidth = width
        latestHeight = height
        framesSeen += 1
    }

    /**
     * El último frame, o `null` si todavía no hay ninguno.
     *
     * Se entrega **una vez**: tras recogerlo se borra, para que Dart pueda
     * distinguir «un frame nuevo» de «el mismo de antes» sin comparar
     * novecientos mil bytes. `framesSeen` sigue subiendo aunque nadie recoja,
     * que es lo que dice si la cámara está entregando y quien mira va lento.
     */
    private fun latestFrame(): Map<String, Any>? {
        val frame = latest ?: return null
        latest = null
        return mapOf(
            "luma" to frame,
            "width" to latestWidth,
            "height" to latestHeight,
            "seen" to framesSeen,
        )
    }

    fun stop() {
        provider?.unbindAll()
        provider = null
        latest = null
    }
}
