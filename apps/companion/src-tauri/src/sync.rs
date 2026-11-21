use companion_core::{storage::Store, video_device::VideoDeviceSlot};
use companion_secrets::{SecretStore, VaultError};
use companion_sync::{
    Error, PairProvider, PairProviderKind, ProviderHandoff, SyncService, discovery::Discovery,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::State;

fn sync_error(error: impl Into<companion_sync::Error>) -> String {
    error.into().code().to_owned()
}

pub struct PairedProviders {
    pub store: Arc<Mutex<Store>>,
    pub qwen: Arc<dyn SecretStore>,
    pub speech: Arc<dyn SecretStore>,
}

impl PairedProviders {
    fn vault(&self, kind: PairProviderKind) -> &dyn SecretStore {
        match kind {
            PairProviderKind::QwenVision => self.qwen.as_ref(),
            PairProviderKind::Speech => self.speech.as_ref(),
        }
    }

    fn read(
        store: &Store,
        kind: PairProviderKind,
    ) -> companion_sync::Result<Option<serde_json::Value>> {
        match kind {
            PairProviderKind::QwenVision => store.read_video_device(VideoDeviceSlot::Provider),
            PairProviderKind::Speech => store.read_cloud_voice(None),
        }
        .map_err(Error::from)
    }

    fn validate(kind: PairProviderKind, value: &serde_json::Value) -> companion_sync::Result<()> {
        match kind {
            PairProviderKind::QwenVision => {
                serde_json::from_value::<companion_chat::video_cloud::QwenSettings>(value.clone())
                    .map_err(|_| Error::Invalid)?
                    .validate()
                    .map_err(|_| Error::Invalid)
            }
            PairProviderKind::Speech => {
                serde_json::from_value::<companion_chat::voice_cloud::SpeechSettings>(value.clone())
                    .map_err(|_| Error::Invalid)?
                    .validate()
                    .map_err(|_| Error::Invalid)
            }
        }
    }
}

impl ProviderHandoff for PairedProviders {
    fn snapshot(&self) -> companion_sync::Result<Vec<PairProvider>> {
        let store = self.store.lock().map_err(|_| Error::Storage)?;
        let mut providers = Vec::new();
        for kind in [PairProviderKind::QwenVision, PairProviderKind::Speech] {
            let Some(settings) = Self::read(&store, kind)? else {
                continue;
            };
            Self::validate(kind, &settings)?;
            match self.vault(kind).get() {
                Ok(key) => providers.push(PairProvider::new(kind, settings, key)),
                Err(VaultError::Missing) => {}
                Err(_) => return Err(Error::Vault),
            }
        }
        Ok(providers)
    }

    fn receive(&self, provider: PairProvider) -> companion_sync::Result<bool> {
        Self::validate(provider.kind, &provider.settings)?;
        let store = self.store.lock().map_err(|_| Error::Storage)?;
        let local = Self::read(&store, provider.kind)?;
        let vault = self.vault(provider.kind);
        match vault.get() {
            Ok(key) => {
                return Ok(local.as_ref() != Some(&provider.settings)
                    || key.expose() != provider.secret().expose());
            }
            Err(VaultError::Missing) => {}
            Err(_) => return Err(Error::Vault),
        }
        if local.is_some_and(|value| value != provider.settings) {
            return Ok(true);
        }
        vault.set(provider.secret()).map_err(|_| Error::Vault)?;
        let verified = vault
            .get()
            .is_ok_and(|value| value.expose() == provider.secret().expose());
        if !verified {
            let _ = vault.delete();
            return Err(Error::Vault);
        }
        let stored = match provider.kind {
            PairProviderKind::QwenVision => {
                store.write_video_device(VideoDeviceSlot::Provider, Some(provider.settings))
            }
            PairProviderKind::Speech => store.write_cloud_voice_provider(provider.settings),
        };
        if let Err(error) = stored {
            let _ = vault.delete();
            return Err(error.into());
        }
        Ok(false)
    }
}

pub struct SyncHost {
    pub suspended: AtomicBool,
    pub store: Arc<Mutex<Store>>,
    pub credentials: Arc<dyn SecretStore>,
    pub api_credentials: Arc<dyn SecretStore>,
    pub providers: Arc<PairedProviders>,
    pub discovery: Arc<dyn Discovery>,
    pub service: Mutex<Option<Arc<SyncService>>>,
}
impl SyncHost {
    fn ensure(&self) -> Result<Arc<SyncService>, String> {
        let mut service = self.service.lock().map_err(|_| "STORAGE_ERROR")?;
        if let Some(service) = &*service {
            return Ok(service.clone());
        }
        let name = if cfg!(target_os = "android") {
            "Android 手机"
        } else {
            "Windows 电脑"
        };
        let started = SyncService::start(
            self.store.clone(),
            self.credentials.clone(),
            name,
            self.discovery.clone(),
        )
        .map_err(sync_error)?;
        started.configure_platform(cfg!(target_os = "windows"), cfg!(target_os = "android"));
        started
            .configure_api_vault(self.api_credentials.clone())
            .map_err(sync_error)?;
        started
            .configure_provider_handoff(self.providers.clone())
            .map_err(sync_error)?;
        *service = Some(started.clone());
        Ok(started)
    }
}
pub fn discovery(app: &tauri::AppHandle) -> Arc<dyn Discovery> {
    #[cfg(target_os = "android")]
    {
        Arc::new(tauri_plugin_companion_keystore::AndroidDiscovery(
            app.state::<tauri_plugin_companion_keystore::KeystoreHandle<tauri::Wry>>()
                .inner()
                .clone(),
        ))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Arc::new(companion_sync::discovery::Mdns::default())
    }
}
#[tauri::command]
pub async fn sync_status(host: State<'_, Arc<SyncHost>>) -> Result<serde_json::Value, String> {
    let host = host.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let enabled=host.store.lock().map_err(|_|"STORAGE_ERROR")?.lan_enabled().map_err(sync_error)?;
        if host.suspended.load(Ordering::Acquire) || !enabled {
            let store=host.store.lock().map_err(|_|"STORAGE_ERROR")?;
            return Ok(serde_json::json!({"enabled":false,"device_id":store.device_id,"fingerprint":"","name":std::env::consts::OS,"port":0,"candidates":[],"peers":[],"pending":[],"conflicts":[],"discovery_error":null}));
        }
        let status=host.ensure()?.status().map_err(sync_error)?;
        serde_json::to_value(status).map_err(|_|"SYNC_STATUS_ERROR".into())
    }).await.map_err(|_|"SYNC_TASK_ERROR".to_string())?
}
#[tauri::command]
pub async fn sync_command(
    action: String,
    id: Option<String>,
    text: Option<String>,
    address: Option<String>,
    host: State<'_, Arc<SyncHost>>,
) -> Result<serde_json::Value, String> {
    let host = host.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if action == "disable" {
            host.suspended.store(true, Ordering::Release);
            if let Some(service) = host.service.lock().map_err(|_| "STORAGE_ERROR")?.take() {
                service.stop();
            }
            host.store
                .lock()
                .map_err(|_| "STORAGE_ERROR")?
                .lan_set_enabled(false)
                .map_err(sync_error)?;
            return Ok(serde_json::Value::Null);
        }
        let enabled = host
            .store
            .lock()
            .map_err(|_| "STORAGE_ERROR")?
            .lan_enabled()
            .map_err(sync_error)?;
        let explicit_start = matches!(
            action.as_str(),
            "enable" | "invite_quick" | "pair_scanned" | "invite"
        );
        if !explicit_start && (host.suspended.load(Ordering::Acquire) || !enabled) {
            return Err("SYNC_DISABLED".into());
        }
        let service = host.ensure()?;
        if explicit_start {
            let result = host
                .store
                .lock()
                .map_err(|_| "STORAGE_ERROR")?
                .lan_set_enabled(true);
            if let Err(e) = result {
                service.stop();
                host.suspended.store(true, Ordering::Release);
                *host.service.lock().map_err(|_| "STORAGE_ERROR")? = None;
                return Err(e.to_string());
            }
            host.suspended.store(false, Ordering::Release);
        }
        match action.as_str() {
            "enable" => {}
            "invite_quick" => {
                return serde_json::to_value(service.invite_quick().map_err(sync_error)?)
                    .map_err(|_| "PAIR_ERROR".into());
            }
            "pair_scanned" => service
                .pair_scanned(text.as_deref().ok_or("PAIR_CODE_REQUIRED")?)
                .map_err(sync_error)?,
            "invite" => {
                return serde_json::to_value(service.invite().map_err(sync_error)?)
                    .map_err(|_| "PAIR_ERROR".into());
            }
            "cancel" => service.cancel_pair(),
            "pair" => service
                .request_pair(
                    text.as_deref().ok_or("PAIR_CODE_REQUIRED")?,
                    address.as_deref().ok_or("LAN_ADDRESS_REQUIRED")?,
                )
                .map_err(sync_error)?,
            "confirm" => service
                .confirm(id.as_deref().ok_or("DEVICE_REQUIRED")?)
                .map_err(sync_error)?,
            "revoke" => service
                .revoke(id.as_deref().ok_or("DEVICE_REQUIRED")?)
                .map_err(sync_error)?,
            "reconnect" => service
                .reconnect(
                    id.as_deref().ok_or("DEVICE_REQUIRED")?,
                    address.as_deref().ok_or("LAN_ADDRESS_REQUIRED")?,
                )
                .map_err(sync_error)?,
            "sync" => service.wake(),
            _ => return Err("UNKNOWN_SYNC_COMMAND".into()),
        }
        Ok(serde_json::Value::Null)
    })
    .await
    .map_err(|_| "SYNC_TASK_ERROR".to_string())?
}
