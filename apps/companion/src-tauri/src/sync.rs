use companion_core::storage::Store;
use companion_secrets::SecretStore;
use companion_sync::{SyncService, discovery::Discovery};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::State;

pub struct SyncHost {
    pub suspended: AtomicBool,
    pub store: Arc<Mutex<Store>>,
    pub credentials: Arc<dyn SecretStore>,
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
        .map_err(|e| e.to_string())?;
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
        let enabled=host.store.lock().map_err(|_|"STORAGE_ERROR")?.lan_enabled().map_err(|e|e.to_string())?;
        if host.suspended.load(Ordering::Acquire) || !enabled {
            let store=host.store.lock().map_err(|_|"STORAGE_ERROR")?;
            return Ok(serde_json::json!({"enabled":false,"device_id":store.device_id,"fingerprint":"","name":std::env::consts::OS,"port":0,"candidates":[],"peers":[],"pending":[],"conflicts":[],"discovery_error":null}));
        }
        let status=host.ensure()?.status().map_err(|e|e.to_string())?;
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
                .map_err(|e| e.to_string())?;
            return Ok(serde_json::Value::Null);
        }
        let enabled = host
            .store
            .lock()
            .map_err(|_| "STORAGE_ERROR")?
            .lan_enabled()
            .map_err(|e| e.to_string())?;
        if action != "enable" && (host.suspended.load(Ordering::Acquire) || !enabled) {
            return Err("SYNC_DISABLED".into());
        }
        let service = host.ensure()?;
        match action.as_str() {
            "enable" => {
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
            "invite" => {
                return serde_json::to_value(service.invite().map_err(|e| e.to_string())?)
                    .map_err(|_| "PAIR_ERROR".into());
            }
            "cancel" => service.cancel_pair(),
            "pair" => service
                .request_pair(
                    text.as_deref().ok_or("PAIR_CODE_REQUIRED")?,
                    address.as_deref().ok_or("LAN_ADDRESS_REQUIRED")?,
                )
                .map_err(|e| e.to_string())?,
            "confirm" => service
                .confirm(id.as_deref().ok_or("DEVICE_REQUIRED")?)
                .map_err(|e| e.to_string())?,
            "revoke" => service
                .revoke(id.as_deref().ok_or("DEVICE_REQUIRED")?)
                .map_err(|e| e.to_string())?,
            "reconnect" => service
                .reconnect(
                    id.as_deref().ok_or("DEVICE_REQUIRED")?,
                    address.as_deref().ok_or("LAN_ADDRESS_REQUIRED")?,
                )
                .map_err(|e| e.to_string())?,
            "sync" => service.wake(),
            _ => return Err("UNKNOWN_SYNC_COMMAND".into()),
        }
        Ok(serde_json::Value::Null)
    })
    .await
    .map_err(|_| "SYNC_TASK_ERROR".to_string())?
}
