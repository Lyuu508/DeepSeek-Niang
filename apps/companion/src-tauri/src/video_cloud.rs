use super::AppState;
use companion_chat::{
    deepseek::DeepSeekClient,
    video_cloud::{self, QwenClient, QwenSettings, VideoInput, VideoRecord},
};
use companion_core::{chat::ChatSettings, storage::Store, video_device::VideoDeviceSlot};
use companion_secrets::{Secret, SecretStore, VaultError};
use serde::Serialize;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::{State, WebviewWindow};
use tokio::sync::Mutex as AsyncMutex;

type Reply<T> = Result<T, &'static str>;
pub struct CloudVideoHost {
    credentials: Arc<dyn SecretStore>,
    request: AsyncMutex<()>,
    last_test: Mutex<Option<(&'static str, QwenSettings)>>,
}

impl CloudVideoHost {
    pub fn new(credentials: Arc<dyn SecretStore>) -> Self {
        Self {
            credentials,
            request: AsyncMutex::new(()),
            last_test: Mutex::new(None),
        }
    }
}

#[derive(Serialize)]
pub struct ProviderView {
    settings: QwenSettings,
    configured: bool,
    connection: &'static str,
}

fn main_window(window: &WebviewWindow) -> Reply<()> {
    (window.label() == "main")
        .then_some(())
        .ok_or("VIDEO_WINDOW")
}

fn storage_error<T>(_: T) -> &'static str {
    "VIDEO_STORAGE_ERROR"
}

fn store(state: &AppState) -> Reply<MutexGuard<'_, Store>> {
    state.0.store.lock().map_err(storage_error)
}

fn active(store: &Store, persona: &str) -> Reply<()> {
    if store.active_persona_id().map_err(storage_error)? != persona {
        return Err("VIDEO_PERSONA_CHANGED");
    }
    Ok(())
}

fn read_settings(state: &AppState) -> Reply<QwenSettings> {
    store(state)?
        .read_video_device(VideoDeviceSlot::Provider)
        .map_err(storage_error)?
        .map(|value| serde_json::from_value(value).map_err(storage_error))
        .transpose()
        .map(|settings| settings.unwrap_or_default())
}

fn record(store: &Store, persona: &str) -> Reply<Option<VideoRecord>> {
    store
        .read_video_device(VideoDeviceSlot::Analysis(persona))
        .map_err(storage_error)?
        .map(|value| serde_json::from_value(value).map_err(storage_error))
        .transpose()
}

fn write_record(state: &AppState, persona: &str, value: &VideoRecord) -> Reply<()> {
    store(state)?
        .write_video_device(
            VideoDeviceSlot::Analysis(persona),
            Some(serde_json::to_value(value).map_err(storage_error)?),
        )
        .map_err(storage_error)
}

fn provider_view(state: &AppState, host: &CloudVideoHost) -> Reply<ProviderView> {
    let settings = read_settings(state)?;
    let configured = host.credentials.has().map_err(|_| "QWEN_STORAGE_FAILED")?;
    let connection = if !configured {
        "unconfigured"
    } else {
        host.last_test
            .lock()
            .map_err(storage_error)?
            .as_ref()
            .filter(|(_, tested)| tested == &settings)
            .map_or("untested", |(status, _)| *status)
    };
    Ok(ProviderView {
        settings,
        configured,
        connection,
    })
}

fn key_error(error: VaultError) -> &'static str {
    if matches!(error, VaultError::Missing) {
        "QWEN_KEY_REQUIRED"
    } else {
        "QWEN_STORAGE_FAILED"
    }
}

#[tauri::command]
pub async fn qwen_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
) -> Reply<ProviderView> {
    main_window(&window)?;
    provider_view(&state, &host)
}

#[tauri::command]
pub async fn qwen_settings(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    settings: QwenSettings,
    key: Option<String>,
    delete_key: bool,
) -> Reply<ProviderView> {
    main_window(&window)?;
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    settings.validate()?;
    if let Some(key) = key {
        if delete_key {
            return Err("QWEN_SETTINGS_INVALID");
        }
        let key = Secret::new(key).map_err(|_| "QWEN_KEY_INVALID")?;
        host.credentials.set(&key).map_err(key_error)?;
        if host.credentials.get().map_err(key_error)?.expose() != key.expose() {
            return Err("QWEN_STORAGE_FAILED");
        }
    } else if delete_key {
        host.credentials.delete().map_err(key_error)?;
    }
    store(&state)?
        .write_video_device(
            VideoDeviceSlot::Provider,
            Some(serde_json::to_value(settings).map_err(storage_error)?),
        )
        .map_err(storage_error)?;
    *host.last_test.lock().map_err(storage_error)? = None;
    provider_view(&state, &host)
}

#[tauri::command]
pub async fn qwen_connection_test(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    authorized: bool,
) -> Reply<ProviderView> {
    main_window(&window)?;
    if !authorized {
        return Err("VIDEO_CONSENT_REQUIRED");
    }
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    let settings = read_settings(&state)?;
    let key = host.credentials.get().map_err(key_error)?;
    let result = QwenClient::new(settings.clone())?
        .connection_test(&key)
        .await;
    *host.last_test.lock().map_err(storage_error)? =
        Some((if result.is_ok() { "ready" } else { "error" }, settings));
    result?;
    provider_view(&state, &host)
}

#[tauri::command]
pub async fn cloud_video_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
    persona: String,
) -> Reply<Option<VideoRecord>> {
    main_window(&window)?;
    let store = store(&state)?;
    active(&store, &persona)?;
    record(&store, &persona)
}

fn saved_and_settings(
    state: &AppState,
    persona: &str,
    id: &str,
) -> Reply<(VideoRecord, ChatSettings)> {
    let store = store(state)?;
    active(&store, persona)?;
    let saved = record(&store, persona)?
        .filter(|value| value.id == id)
        .ok_or("VIDEO_RECORD_CHANGED")?;
    Ok((saved, store.chat_settings().map_err(storage_error)?))
}

fn deepseek_key(state: &AppState) -> Reply<Secret> {
    state
        .0
        .vault
        .get()
        .map_err(|_| "VIDEO_DEEPSEEK_KEY_REQUIRED")
}

#[tauri::command]
pub async fn cloud_video_plan(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    persona: String,
    question: String,
    duration_ms: u32,
    authorized: bool,
) -> Reply<VideoRecord> {
    main_window(&window)?;
    if !authorized {
        return Err("VIDEO_CONSENT_REQUIRED");
    }
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    let key = deepseek_key(&state)?;
    let _ = host.credentials.get().map_err(key_error)?;
    read_settings(&state)?.validate()?;
    let settings = {
        let store = store(&state)?;
        active(&store, &persona)?;
        store.chat_settings().map_err(storage_error)?
    };
    let question = question.trim().to_owned();
    let engine = DeepSeekClient::official().map_err(|_| "VIDEO_DEEPSEEK_FAILED")?;
    let plan = video_cloud::plan(&engine, &key, &settings, &question, duration_ms).await?;
    let value = VideoRecord {
        id: uuid::Uuid::new_v4().to_string(),
        question,
        duration_ms,
        plan,
        evidence: None,
        answer: None,
    };
    write_record(&state, &persona, &value)?;
    Ok(value)
}

#[tauri::command]
pub async fn cloud_video_recognize(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    persona: String,
    id: String,
    authorized: bool,
    input: VideoInput,
) -> Reply<VideoRecord> {
    main_window(&window)?;
    if !authorized {
        return Err("VIDEO_CONSENT_REQUIRED");
    }
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    let (mut saved, _) = saved_and_settings(&state, &persona, &id)?;
    if saved.evidence.is_some() {
        return Ok(saved);
    }
    let key = host.credentials.get().map_err(key_error)?;
    let client = QwenClient::new(read_settings(&state)?)?;
    saved.evidence = Some(client.recognize(&key, &saved, input).await?);
    write_record(&state, &persona, &saved)?;
    Ok(saved)
}

#[tauri::command]
pub async fn cloud_video_answer(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    persona: String,
    id: String,
    authorized: bool,
) -> Reply<VideoRecord> {
    main_window(&window)?;
    if !authorized {
        return Err("VIDEO_CONSENT_REQUIRED");
    }
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    let (mut saved, settings) = saved_and_settings(&state, &persona, &id)?;
    let key = deepseek_key(&state)?;
    let engine = DeepSeekClient::official().map_err(|_| "VIDEO_DEEPSEEK_FAILED")?;
    saved.answer = Some(video_cloud::answer(&engine, &key, &settings, &saved).await?);
    write_record(&state, &persona, &saved)?;
    Ok(saved)
}

#[tauri::command]
pub async fn cloud_video_clear(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVideoHost>,
    persona: String,
) -> Reply<()> {
    main_window(&window)?;
    let _guard = host.request.try_lock().map_err(|_| "VIDEO_BUSY")?;
    let store = store(&state)?;
    active(&store, &persona)?;
    store
        .write_video_device(VideoDeviceSlot::Analysis(&persona), None)
        .map_err(storage_error)
}
