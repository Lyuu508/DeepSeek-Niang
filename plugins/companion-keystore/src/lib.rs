//! No JS command handler: reading plaintext is only accessible through native Rust.
use tauri::{
    Runtime,
    plugin::{Builder, TauriPlugin},
};
#[cfg(target_os = "android")]
pub struct KeystoreHandle<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(target_os = "android")]
impl<R: Runtime> Clone for KeystoreHandle<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
#[cfg(target_os = "android")]
pub struct AndroidVault<R: Runtime> {
    handle: KeystoreHandle<R>,
    profile: String,
}
#[cfg(target_os = "android")]
impl<R: Runtime> AndroidVault<R> {
    pub fn new(handle: KeystoreHandle<R>, profile: String) -> Self {
        Self { handle, profile }
    }
}
#[cfg(target_os = "android")]
impl<R: Runtime> companion_secrets::SecretStore for AndroidVault<R> {
    fn backend_name(&self) -> &'static str {
        "Android Keystore / AES-GCM"
    }
    fn get(&self) -> Result<companion_secrets::Secret, companion_secrets::VaultError> {
        #[derive(serde::Deserialize)]
        struct Reply {
            key: Option<String>,
        }
        let reply: Reply = self
            .handle
            .0
            .run_mobile_plugin("readSecret", serde_json::json!({"profileId":self.profile}))
            .map_err(|_| companion_secrets::VaultError::Unavailable)?;
        companion_secrets::Secret::new(reply.key.ok_or(companion_secrets::VaultError::Missing)?)
            .map_err(|_| companion_secrets::VaultError::Corrupt)
    }
    fn set(&self, key: &companion_secrets::Secret) -> Result<(), companion_secrets::VaultError> {
        self.handle
            .0
            .run_mobile_plugin::<serde_json::Value>(
                "writeSecret",
                serde_json::json!({"profileId":self.profile,"key":key.expose()}),
            )
            .map(|_| ())
            .map_err(|_| companion_secrets::VaultError::Unavailable)
    }
    fn delete(&self) -> Result<(), companion_secrets::VaultError> {
        self.handle
            .0
            .run_mobile_plugin::<serde_json::Value>(
                "deleteSecret",
                serde_json::json!({"profileId":self.profile}),
            )
            .map(|_| ())
            .map_err(|_| companion_secrets::VaultError::Unavailable)
    }
}
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("companion-keystore")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle = _api.register_android_plugin(
                    "io.local.companion.keystore",
                    "CompanionKeystorePlugin",
                )?;
                _app.manage(KeystoreHandle(handle));
            }
            Ok(())
        })
        .build()
}
