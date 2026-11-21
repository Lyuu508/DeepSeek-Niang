mod companion;
#[cfg(target_os = "windows")]
mod desktop_pet;
mod sync;
mod update;
#[cfg(target_os = "windows")]
mod video;
mod video_cloud;
mod voice;
use companion_chat::{
    ChatFailure,
    deepseek::DeepSeekClient,
    service::{ChatEvent, ChatService},
};
use companion_core::{
    chat::{ChatMessage, ChatSettings, PreparedChat, StartChat, UsageSummary},
    persona::{Persona, PersonaSummary},
    plugins::{Action as PluginAction, DueReminder, Plugin, Preview as PluginPreview},
    recycle::{RecycleAction, RecycledPersona},
    storage::{Diagnostics, MemoryRecord, Store},
};
use companion_secrets::{Secret, SecretStore};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, ipc::Channel};
use uuid::Uuid;

struct AppState(Arc<ChatService>);

#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn voice_self_test() -> i32 {
    voice::windows::probe()
}
#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn run_update_worker_if_requested() {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == update::windows::WORKER_ARG)
    {
        std::process::exit(i32::from(update::windows::run_worker().is_err()));
    }
}
#[derive(Serialize)]
struct KeyStatus {
    configured: bool,
    backend: String,
    error: Option<ChatFailure>,
}
#[derive(Serialize)]
struct Bootstrap {
    font_scale: u8,
    version: &'static str,
    platform: &'static str,
    settings: ChatSettings,
    personas: Vec<Persona>,
    active_persona_id: String,
    persona_summary: PersonaSummary,
    key_status: KeyStatus,
    usage: UsageSummary,
    active_request: Option<String>,
    active_conversation: Option<String>,
    schema_version: u32,
}
fn key_status(service: &ChatService) -> KeyStatus {
    match service.vault.has() {
        Ok(configured) => KeyStatus {
            configured,
            backend: service.vault.backend_name().into(),
            error: None,
        },
        Err(error) => KeyStatus {
            configured: false,
            backend: service.vault.backend_name().into(),
            error: Some(error.into()),
        },
    }
}
fn vault(_app: &tauri::AppHandle, profile: &str) -> Result<Arc<dyn SecretStore>, ChatFailure> {
    #[cfg(target_os = "windows")]
    {
        Ok(Arc::new(companion_secrets::WindowsVault::new(profile)?))
    }
    #[cfg(target_os = "android")]
    {
        let handle = _app
            .state::<tauri_plugin_companion_keystore::KeystoreHandle<tauri::Wry>>()
            .inner()
            .clone();
        Ok(Arc::new(
            tauri_plugin_companion_keystore::AndroidVault::new(handle, profile.into()),
        ))
    }
    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    {
        let _ = profile;
        Ok(Arc::new(companion_secrets::UnsupportedVault))
    }
}
#[tauri::command]
async fn bootstrap(
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Bootstrap, ChatFailure> {
    cancel_inactive(&state, &window);
    let key_status = key_status(&state.0);
    let (active_request, active_conversation) = state
        .0
        .active_generation()
        .map(|(request, conversation)| (Some(request), Some(conversation)))
        .unwrap_or((None, None));
    let store = state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
    let active_persona_id = store.active_persona_id()?;
    Ok(Bootstrap {
        font_scale: store.font_scale()?,
        personas: store.personas()?,
        persona_summary: store.persona_summary(&active_persona_id)?,
        active_persona_id,
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        settings: store.chat_settings()?,
        key_status,
        usage: store.usage_summary()?,
        active_request,
        active_conversation,
        schema_version: store.diagnostics()?.schema_version,
    })
}
#[tauri::command]
fn set_font_scale(percent: u8, state: State<'_, AppState>) -> Result<(), ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .set_font_scale(percent)?)
}
fn cancel_inactive(state: &AppState, _window: &tauri::WebviewWindow) {
    state.0.cancel_inactive_personas();
    #[cfg(target_os = "windows")]
    _window
        .state::<Arc<video::VideoHost>>()
        .stop_inactive_personas();
}

#[tauri::command]
fn list_recycled_personas(state: State<'_, AppState>) -> Result<Vec<RecycledPersona>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .recycled_personas()?)
}

#[tauri::command]
fn change_recycled_personas(
    action: RecycleAction,
    state: State<'_, AppState>,
    window: tauri::WebviewWindow,
) -> Result<Vec<String>, ChatFailure> {
    let conversations = state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .change_recycle(action)?;
    cancel_inactive(&state, &window);
    Ok(conversations)
}
#[tauri::command]
fn inspect_plugin(
    raw: String,
    source: String,
    state: State<'_, AppState>,
) -> Result<PluginPreview, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .inspect_plugin(&raw, &source)?)
}
#[tauri::command]
fn list_plugins(persona: String, state: State<'_, AppState>) -> Result<Vec<Plugin>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .list_plugins(&persona)?)
}
#[tauri::command]
fn change_plugin(
    persona: String,
    action: PluginAction,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .apply_plugin(&persona, action)?)
}
#[tauri::command]
fn due_plugin_reminders(
    persona: String,
    state: State<'_, AppState>,
) -> Result<Vec<DueReminder>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .due_plugin_reminders(&persona, companion_core::chat::now_ms())?)
}
#[tauri::command]
async fn list_messages(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, ChatFailure> {
    let store = state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
    store.permanent_persona(&conversation_id)?;
    Ok(store.chat_messages(&conversation_id)?)
}
#[tauri::command]
async fn create_persona(
    name: String,
    personality: String,
    state: State<'_, AppState>,
) -> Result<Persona, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .create_persona(&name, &personality)?)
}
#[tauri::command]
async fn select_persona(id: String, state: State<'_, AppState>) -> Result<(), ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .select_persona(&id)?)
}
#[tauri::command]
async fn save_chat_settings(
    persona_id: String,
    settings: ChatSettings,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .save_persona_settings(&persona_id, &settings)?)
}
#[tauri::command]
async fn set_api_key(key: String, state: State<'_, AppState>) -> Result<KeyStatus, ChatFailure> {
    state.0.replace_key(Some(Secret::new(key)?)).await?;
    Ok(key_status(&state.0))
}
#[tauri::command]
async fn delete_api_key(state: State<'_, AppState>) -> Result<KeyStatus, ChatFailure> {
    state.0.replace_key(None).await?;
    Ok(key_status(&state.0))
}
#[tauri::command]
async fn vault_self_test(app: tauri::AppHandle) -> Result<String, ChatFailure> {
    let temporary = vault(&app, &Uuid::new_v4().to_string())?;
    let key = Secret::new(format!("fixture-only-selftest-{}", Uuid::new_v4()))?;
    let result = (|| -> Result<(), ChatFailure> {
        temporary.set(&key)?;
        if temporary.get()?.expose() != key.expose() {
            return Err(ChatFailure::new("SECURE_STORAGE_CORRUPT"));
        }
        Ok(())
    })();
    let cleanup = temporary.delete();
    result?;
    cleanup?;
    if temporary.has()? {
        return Err(ChatFailure::new("SECURE_STORAGE_CORRUPT"));
    }
    Ok(temporary.backend_name().into())
}
#[tauri::command]
async fn start_chat(
    input: StartChat,
    on_event: Channel<ChatEvent>,
    state: State<'_, AppState>,
    image: Option<companion_core::chat_images::ChatImage>,
    images: Option<Vec<companion_core::chat_images::ChatImage>>,
) -> Result<PreparedChat, ChatFailure> {
    {
        let store = state
            .0
            .store
            .lock()
            .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
        store.permanent_persona(&input.conversation_id)?;
    }
    let observer = Arc::new(move |event| on_event.send(event).is_ok());
    match (image, images) {
        (None, Some(images)) => state.0.start_with_images(input, observer, &images),
        (Some(image), None) => state.0.start_with_image(input, observer, image),
        (None, None) => state.0.start(input, observer),
        _ => Err(ChatFailure::new("CHAT_IMAGE_INVALID")),
    }
}
#[tauri::command]
async fn chat_image_ids(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .local_chat_images(&conversation_id)?)
}
#[tauri::command]
async fn chat_image(
    conversation_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<companion_core::chat_images::ChatImage>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .local_chat_image(&conversation_id, &id)?)
}
#[tauri::command]
async fn chat_gallery(
    window: tauri::WebviewWindow,
    conversation_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<companion_core::chat_images::ChatImage>, ChatFailure> {
    if window.label() != "main" {
        return Err(ChatFailure::new("CHAT_IMAGE_INVALID"));
    }
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .local_chat_gallery(&conversation_id, &id)?)
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StickerAction {
    List,
    Add {
        tag: String,
        image: companion_core::chat_images::ChatImage,
    },
    Remove {
        id: String,
    },
    Automatic {
        enabled: bool,
    },
}

#[tauri::command]
async fn sticker_catalog(
    window: tauri::WebviewWindow,
    state: State<'_, AppState>,
    persona: String,
    action: StickerAction,
) -> Result<companion_core::stickers::StickerCatalog, ChatFailure> {
    if window.label() != "main" {
        return Err(ChatFailure::new("INVALID_INPUT"));
    }
    let store = state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
    if store.active_persona_id()? != persona {
        return Err(ChatFailure::new("INVALID_INPUT"));
    }
    match action {
        StickerAction::List => {}
        StickerAction::Add { tag, image } => store.add_sticker(&persona, tag, image)?,
        StickerAction::Remove { id } => store.remove_sticker(&persona, &id)?,
        StickerAction::Automatic { enabled } => store.set_automatic_stickers(&persona, enabled)?,
    }
    Ok(store.stickers(&persona)?)
}

#[tauri::command]
async fn stop_generation(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<bool, ChatFailure> {
    state.0.stop_and_wait(&request_id).await
}
#[tauri::command]
async fn diagnostics(state: State<'_, AppState>) -> Result<Diagnostics, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .diagnostics()?)
}
#[tauri::command]
async fn list_memories(
    persona_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MemoryRecord>, ChatFailure> {
    Ok(state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .memories_for_persona(&persona_id)?)
}
#[tauri::command]
async fn save_memory(
    persona_id: String,
    text: String,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .put_persona_memory(&persona_id, &Uuid::new_v4().to_string(), &text)?;
    Ok(())
}
#[tauri::command]
async fn delete_memory(
    persona_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .delete_persona_memory(&persona_id, &id)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(tauri_plugin_companion_voice::init());
    builder
        .plugin(tauri_plugin_companion_keystore::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(update::plugin())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let mut store = Store::open(directory.join("companion.sqlite3"))?;
            store.set_device_platform(std::env::consts::OS)?;
            let credentials = vault(app.handle(), &store.profile_id)?;
            // Derive separate slots for video and cloud speech credentials.
            let mut qwen_slot = *Uuid::parse_str(&store.profile_id)?.as_bytes();
            qwen_slot[0] ^= 0x52;
            qwen_slot[15] ^= 0x81;
            let qwen_vault = vault(app.handle(), &Uuid::from_bytes(qwen_slot).to_string())?;
            app.manage(video_cloud::CloudVideoHost::new(qwen_vault.clone()));
            let mut voice_slot = *Uuid::parse_str(&store.profile_id)?.as_bytes();
            voice_slot[0] ^= 0x7A;
            voice_slot[15] ^= 0x32;
            let voice_vault = vault(app.handle(), &Uuid::from_bytes(voice_slot).to_string())?;
            app.manage(voice::cloud::CloudVoiceHost::new(voice_vault.clone()));
            let service = Arc::new(ChatService::new(
                Arc::new(Mutex::new(store)),
                credentials,
                DeepSeekClient::official()?,
            ));
            let weak = Arc::downgrade(&service);
            let boot = std::time::Instant::now();
            #[cfg(target_os = "windows")]
            app.manage(Arc::new(video::VideoHost::new(
                service.store.clone(),
                directory.join("video-work"),
                boot,
            )));
            #[cfg(target_os = "windows")]
            app.manage(voice::windows::VoiceHost::new(directory.join("voice-work")));
            std::thread::spawn(move || {
                loop {
                    let Some(service) = weak.upgrade() else { break };
                    let cycle = std::time::Instant::now();
                    let _ = tauri::async_runtime::block_on(service.life_tick_delayed(
                        companion_core::chat::now_ms(),
                        boot.elapsed().as_millis().min(i64::MAX as u128) as i64,
                        cfg!(target_os = "windows"),
                    ));
                    drop(service);
                    std::thread::sleep(
                        std::time::Duration::from_secs(30).saturating_sub(cycle.elapsed()),
                    );
                }
            });
            // Separate secure-storage slot from the local DeepSeek API key.
            let mut slot =
                *Uuid::parse_str(&service.store.lock().map_err(|_| "storage lock")?.device_id)?
                    .as_bytes();
            slot[0] ^= 0xA7;
            slot[15] ^= 0x62;
            let sync_vault = vault(app.handle(), &Uuid::from_bytes(slot).to_string())?;
            let host = Arc::new(sync::SyncHost {
                suspended: std::sync::atomic::AtomicBool::new(false),
                store: service.store.clone(),
                credentials: sync_vault,
                api_credentials: service.vault.clone(),
                providers: Arc::new(sync::PairedProviders {
                    store: service.store.clone(),
                    qwen: qwen_vault,
                    speech: voice_vault,
                }),
                discovery: sync::discovery(app.handle()),
                service: Mutex::new(None),
            });
            app.manage(host);
            app.manage(AppState(service));
            #[cfg(target_os = "windows")]
            desktop_pet::install(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "windows")]
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                window.state::<Arc<video::VideoHost>>().stop();
                window.app_handle().exit(0);
            }
            #[cfg(not(target_os = "windows"))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            set_font_scale,
            list_recycled_personas,
            change_recycled_personas,
            inspect_plugin,
            list_plugins,
            change_plugin,
            chat_image_ids,
            chat_image,
            chat_gallery,
            sticker_catalog,
            update::update_check,
            update::update_download,
            update::update_install,
            video_cloud::qwen_status,
            video_cloud::qwen_settings,
            video_cloud::qwen_connection_test,
            video_cloud::cloud_video_status,
            video_cloud::cloud_video_plan,
            video_cloud::cloud_video_recognize,
            video_cloud::cloud_video_answer,
            video_cloud::cloud_video_clear,
            video_cloud::cloud_video_save_chat,
            due_plugin_reminders,
            sync::sync_status,
            sync::sync_command,
            bootstrap,
            list_messages,
            create_persona,
            select_persona,
            save_chat_settings,
            set_api_key,
            delete_api_key,
            vault_self_test,
            start_chat,
            stop_generation,
            diagnostics,
            list_memories,
            save_memory,
            delete_memory,
            companion::companion_snapshot,
            companion::save_companion_preferences,
            companion::edit_companion_memory,
            companion::resolve_memory_candidate,
            companion::life_plan_create,
            companion::life_task_control,
            companion::life_diary,
            companion::share_receipt,
            companion::claim_share_notification,
            #[cfg(target_os = "windows")]
            desktop_pet::desktop_pet_settings,
            #[cfg(target_os = "windows")]
            desktop_pet::desktop_pet_configure,
            #[cfg(target_os = "windows")]
            desktop_pet::pet_snapshot,
            #[cfg(target_os = "windows")]
            desktop_pet::pet_motion,
            #[cfg(target_os = "windows")]
            desktop_pet::pet_action,
            #[cfg(target_os = "windows")]
            desktop_pet::pet_send,
            #[cfg(target_os = "windows")]
            desktop_pet::pet_stop,
            #[cfg(target_os = "windows")]
            video::video_list,
            #[cfg(target_os = "windows")]
            video::video_select_whisper,
            #[cfg(target_os = "windows")]
            video::video_import,
            #[cfg(target_os = "windows")]
            video::video_control,
            voice::voice_preferences,
            voice::cloud::cloud_voice_status,
            voice::cloud::cloud_voice_save,
            voice::cloud::cloud_voice_auto,
            voice::voice_voices,
            voice::voice_speak,
            voice::voice_guide,
            voice::voice_focus,
            voice::voice_record,
            #[cfg(target_os = "windows")]
            voice::windows::voice_meter,
            #[cfg(target_os = "windows")]
            voice::windows::pet_speech_level,
        ])
        .run(tauri::generate_context!())
        .expect("native application startup failed");
}
