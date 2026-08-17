package com.owner.qyro

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.net.InetAddress

/**
 * Discovery on Android, through `NsdManager` and nothing else.
 *
 * Specification: `docs/adr/ADR-0035-discovery-and-pairing.md` §7 and amendment 1.
 *
 * # Why not a socket from Rust
 *
 * Local Network Protections put the gate **below** the socket API: "these
 * restrictions are implemented deep in the networking stack, and thus they apply
 * to all networking APIs". A `UdpSocket` opened in Rust does not escape it, and
 * from Android 17 the permission is `ACCESS_LOCAL_NETWORK`, dangerous and
 * denied by default.
 *
 * `NsdManager` with `FLAG_SHOW_PICKER` is the way through, and it is better than
 * a permission: "Once the user selects a service, the app is granted permission
 * to communicate with that specific device… This grant persists across reboots.
 * … Connections to IP addresses obtained this way don't require the
 * ACCESS_LOCAL_NETWORK permission." Discovery **and** an authorised TCP
 * connection, with zero runtime permissions.
 *
 * # And the failure that gives no error
 *
 * Anything that is not `NsdManager` needs `WifiManager.MulticastLock`: the Wi-Fi
 * stack filters multicast packets **beneath** the socket, so a raw
 * `joinGroup`/`join_multicast_v4` succeeds and then receives nothing, silently.
 * That is the whole reason this file exists instead of a UDP socket in Rust.
 *
 * The lock is acquired here anyway, for the API levels whose `NsdManager`
 * implementation still resolves over the app's own multicast socket. It costs a
 * wake-lock's worth of battery while a browse is open and it is released the
 * moment the browse stops.
 */
class DiscoveryChannel(private val context: Context) : MethodChannel.MethodCallHandler {

    companion object {
        const val CHANNEL = "dev.qyro/discovery"

        /** The service type. Must agree with `qyro_net::SERVICE_TYPE`. */
        const val SERVICE_TYPE = "_qyro._tcp."

        /** The TXT key. Must agree with `qyro_net::TXT_FINGERPRINT_KEY`. */
        const val TXT_FINGERPRINT_KEY = "fp"

        /** Thirty-two lowercase hex characters, like everywhere else. */
        private val FINGERPRINT = Regex("^[0-9a-f]{32}$")
    }

    private val nsd: NsdManager? =
        context.getSystemService(Context.NSD_SERVICE) as? NsdManager

    private var multicastLock: android.net.wifi.WifiManager.MulticastLock? = null
    private var registration: NsdManager.RegistrationListener? = null
    private var browse: NsdManager.DiscoveryListener? = null
    private val found = linkedMapOf<String, Map<String, Any>>()

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        val manager = nsd
        if (manager == null) {
            // No NSD service is a device answer, not a crash. The manual pairing
            // string still works, which is why it was built first.
            result.error("unavailable", "this device has no NsdManager", null)
            return
        }
        when (call.method) {
            "advertise" -> advertise(manager, call, result)
            "browse" -> browse(manager, result)
            "stop" -> {
                stop(manager)
                result.success(null)
            }
            else -> result.notImplemented()
        }
    }

    private fun advertise(manager: NsdManager, call: MethodCall, result: MethodChannel.Result) {
        val port = call.argument<Int>("port")
        val fingerprint = call.argument<String>("fingerprint")
        if (port == null || port <= 0 || fingerprint == null ||
            !FINGERPRINT.matches(fingerprint)
        ) {
            // Refused here rather than announced malformed: what is advertised is
            // read by the whole network, and a record nobody can verify is worse
            // than no record.
            result.error("bad_argument", "a port and 32 lowercase hex characters", null)
            return
        }

        val info = NsdServiceInfo().apply {
            // The instance name is derived from the fingerprint. A device name
            // would leak into every café this is ever switched on in
            // (ADR-0035 §6).
            serviceName = "qyro-" + fingerprint.substring(0, 12)
            serviceType = SERVICE_TYPE
            this.port = port
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
                setAttribute(TXT_FINGERPRINT_KEY, fingerprint)
            }
        }

        registration?.let { runCatching { manager.unregisterService(it) } }
        val listener = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) = Unit
            override fun onRegistrationFailed(info: NsdServiceInfo, code: Int) = Unit
            override fun onServiceUnregistered(info: NsdServiceInfo) = Unit
            override fun onUnregistrationFailed(info: NsdServiceInfo, code: Int) = Unit
        }
        registration = listener
        manager.registerService(info, NsdManager.PROTOCOL_DNS_SD, listener)
        result.success(null)
    }

    private fun browse(manager: NsdManager, result: MethodChannel.Result) {
        found.clear()
        acquireMulticastLock()

        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(type: String) = Unit
            override fun onStartDiscoveryFailed(type: String, code: Int) = releaseMulticastLock()
            override fun onStopDiscoveryFailed(type: String, code: Int) = releaseMulticastLock()
            override fun onDiscoveryStopped(type: String) = releaseMulticastLock()
            override fun onServiceLost(info: NsdServiceInfo) {
                found.remove(info.serviceName)
            }

            override fun onServiceFound(info: NsdServiceInfo) {
                manager.resolveService(info, object : NsdManager.ResolveListener {
                    override fun onResolveFailed(info: NsdServiceInfo, code: Int) = Unit
                    override fun onServiceResolved(resolved: NsdServiceInfo) {
                        val entry = describe(resolved) ?: return
                        found[resolved.serviceName] = entry
                    }
                })
            }
        }
        browse = listener
        manager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
        // The current snapshot. Dart polls; a callback into Dart from an
        // arbitrary NSD thread would need its own lifecycle and this does not.
        result.success(found.values.toList())
    }

    private fun describe(info: NsdServiceInfo): Map<String, Any>? {
        val host: InetAddress = info.host ?: return null
        val raw = info.attributes?.get(TXT_FINGERPRINT_KEY) ?: return null
        val fingerprint = String(raw, Charsets.US_ASCII)
        // A service with no verifiable fingerprint is not a Qyro peer, whatever
        // it calls itself. Dropped rather than shown with a blank identity: an
        // entry a person cannot check is one they should not be offered.
        if (!FINGERPRINT.matches(fingerprint)) return null
        return mapOf(
            "address" to "${host.hostAddress}:${info.port}",
            "fingerprint" to fingerprint,
        )
    }

    private fun stop(manager: NsdManager) {
        browse?.let { runCatching { manager.stopServiceDiscovery(it) } }
        browse = null
        registration?.let { runCatching { manager.unregisterService(it) } }
        registration = null
        releaseMulticastLock()
    }

    private fun acquireMulticastLock() {
        if (multicastLock != null) return
        val wifi = context.applicationContext
            .getSystemService(Context.WIFI_SERVICE) as? android.net.wifi.WifiManager
            ?: return
        multicastLock = wifi.createMulticastLock("qyro-discovery").apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    private fun releaseMulticastLock() {
        multicastLock?.let { if (it.isHeld) it.release() }
        multicastLock = null
    }
}
