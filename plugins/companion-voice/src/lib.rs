//! Native bridge, reachable only through the main window's Rust commands.
use tauri::{
    Runtime,
    plugin::{Builder, TauriPlugin},
};

#[cfg(target_os = "android")]
pub struct VoiceHandle<R: Runtime>(pub tauri::plugin::PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("companion-voice")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle = _api
                    .register_android_plugin("io.local.companion.voice", "CompanionVoicePlugin")?;
                _app.manage(VoiceHandle(handle));
            }
            Ok(())
        })
        .build()
}
