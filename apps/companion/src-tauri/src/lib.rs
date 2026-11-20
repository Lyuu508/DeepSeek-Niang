mod companion;
#[cfg(target_os = "windows")]
mod desktop_pet;
mod sync;
#[cfg(target_os = "windows")]
mod video;
use companion_chat::{
    ChatFailure,
    deepseek::DeepSeekClient,
    service::{ChatEvent, ChatService},
};
use companion_core::{
    chat::{ChatMessage, ChatSettings, PreparedChat, StartChat, UsageSummary},
    persona::{Persona, PersonaSummary},
    storage::{Diagnostics, MemoryRecord, Store},
};
use companion_secrets::{Secret, SecretStore};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State, ipc::Channel};
use uuid::Uuid;

struct AppState(Arc<ChatService>);
#[derive(Serialize)]
struct KeyStatus {
    configured: bool,
    backend: String,
    error: Option<ChatFailure>,
}
#[derive(Serialize)]
struct Bootstrap {
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
async fn bootstrap(state: State<'_, AppState>) -> Result<Bootstrap, ChatFailure> {
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
) -> Result<PreparedChat, ChatFailure> {
    {
        let store = state
            .0
            .store
            .lock()
            .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
        store.permanent_persona(&input.conversation_id)?;
    }
    state
        .0
        .start(input, Arc::new(move |event| on_event.send(event).is_ok()))
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
    tauri::Builder::default()
        .plugin(tauri_plugin_companion_keystore::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let mut store = Store::open(directory.join("companion.sqlite3"))?;
            store.set_device_platform(std::env::consts::OS)?;
            let credentials = vault(app.handle(), &store.profile_id)?;
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
            std::thread::spawn(move || {
                loop {
                    let Some(service) = weak.upgrade() else { break };
                    let _ = tauri::async_runtime::block_on(service.life_tick(
                        companion_core::chat::now_ms(),
                        boot.elapsed().as_millis().min(i64::MAX as u128) as i64,
                        cfg!(target_os = "windows"),
                    ));
                    drop(service);
                    std::thread::sleep(std::time::Duration::from_secs(30));
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
        ])
        .run(tauri::generate_context!())
        .expect("native application startup failed");
}
