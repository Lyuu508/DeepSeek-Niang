use crate::AppState;
use base64::{Engine, engine::general_purpose::STANDARD};
use companion_core::{
    storage::Store,
    voice::{GuideNote, VoicePreferences, guide_take},
};
use serde::{Deserialize, Serialize};
use std::sync::MutexGuard;
use tauri::{State, WebviewWindow};

#[cfg(target_os = "android")]
use tauri::{Manager, Runtime};
#[cfg(target_os = "windows")]
pub mod windows;

type Reply<T> = Result<T, &'static str>;

#[derive(Serialize, Deserialize)]
pub struct Voice {
    id: String,
    name: String,
    language: String,
}

#[derive(Serialize, Deserialize)]
pub struct Take {
    wav: String,
}

const MAX_WAV: usize = 8 * 1024 * 1024;

impl Take {
    fn from_wav(bytes: &[u8]) -> Reply<Self> {
        if !(44..=MAX_WAV).contains(&bytes.len())
            || &bytes[..4] != b"RIFF"
            || &bytes[8..12] != b"WAVE"
        {
            return Err("VOICE_INVALID_AUDIO");
        }
        Ok(Self {
            wav: STANDARD.encode(bytes),
        })
    }
}

#[derive(Serialize)]
pub struct Guide {
    audio: Take,
    notes: &'static [GuideNote],
}

fn main_only(window: &WebviewWindow) -> Reply<()> {
    if window.label() != "main" {
        return Err("VOICE_UNAVAILABLE");
    }
    Ok(())
}

fn active<'a>(
    window: &WebviewWindow,
    state: &'a AppState,
    persona: &str,
) -> Reply<MutexGuard<'a, Store>> {
    main_only(window)?;
    let store = state.0.store.lock().map_err(|_| "VOICE_STORAGE")?;
    if store.active_persona_id().map_err(|_| "VOICE_STORAGE")? != persona {
        return Err("VOICE_PERSONA_CHANGED");
    }
    Ok(store)
}

fn valid_session(session: &str) -> Reply<()> {
    uuid::Uuid::parse_str(session)
        .map(|_| ())
        .map_err(|_| "VOICE_UNAVAILABLE")
}

#[tauri::command]
pub fn voice_preferences(
    window: WebviewWindow,
    state: State<'_, AppState>,
    persona_id: String,
    update: Option<VoicePreferences>,
) -> Reply<VoicePreferences> {
    let store = active(&window, &state, &persona_id)?;
    if let Some(prefs) = update {
        prefs.validate().map_err(|_| "VOICE_INVALID_SETTINGS")?;
        store
            .save_voice_preferences(&persona_id, &prefs)
            .map_err(|_| "VOICE_STORAGE")?;
    }
    store
        .voice_preferences(&persona_id)
        .map_err(|_| "VOICE_STORAGE")
}

#[tauri::command]
pub async fn voice_voices(window: WebviewWindow) -> Reply<Vec<Voice>> {
    main_only(&window)?;
    #[cfg(target_os = "windows")]
    return windows::voices().await;
    #[cfg(target_os = "android")]
    return mobile(&window, "listVoices", serde_json::json!({})).await;
    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    Err("VOICE_UNAVAILABLE")
}

#[tauri::command]
pub async fn voice_speak(
    window: WebviewWindow,
    state: State<'_, AppState>,
    persona_id: String,
    session: String,
    text: String,
) -> Reply<Take> {
    valid_session(&session)?;
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 280 {
        return Err("VOICE_TEXT_LENGTH");
    }
    let (prefs, speed, pitch) = {
        let store = active(&window, &state, &persona_id)?;
        let prefs = store
            .voice_preferences(&persona_id)
            .map_err(|_| "VOICE_STORAGE")?;
        let mood = store
            .current_mood(&persona_id, companion_core::chat::now_ms())
            .map_err(|_| "VOICE_STORAGE")?;
        let (speed, pitch) = prefs.expression(&mood.kind, mood.intensity_milli);
        (prefs, speed, pitch)
    };
    #[cfg(target_os = "windows")]
    return windows::speak(&window, prefs.voice_id, speed, pitch, text.into()).await;
    #[cfg(target_os = "android")]
    return mobile(
        &window, "synthesize",
        serde_json::json!({"session":session,"voiceId":prefs.voice_id,"speed":speed,"pitch":pitch,"text":text}),
    ).await;
    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        let _ = (prefs, speed, pitch);
        Err("VOICE_UNAVAILABLE")
    }
}

#[tauri::command]
pub fn voice_guide(
    window: WebviewWindow,
    state: State<'_, AppState>,
    persona_id: String,
) -> Reply<Guide> {
    drop(active(&window, &state, &persona_id)?);
    let (notes, wav) = guide_take();
    Ok(Guide {
        audio: Take::from_wav(&wav)?,
        notes,
    })
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Focus {
    Acquire,
    Status,
    Release,
}

#[tauri::command]
pub async fn voice_focus(window: WebviewWindow, session: String, action: Focus) -> Reply<bool> {
    main_only(&window)?;
    valid_session(&session)?;
    #[cfg(target_os = "android")]
    {
        let command = match action {
            Focus::Acquire => "acquireFocus",
            Focus::Status => "hasFocus",
            Focus::Release => "releaseFocus",
        };
        mobile(&window, command, serde_json::json!({"session":session})).await
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = action;
        Ok(true)
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Recording {
    Check,
    Begin,
    Status,
    Finish,
    Cancel,
    Transcribe { pcm: String },
}

#[tauri::command]
pub async fn voice_record(
    window: WebviewWindow,
    state: State<'_, AppState>,
    persona_id: String,
    session: String,
    action: Recording,
) -> Reply<Option<String>> {
    main_only(&window)?;
    valid_session(&session)?;
    if !matches!(action, Recording::Cancel) {
        drop(active(&window, &state, &persona_id)?);
    }
    #[cfg(target_os = "windows")]
    return windows::record(&window, session, action).await;
    #[cfg(target_os = "android")]
    {
        let command = match action {
            Recording::Check => "checkRecognition",
            Recording::Begin => "beginRecognition",
            Recording::Status => "recognitionStatus",
            Recording::Finish => "finishRecognition",
            Recording::Cancel => "cancelRecognition",
            Recording::Transcribe { pcm } => {
                drop(pcm);
                return Err("VOICE_UNAVAILABLE");
            }
        };
        mobile(&window, command, serde_json::json!({"session":session})).await
    }
    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        if let Recording::Transcribe { pcm } = action {
            drop(pcm);
        }
        Err("VOICE_UNAVAILABLE")
    }
}

#[cfg(target_os = "android")]
async fn mobile<R: Runtime, T: serde::de::DeserializeOwned>(
    window: &WebviewWindow<R>,
    command: &str,
    payload: serde_json::Value,
) -> Reply<T> {
    let handle = window.state::<tauri_plugin_companion_voice::VoiceHandle<R>>();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(55),
        handle.0.run_mobile_plugin_async(command, payload),
    )
    .await
    .map_err(|_| "VOICE_TIMEOUT")?;
    result.map_err(|error| {
        let tauri::plugin::mobile::PluginInvokeError::InvokeRejected(info) = error else {
            return "VOICE_UNAVAILABLE";
        };
        match info.code.as_deref() {
            Some("VOICE_NO_OFFLINE_VOICE") => "VOICE_NO_OFFLINE_VOICE",
            Some("VOICE_VOICE_REMOVED") => "VOICE_VOICE_REMOVED",
            Some("VOICE_NO_ON_DEVICE_RECOGNIZER") => "VOICE_NO_ON_DEVICE_RECOGNIZER",
            Some("VOICE_RECOGNIZER_LANGUAGE") => "VOICE_RECOGNIZER_LANGUAGE",
            Some("VOICE_PERMISSION_DENIED") => "VOICE_PERMISSION_DENIED",
            Some("VOICE_FOCUS_LOST") => "VOICE_FOCUS_LOST",
            Some("VOICE_CANCELLED") => "VOICE_CANCELLED",
            Some("VOICE_BUSY") => "VOICE_BUSY",
            Some("VOICE_NO_SPEECH") => "VOICE_NO_SPEECH",
            Some("VOICE_TIMEOUT") => "VOICE_TIMEOUT",
            Some("VOICE_INVALID_AUDIO") => "VOICE_INVALID_AUDIO",
            Some("VOICE_RECOGNITION_FAILED") => "VOICE_RECOGNITION_FAILED",
            Some("VOICE_SYNTHESIS_FAILED") => "VOICE_SYNTHESIS_FAILED",
            _ => "VOICE_UNAVAILABLE",
        }
    })
}
