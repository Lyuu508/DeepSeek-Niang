use super::AppState;
use companion_chat::ChatFailure;
use companion_core::companion::{CompanionPreferences, memory::MemoryEdit};
use tauri::State;

fn error(e: companion_core::Error) -> ChatFailure {
    if matches!(e, companion_core::Error::Conflict) {
        ChatFailure::new("DATA_VERSION_CONFLICT")
    } else {
        e.into()
    }
}
#[tauri::command]
pub async fn companion_snapshot(
    window: tauri::Window,
    persona_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, ChatFailure> {
    let mut store = state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?;
    let now = companion_core::chat::now_ms();
    store
        .record_life_presence(now, window.is_focused().unwrap_or(false))
        .map_err(error)?;
    store.companion_snapshot(&persona_id, now).map_err(error)
}
#[tauri::command]
pub async fn save_companion_preferences(
    persona_id: String,
    preferences: CompanionPreferences,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    if !preferences.proactive_enabled
        || !preferences.proactive_model_enabled
        || preferences.proactive_mode == "off"
    {
        state.0.cancel_life_generation();
    }
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .save_companion_preferences(&persona_id, &preferences)
        .map_err(error)
}
#[tauri::command]
pub async fn edit_companion_memory(
    persona_id: String,
    edit: MemoryEdit,
    state: State<'_, AppState>,
) -> Result<String, ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .edit_companion_memory(&persona_id, &edit)
        .map_err(error)
}
#[tauri::command]
pub async fn resolve_memory_candidate(
    persona_id: String,
    id: String,
    accept: bool,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .resolve_memory_candidate(&persona_id, &id, accept)
        .map_err(error)
}
#[tauri::command]
pub async fn life_plan_create(
    persona_id: String,
    input: companion_core::companion::living::LifePlanInput,
    state: State<'_, AppState>,
) -> Result<String, ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .create_life_plan(&persona_id, &input, companion_core::chat::now_ms())
        .map_err(error)
}
#[tauri::command]
pub async fn life_task_control(
    persona_id: String,
    task_id: String,
    action: String,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .control_life_task(
            &persona_id,
            &task_id,
            &action,
            companion_core::chat::now_ms(),
        )
        .map_err(error)
}
#[tauri::command]
pub async fn life_diary(
    persona_id: String,
    day: i64,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, ChatFailure> {
    if !(0..=1000000).contains(&day) {
        return Err(ChatFailure::new("INVALID_INPUT"));
    }
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .life_diary(&persona_id, day)
        .map_err(error)
}
#[tauri::command]
pub async fn share_receipt(
    persona_id: String,
    id: String,
    read: bool,
    notification: String,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .record_share_receipt(
            &persona_id,
            &id,
            read,
            &notification,
            companion_core::chat::now_ms(),
        )
        .map_err(error)
}
#[tauri::command]
pub async fn claim_share_notification(
    persona_id: String,
    id: String,
    state: State<'_, AppState>,
) -> Result<bool, ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .claim_share_notification(&persona_id, &id, companion_core::chat::now_ms())
        .map_err(error)
}
