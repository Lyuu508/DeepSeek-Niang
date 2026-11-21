package io.local.deepseekcompanion

import android.app.Activity
import android.content.ClipData
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.security.MessageDigest

class CompanionUpdateFileProvider : FileProvider()

@InvokeArg
class UpdateArgs {
    lateinit var path: String
    lateinit var version: String
    lateinit var sha256: String
}

@TauriPlugin
class CompanionUpdatePlugin(private val activity: Activity) : Plugin(activity) {
    private fun reply(invoke: Invoke, status: String) = invoke.resolve(JSObject().put("status", status))

    private fun digest(bytes: ByteArray): String = bytes.joinToString("") { "%02x".format(it.toInt() and 255) }

    private fun signers(info: PackageInfo): Set<String> =
        info.signingInfo?.apkContentsSigners?.map {
            digest(MessageDigest.getInstance("SHA-256").digest(it.toByteArray()))
        }?.toSet() ?: emptySet()

    @Suppress("DEPRECATION")
    private fun checked(args: UpdateArgs): File {
        val cache = File(activity.cacheDir, "companion-updates").canonicalFile
        val file = File(args.path).canonicalFile
        val folder = file.parentFile ?: throw IllegalArgumentException("UPDATE_ANDROID_IDENTITY")
        if (folder.parentFile != cache || !folder.name.startsWith("release-") ||
            !args.version.matches(Regex("[0-9]+\\.[0-9]+\\.[0-9]+")) ||
            file.name != "DeepSeek-Niang_${args.version}_android-arm64.apk" || !file.isFile ||
            file.length() !in 1L..(384L * 1024L * 1024L)) {
            throw IllegalArgumentException("UPDATE_ANDROID_IDENTITY")
        }
        val hash = MessageDigest.getInstance("SHA-256")
        file.inputStream().buffered().use { input ->
            val buffer = ByteArray(65536)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                hash.update(buffer, 0, count)
            }
        }
        if (digest(hash.digest()) != args.sha256) throw IllegalArgumentException("UPDATE_CHECKSUM_INVALID")
        val manager = activity.packageManager
        val flags = PackageManager.GET_SIGNING_CERTIFICATES
        val installed = manager.getPackageInfo(activity.packageName, flags)
        val target = manager.getPackageArchiveInfo(file.path, flags)
            ?: throw IllegalArgumentException("UPDATE_ANDROID_IDENTITY")
        val expected = signers(installed)
        if (target.packageName != installed.packageName || target.versionName != args.version ||
            target.longVersionCode <= installed.longVersionCode || expected.isEmpty() || signers(target) != expected) {
            throw IllegalArgumentException("UPDATE_ANDROID_IDENTITY")
        }
        return file
    }

    private fun handle(invoke: Invoke, open: Boolean) {
        Thread {
            try {
                val file = checked(invoke.parseArgs(UpdateArgs::class.java))
                if (!open) {
                    reply(invoke, "verified")
                    return@Thread
                }
                activity.runOnUiThread {
                    try {
                        if (!activity.packageManager.canRequestPackageInstalls()) {
                            activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:${activity.packageName}")))
                            reply(invoke, "permission_required")
                        } else {
                            val apk = FileProvider.getUriForFile(activity, "${activity.packageName}.companion.update", file)
                            val intent = Intent(Intent.ACTION_VIEW).apply {
                                setDataAndType(apk, "application/vnd.android.package-archive")
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                                clipData = ClipData.newRawUri("DeepSeek娘", apk)
                            }
                            activity.startActivity(intent)
                            reply(invoke, "installer_open")
                        }
                    } catch (_: Exception) { reply(invoke, "UPDATE_INSTALLER_FAILED") }
                }
            } catch (error: IllegalArgumentException) {
                reply(invoke, if (error.message == "UPDATE_CHECKSUM_INVALID") "UPDATE_CHECKSUM_INVALID" else "UPDATE_ANDROID_IDENTITY")
            } catch (_: Exception) { reply(invoke, "UPDATE_ANDROID_IDENTITY") }
        }.start()
    }

    @Command
    fun verifyUpdate(invoke: Invoke) = handle(invoke, false)

    @Command
    fun installUpdate(invoke: Invoke) = handle(invoke, true)
}
