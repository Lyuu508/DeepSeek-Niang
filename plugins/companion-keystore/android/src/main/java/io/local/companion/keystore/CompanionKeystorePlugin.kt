package io.local.companion.keystore

import android.app.Activity
import android.content.Context
import android.content.pm.PackageManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import org.json.JSONArray
import java.util.concurrent.ConcurrentHashMap
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import app.tauri.plugin.JSObject
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import java.io.File
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.Executors
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class SecretArgs {
    lateinit var profileId: String
    var key: String? = null
}

@InvokeArg
class LanArgs { var port: Int = 0; var instance: String = "" }

/** Private native bridge. No frontend permission or JS invoke handler grants readSecret. */
@TauriPlugin
class CompanionKeystorePlugin(private val activity: Activity) : Plugin(activity) {
    private val executor = Executors.newSingleThreadExecutor()
    private val nsd by lazy { activity.getSystemService(Context.NSD_SERVICE) as NsdManager }
    private val candidates = ConcurrentHashMap<String, Pair<String, Long>>()
    private var discovery: NsdManager.DiscoveryListener? = null
    private var registration: NsdManager.RegistrationListener? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private var lanArgs: LanArgs? = null
    @Volatile private var lanError: String? = null

    private fun stopNsd() {
        discovery?.let { try { nsd.stopServiceDiscovery(it) } catch (_: Exception) {} }
        registration?.let { try { nsd.unregisterService(it) } catch (_: Exception) {} }
        discovery = null; registration = null; candidates.clear()
    }
    private fun startNsd(args: LanArgs) {
        stopNsd()
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(type: String) { lanError = null }
            override fun onDiscoveryStopped(type: String) {}
            override fun onStartDiscoveryFailed(type: String, error: Int) { candidates.clear(); lanError = "NSD_DISCOVERY_FAILED" }
            override fun onStopDiscoveryFailed(type: String, error: Int) {}
            override fun onServiceLost(info: NsdServiceInfo) { candidates.remove(info.serviceName) }
            override fun onServiceFound(info: NsdServiceInfo) {
                if (info.serviceName == args.instance) return
                if (candidates.size >= 64) return
                try {
                @Suppress("DEPRECATION")
                nsd.resolveService(info, object : NsdManager.ResolveListener {
                    override fun onResolveFailed(service: NsdServiceInfo, code: Int) {}
                    override fun onServiceResolved(service: NsdServiceInfo) {
                        @Suppress("DEPRECATION")
                        val host = service.host?.hostAddress ?: return
                        candidates[service.serviceName] = Pair("$host:${service.port}", android.os.SystemClock.elapsedRealtime())
                    }
                })
                } catch (_: Exception) { lanError = "NSD_RESOLVE_FAILED" }
            }
        }
        discovery = listener
        nsd.discoverServices("_dscompanion._tcp.", NsdManager.PROTOCOL_DNS_SD, listener)
        val registered = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) {}
            override fun onRegistrationFailed(info: NsdServiceInfo, code: Int) { lanError = "NSD_REGISTRATION_FAILED" }
            override fun onServiceUnregistered(info: NsdServiceInfo) {}
            override fun onUnregistrationFailed(info: NsdServiceInfo, code: Int) {}
        }
        registration = registered
        nsd.registerService(NsdServiceInfo().apply {
            serviceName = args.instance; serviceType = "_dscompanion._tcp."; port = args.port
            setAttribute("v", "2")
        }, NsdManager.PROTOCOL_DNS_SD, registered)
    }
    @Command
    fun startLan(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(LanArgs::class.java)
            require(args.port in 1..65535 && args.instance.length in 1..64)
            if (Build.VERSION.SDK_INT >= 37 && activity.applicationInfo.targetSdkVersion >= 37 &&
                activity.checkSelfPermission("android.permission.ACCESS_LOCAL_NETWORK") != PackageManager.PERMISSION_GRANTED) {
                activity.runOnUiThread { activity.requestPermissions(arrayOf("android.permission.ACCESS_LOCAL_NETWORK"), 2702) }
                invoke.reject("LOCAL_NETWORK_PERMISSION"); return
            }
            lanArgs = args
            startNsd(args)
            if (networkCallback == null) {
                val callback = object : ConnectivityManager.NetworkCallback() {
                    override fun onAvailable(network: Network) { activity.runOnUiThread { try { lanArgs?.let { startNsd(it) } } catch (_: Exception) { lanError = "LOCAL_NETWORK_PERMISSION_OR_DISCOVERY" } } }
                    override fun onLost(network: Network) { candidates.clear() }
                }
                (activity.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager).registerDefaultNetworkCallback(callback)
                networkCallback = callback
            }
            invoke.resolve(JSObject().apply { put("ok", true) })
        } catch (_: Exception) { invoke.reject("LOCAL_NETWORK_PERMISSION_OR_DISCOVERY") }
    }
    @Command
    fun pollLan(invoke: Invoke) {
        val list = JSONArray()
        val now = android.os.SystemClock.elapsedRealtime()
        candidates.forEach { (name, value) -> if (now - value.second < 120000) {
            list.put(JSObject().apply { put("instance", name); put("address", value.first) })
        } }
        invoke.resolve(JSObject().apply { put("candidates", list); put("error", lanError) })
    }
    @Command
    fun stopLan(invoke: Invoke) {
        lanArgs = null; stopNsd()
        networkCallback?.let { (activity.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager).unregisterNetworkCallback(it) }
        networkCallback = null
        invoke.resolve(JSObject().apply { put("ok", true) })
    }


    private fun profile(args: SecretArgs): String {
        require(UUID.fromString(args.profileId).toString() == args.profileId.lowercase())
        return args.profileId.lowercase()
    }
    private fun alias(profile: String) = "${activity.packageName}.deepseek.$profile"
    private fun file(profile: String): AtomicFile {
        val directory = File(activity.noBackupFilesDir, "companion-keystore")
        check(directory.isDirectory || directory.mkdirs())
        return AtomicFile(File(directory, "$profile.bin"))
    }
    private fun keyStore(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private fun key(profile: String, create: Boolean): SecretKey? {
        val store = keyStore()
        (store.getKey(alias(profile), null) as? SecretKey)?.let { return it }
        if (!create) return null
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder(alias(profile), KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .build())
        }.generateKey()
    }
    private fun aad(profile: String) = "companion-deepseek-v1/$profile".toByteArray(Charsets.UTF_8)
    private fun run(invoke: Invoke, action: (SecretArgs) -> JSObject) {
        executor.execute {
            try { invoke.resolve(action(invoke.parseArgs(SecretArgs::class.java))) }
            catch (_: Exception) { invoke.reject("SECURE_STORAGE_UNAVAILABLE") }
        }
    }
    @Command
    fun writeSecret(invoke: Invoke) = run(invoke) { args ->
        val profile = profile(args)
        val value = requireNotNull(args.key)
        require(value.length in 8..2048 && value.all { it.code in 33..126 })
        val plain = value.toByteArray(Charsets.UTF_8)
        try {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key(profile, true))
            cipher.updateAAD(aad(profile))
            val encrypted = cipher.doFinal(plain)
            check(cipher.iv.size == 12)
            val target = file(profile)
            val stream = target.startWrite()
            try {
                stream.write(byteArrayOf(1)); stream.write(cipher.iv); stream.write(encrypted)
                target.finishWrite(stream)
            } catch (error: Exception) { target.failWrite(stream); throw error }
        } finally { plain.fill(0) }
        JSObject().apply { put("ok", true) }
    }
    @Command
    fun readSecret(invoke: Invoke) = run(invoke) { args ->
        val profile = profile(args)
        val target = file(profile)
        if (!target.baseFile.exists()) JSObject().apply { put("key", null) }
        else {
            val blob = target.readFully()
            check(blob.size in 30..4096 && blob[0].toInt() == 1)
            val existingKey = checkNotNull(key(profile, false))
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, existingKey, GCMParameterSpec(128, blob.copyOfRange(1, 13)))
            cipher.updateAAD(aad(profile))
            val plain = cipher.doFinal(blob.copyOfRange(13, blob.size))
            try { JSObject().apply { put("key", String(plain, Charsets.UTF_8)) } }
            finally { plain.fill(0) }
        }
    }
    @Command
    fun deleteSecret(invoke: Invoke) = run(invoke) { args ->
        val profile = profile(args)
        file(profile).delete()
        keyStore().deleteEntry(alias(profile))
        check(!file(profile).baseFile.exists())
        JSObject().apply { put("ok", true) }
    }
}
