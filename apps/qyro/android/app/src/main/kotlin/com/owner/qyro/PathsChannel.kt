package com.owner.qyro

import android.content.Context
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.io.File

/**
 * Dónde escribe Qyro lo que recibe, en Android.
 *
 * **QYR-0373, y era un P0.** `defaultDestination()` en Dart devolvía
 * `Directory.current.path + "/Qyro"`, con un comentario que decía «el lado
 * Kotlin la pasa; hasta que lo haga, el directorio de trabajo del proceso es la
 * respuesta honesta». **En Android el directorio de trabajo del proceso es
 * `/`**, así que la respuesta era `/Qyro` — la raíz del sistema, que ninguna
 * aplicación puede escribir. `Directory("/Qyro").createSync()` lanza, y lanza
 * **antes** de que la pantalla de recibir emita un solo estado: pulsar Recibir
 * en el teléfono no hacía nada. El lado Kotlin que iba a pasarla nunca se
 * escribió.
 *
 * # Por qué `getExternalFilesDir` y no otra cosa
 *
 * Devuelve `/sdcard/Android/data/dev.qyro.app/files`, y esa ruta tiene las tres
 * propiedades que hacen falta a la vez:
 *
 * 1. **No necesita ningún permiso.** Es el directorio propio de la aplicación en
 *    el almacenamiento externo, y desde Android 4.4 escribir ahí no pide nada.
 *    Un permiso de almacenamiento aquí sería el defecto que ADR-0034 §4 existe
 *    para impedir.
 * 2. **La persona puede llegar a lo que recibió**, por USB o desde un explorador
 *    de archivos. `getFilesDir()` —el almacenamiento interno— también es
 *    escribible y **no se ve desde fuera**: un archivo que llega y que su dueño
 *    no puede abrir no ha llegado.
 * 3. **Se borra al desinstalar**, que es la promesa de una aplicación que no
 *    deja rastro.
 *
 * `getExternalFilesDir` puede devolver `null` si el almacenamiento externo no
 * está montado en ese instante. Entonces se contesta `null` y Dart se queda con
 * lo que tenía, en vez de inventar una ruta que fallaría al escribir.
 */
class PathsChannel(private val context: Context) : MethodChannel.MethodCallHandler {

    companion object {
        const val CHANNEL = "dev.qyro/paths"

        /**
         * La carpeta, dentro del directorio de la aplicación.
         *
         * Una subcarpeta con nombre y no el directorio a secas: ahí dentro
         * pueden acabar viviendo otras cosas, y lo que llega de otro aparato
         * merece estar separado de lo que la aplicación se guarda a sí misma.
         */
        const val FOLDER = "Qyro"
    }

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "destination" -> result.success(destination())
            else -> result.notImplemented()
        }
    }

    /**
     * La carpeta de destino, creada si no existía, o `null` si no se puede.
     *
     * Se crea aquí y no en Dart porque el fallo que importa es «no se pudo», y
     * responder una ruta que todavía no existe deja ese fallo para el momento de
     * escribir el primer byte — que es tres capas más tarde y con un archivo ya
     * a medias en el cable.
     */
    private fun destination(): String? {
        val base = context.getExternalFilesDir(null) ?: return null
        val folder = File(base, FOLDER)
        if (!folder.exists() && !folder.mkdirs()) return null
        if (!folder.isDirectory) return null
        return folder.absolutePath
    }
}
