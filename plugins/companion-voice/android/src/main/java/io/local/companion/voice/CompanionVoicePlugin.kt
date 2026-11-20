package io.local.companion.voice

import android.Manifest
import android.app.Activity
import android.app.Application
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.speech.tts.Voice
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.util.Locale
import java.util.UUID

@TauriPlugin(permissions = [Permission(strings = [Manifest.permission.RECORD_AUDIO], alias = "microphone")])
class CompanionVoicePlugin(private val host: Activity) : Plugin(host) {
    private val handler = Handler(Looper.getMainLooper())
    private val manager = host.getSystemService(Context.AUDIO_SERVICE) as AudioManager
    private val attributes = AudioAttributes.Builder()
        .setUsage(AudioAttributes.USAGE_MEDIA)
        .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
        .build()
    private val scratch = File(host.cacheDir, "companion-speech")

    private data class Focus(val session: String, val request: AudioFocusRequest)
    private data class Synthesis(val session: String, val id: String, val file: File, val invoke: Invoke)
    private data class Recording(
        val session: String,
        val recognizer: SpeechRecognizer,
        val invoke: Invoke,
        var status: String? = null,
    )

    private var focus: Focus? = null
    private var engine: TextToSpeech? = null
    private var engineReady = false
    private var initializing: ((TextToSpeech?) -> Unit)? = null
    private var synthesis: Synthesis? = null
    private var recording: Recording? = null
    private var permission: Invoke? = null

    private val lifecycle = object : Application.ActivityLifecycleCallbacks {
        override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) {}
        override fun onActivityStarted(activity: Activity) {}
        override fun onActivityResumed(activity: Activity) {}
        override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) {}

        override fun onActivityPaused(activity: Activity) {
            if (activity !== host) return
            stopPlayback()
            recording?.let { finishRecording(it, null, "VOICE_CANCELLED") }
        }

        override fun onActivityStopped(activity: Activity) {
            if (activity !== host) return
            permission?.let { reject(it, "VOICE_CANCELLED") }
            permission = null
        }

        override fun onActivityDestroyed(activity: Activity) {
            if (activity !== host) return
            host.application.unregisterActivityLifecycleCallbacks(this)
            close()
        }
    }

    init {
        scratch.listFiles()?.filter { it.name.startsWith("utterance-") }?.forEach { it.delete() }
        host.application.registerActivityLifecycleCallbacks(lifecycle)
    }

    private fun reject(invoke: Invoke, code: String) = invoke.reject(code, code)
    private fun session(invoke: Invoke): String = invoke.getArgs().getString("session")

    private fun stopPlayback(session: String? = null, code: String = "VOICE_CANCELLED") {
        val active = focus
        if (active != null && (session == null || active.session == session)) {
            focus = null
            manager.abandonAudioFocusRequest(active.request)
        }
        val job = synthesis
        if (job != null && (session == null || job.session == session)) {
            synthesis = null
            engine?.stop()
            job.file.delete()
            reject(job.invoke, code)
        }
    }

    @Command
    fun acquireFocus(invoke: Invoke) = host.runOnUiThread {
        val id = session(invoke)
        if (!host.hasWindowFocus() || recording != null) {
            return@runOnUiThread invoke.resolveObject(false)
        }
        stopPlayback()
        val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT)
            .setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener({ state ->
                if (state < 0) stopPlayback(id, "VOICE_FOCUS_LOST")
            }, handler).build()
        val granted = manager.requestAudioFocus(request) == AudioManager.AUDIOFOCUS_REQUEST_GRANTED
        if (granted) focus = Focus(id, request)
        invoke.resolveObject(granted)
    }

    @Command
    fun hasFocus(invoke: Invoke) = host.runOnUiThread {
        invoke.resolveObject(focus?.session == session(invoke))
    }

    @Command
    fun releaseFocus(invoke: Invoke) = host.runOnUiThread {
        stopPlayback(session(invoke))
        invoke.resolveObject(true)
    }

    private fun withEngine(invoke: Invoke, use: (TextToSpeech) -> Unit) {
        val current = engine
        if (engineReady && current != null) {
            return use(current)
        }
        if (initializing != null) {
            return reject(invoke, "VOICE_BUSY")
        }
        val callback: (TextToSpeech?) -> Unit = { tts ->
            if (tts == null) reject(invoke, "VOICE_UNAVAILABLE") else use(tts)
        }
        initializing = callback
        engine = TextToSpeech(host.applicationContext) { status ->
            handler.post {
                if (initializing !== callback) return@post
                initializing = null
                engineReady = status == TextToSpeech.SUCCESS
                if (engineReady) {
                    callback(engine)
                } else {
                    engine?.shutdown()
                    engine = null
                    callback(null)
                }
            }
        }
        handler.postDelayed({
            if (initializing === callback) {
                initializing = null
                engine?.shutdown()
                engine = null
                callback(null)
            }
        }, 7000)
    }

    private fun offlineVoices(tts: TextToSpeech): List<Voice> = tts.voices.orEmpty()
        .filter { !it.isNetworkConnectionRequired && !it.features.orEmpty().contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED) }
        .sortedWith(compareBy<Voice>({ it.locale.language != "zh" }, { it.locale.toLanguageTag() }, { it.name }))

    @Command
    fun listVoices(invoke: Invoke) = host.runOnUiThread {
        withEngine(invoke) { tts ->
            invoke.resolveObject(offlineVoices(tts).map {
                mapOf("id" to it.name, "name" to it.name, "language" to it.locale.toLanguageTag())
            })
        }
    }

    private fun finishSynthesis(id: String, error: String? = null) {
        val job = synthesis?.takeIf { it.id == id } ?: return
        synthesis = null
        try {
            if (error != null) {
                engine?.stop()
                reject(job.invoke, error)
                return
            }
            val limit = 8 * 1024 * 1024
            if (job.file.length() !in 44L..limit.toLong()) {
                return reject(job.invoke, "VOICE_INVALID_AUDIO")
            }
            val bytes = job.file.readBytes()
            if (bytes.size !in 44..limit || String(bytes, 0, 4, Charsets.US_ASCII) != "RIFF"
                || String(bytes, 8, 4, Charsets.US_ASCII) != "WAVE") {
                return reject(job.invoke, "VOICE_INVALID_AUDIO")
            }
            job.invoke.resolve(JSObject().apply { put("wav", Base64.encodeToString(bytes, Base64.NO_WRAP)) })
        } catch (_: Exception) {
            reject(job.invoke, "VOICE_INVALID_AUDIO")
        } finally {
            job.file.delete()
        }
    }

    @Command
    fun synthesize(invoke: Invoke) = host.runOnUiThread {
        val id = session(invoke)
        val args = invoke.getArgs()
        val text = args.getString("text")
        if (text.isBlank() || text.codePointCount(0, text.length) > 280) {
            return@runOnUiThread reject(invoke, "VOICE_SYNTHESIS_FAILED")
        }
        withEngine(invoke) { tts ->
            if (focus?.session != id) return@withEngine reject(invoke, "VOICE_FOCUS_LOST")
            if (synthesis != null) return@withEngine reject(invoke, "VOICE_BUSY")
            val selected = args.getString("voiceId")
            val available = offlineVoices(tts)
            val voice = if (selected.isEmpty()) available.firstOrNull() else available.find { it.name == selected }
            if (voice == null) {
                val code = if (selected.isEmpty()) "VOICE_NO_OFFLINE_VOICE" else "VOICE_VOICE_REMOVED"
                return@withEngine reject(invoke, code)
            }
            if (tts.setVoice(voice) != TextToSpeech.SUCCESS ||
                tts.setSpeechRate(args.getDouble("speed").toFloat()) != TextToSpeech.SUCCESS ||
                tts.setPitch(args.getDouble("pitch").toFloat()) != TextToSpeech.SUCCESS) {
                return@withEngine reject(invoke, "VOICE_SYNTHESIS_FAILED")
            }
            val utterance = UUID.randomUUID().toString()
            val file = try {
                scratch.mkdirs()
                File.createTempFile("utterance-", ".wav", scratch)
            } catch (_: Exception) { return@withEngine reject(invoke, "VOICE_SYNTHESIS_FAILED") }
            synthesis = Synthesis(id, utterance, file, invoke)
            tts.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
                override fun onStart(utteranceId: String) {}
                override fun onDone(utteranceId: String) {
                    handler.post { finishSynthesis(utteranceId) }
                }
                @Deprecated("Android also provides the overload with an error code")
                override fun onError(utteranceId: String) {
                    handler.post { finishSynthesis(utteranceId, "VOICE_SYNTHESIS_FAILED") }
                }
                override fun onError(utteranceId: String, errorCode: Int) {
                    handler.post { finishSynthesis(utteranceId, "VOICE_SYNTHESIS_FAILED") }
                }
            })
            try {
                if (tts.synthesizeToFile(text, Bundle(), file, utterance) != TextToSpeech.SUCCESS) {
                    finishSynthesis(utterance, "VOICE_SYNTHESIS_FAILED")
                }
            } catch (_: Exception) {
                finishSynthesis(utterance, "VOICE_SYNTHESIS_FAILED")
            }
            handler.postDelayed({ finishSynthesis(utterance, "VOICE_TIMEOUT") }, 25_000)
        }
    }

    private fun recognitionAvailable(): Boolean = Build.VERSION.SDK_INT >= 31 &&
        SpeechRecognizer.isOnDeviceRecognitionAvailable(host)

    @Command
    fun checkRecognition(invoke: Invoke) = host.runOnUiThread {
        if (recognitionAvailable()) invoke.resolve()
        else reject(invoke, "VOICE_NO_ON_DEVICE_RECOGNIZER")
    }

    @Command
    fun beginRecognition(invoke: Invoke) = host.runOnUiThread {
        if (!recognitionAvailable()) return@runOnUiThread reject(invoke, "VOICE_NO_ON_DEVICE_RECOGNIZER")
        if (recording != null || permission != null) return@runOnUiThread reject(invoke, "VOICE_BUSY")
        stopPlayback()
        if (hasMicrophone()) {
            listen(invoke)
        } else {
            permission = invoke
            requestPermissionForAlias("microphone", invoke, "microphoneResult")
            handler.postDelayed({
                if (permission === invoke) {
                    permission = null
                    reject(invoke, "VOICE_TIMEOUT")
                }
            }, 18_000)
        }
    }

    private fun hasMicrophone() = host.checkSelfPermission(Manifest.permission.RECORD_AUDIO) ==
        PackageManager.PERMISSION_GRANTED

    @PermissionCallback
    fun microphoneResult(invoke: Invoke) = host.runOnUiThread {
        if (permission?.id != invoke.id) return@runOnUiThread
        permission = null
        if (hasMicrophone()) listen(invoke) else reject(invoke, "VOICE_PERMISSION_DENIED")
    }

    private fun listen(invoke: Invoke) {
        if (Build.VERSION.SDK_INT < 31 || host.isFinishing || host.isDestroyed) {
            return reject(invoke, "VOICE_NO_ON_DEVICE_RECOGNIZER")
        }
        val recognizer = try {
            SpeechRecognizer.createOnDeviceSpeechRecognizer(host)
        } catch (_: Exception) { return reject(invoke, "VOICE_NO_ON_DEVICE_RECOGNIZER") }
        val job = Recording(session(invoke), recognizer, invoke)
        recording = job
        recognizer.setRecognitionListener(object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                if (recording === job) job.status = "recording"
            }
            override fun onBeginningOfSpeech() {}
            override fun onRmsChanged(rmsdB: Float) {}
            override fun onBufferReceived(buffer: ByteArray?) {}
            override fun onEndOfSpeech() {
                if (recording === job) job.status = "transcribing"
            }
            override fun onPartialResults(partialResults: Bundle?) {}
            override fun onEvent(eventType: Int, params: Bundle?) {}
            override fun onResults(results: Bundle?) {
                val text = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    ?.firstOrNull()?.trim()?.take(4000)
                finishRecording(job, text, if (text.isNullOrEmpty()) "VOICE_NO_SPEECH" else null)
            }
            override fun onError(error: Int) {
                val code = when (error) {
                    SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "VOICE_PERMISSION_DENIED"
                    SpeechRecognizer.ERROR_NO_MATCH, SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "VOICE_NO_SPEECH"
                    SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED,
                    SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE -> "VOICE_RECOGNIZER_LANGUAGE"
                    else -> "VOICE_RECOGNITION_FAILED"
                }
                finishRecording(job, null, code)
            }
        })
        val input = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.SIMPLIFIED_CHINESE.toLanguageTag())
            .putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 1)
        try {
            recognizer.startListening(input)
        } catch (_: Exception) {
            finishRecording(job, null, "VOICE_RECOGNITION_FAILED")
        }
        handler.postDelayed({
            if (recording === job) job.recognizer.stopListening()
        }, 30_000)
        handler.postDelayed({ finishRecording(job, null, "VOICE_TIMEOUT") }, 35_000)
    }

    private fun finishRecording(job: Recording, text: String?, error: String?) {
        if (recording !== job) return
        recording = null
        job.recognizer.destroy()
        when {
            error != null -> reject(job.invoke, error)
            text != null -> job.invoke.resolveObject(text)
            else -> job.invoke.resolve()
        }
    }

    @Command
    fun recognitionStatus(invoke: Invoke) = host.runOnUiThread {
        val phase = recording?.takeIf { it.session == session(invoke) }?.status
        if (phase != null) invoke.resolveObject(phase) else invoke.resolve()
    }

    @Command
    fun finishRecognition(invoke: Invoke) = host.runOnUiThread {
        recording?.takeIf { it.session == session(invoke) }?.recognizer?.stopListening()
        invoke.resolve()
    }

    @Command
    fun cancelRecognition(invoke: Invoke) = host.runOnUiThread {
        val id = session(invoke)
        permission?.takeIf { session(it) == id }?.let {
            permission = null
            reject(it, "VOICE_CANCELLED")
        }
        recording?.takeIf { it.session == id }?.let { finishRecording(it, null, "VOICE_CANCELLED") }
        invoke.resolve()
    }

    private fun close() {
        stopPlayback()
        permission?.let { reject(it, "VOICE_CANCELLED") }
        permission = null
        recording?.let { finishRecording(it, null, "VOICE_CANCELLED") }
        val callback = initializing
        initializing = null
        callback?.invoke(null)
        engine?.shutdown()
        engine = null
        engineReady = false
        handler.removeCallbacksAndMessages(null)
    }
}
