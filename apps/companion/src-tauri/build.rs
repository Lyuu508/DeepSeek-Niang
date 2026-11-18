fn main() {
    // Once an application has a second WebView, every custom command needs an explicit audience.
    let commands = &[
        "sync_status",
        "sync_command",
        "bootstrap",
        "list_messages",
        "create_persona",
        "select_persona",
        "save_chat_settings",
        "set_api_key",
        "delete_api_key",
        "vault_self_test",
        "start_chat",
        "stop_generation",
        "diagnostics",
        "list_memories",
        "save_memory",
        "delete_memory",
        "companion_snapshot",
        "save_companion_preferences",
        "edit_companion_memory",
        "resolve_memory_candidate",
        "life_plan_create",
        "life_task_control",
        "life_diary",
        "share_receipt",
        "claim_share_notification",
        "desktop_pet_settings",
        "desktop_pet_configure",
        "pet_snapshot",
        "pet_motion",
        "pet_action",
        "pet_send",
        "pet_stop",
    ];
    let manifest = tauri_build::AppManifest::new().commands(commands);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("Tauri application permissions could not be built");
}
