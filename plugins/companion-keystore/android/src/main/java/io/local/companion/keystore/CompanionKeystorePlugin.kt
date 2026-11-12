package io.local.companion.keystore

import android.app.Activity
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

/** Private native bridge. No frontend permission or JS invoke handler grants readSecret. */
@TauriPlugin
class CompanionKeystorePlugin(private val activity: Activity) : Plugin(activity) {
    private val executor = Executors.newSingleThreadExecutor()

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
