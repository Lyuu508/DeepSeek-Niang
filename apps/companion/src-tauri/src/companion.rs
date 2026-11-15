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
    persona_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, ChatFailure> {
    state
        .0
        .store
        .lock()
        .map_err(|_| ChatFailure::new("STORAGE_ERROR"))?
        .companion_snapshot(&persona_id, companion_core::chat::now_ms())
        .map_err(error)
}
#[tauri::command]
pub async fn save_companion_preferences(
    persona_id: String,
    preferences: CompanionPreferences,
    state: State<'_, AppState>,
) -> Result<(), ChatFailure> {
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
