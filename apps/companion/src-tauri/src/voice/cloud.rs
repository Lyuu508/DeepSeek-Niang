use super::{Reply, Take, active};
use crate::AppState;
use companion_chat::voice_cloud::{self, SpeechBinding, SpeechSettings};
use companion_core::storage::Store;
use companion_secrets::{Secret, SecretStore, VaultError};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use tauri::{State, WebviewWindow};
use tokio::sync::Mutex;

pub struct CloudVoiceHost {
    vault: Arc<dyn SecretStore>,
    busy: Mutex<()>,
}

impl CloudVoiceHost {
    pub fn new(vault: Arc<dyn SecretStore>) -> Self {
        Self {
            vault,
            busy: Mutex::new(()),
        }
    }

    pub async fn synthesize(
        &self,
        state: &AppState,
        window: &WebviewWindow,
        persona: &str,
        text: &str,
        speed: f32,
    ) -> Reply<Option<Take>> {
        let _guard = self.busy.try_lock().map_err(|_| "VOICE_BUSY")?;
        let (settings, binding, key) = {
            let store = active(window, state, persona)?;
            let binding: SpeechBinding = read(&store, Some(persona))?;
            if !binding.enabled {
                return Ok(None);
            }
            (
                read(&store, None)?,
                binding,
                self.vault.get().map_err(key_error)?,
            )
        };
        let bytes = voice_cloud::speak(&settings, &binding, &key, text, speed).await?;
        drop(active(window, state, persona)?);
        Take::from_wav(&bytes).map(Some)
    }
}

fn key_error(error: VaultError) -> &'static str {
    if matches!(error, VaultError::Missing) {
        "VOICE_CLOUD_KEY_REQUIRED"
    } else {
        "VOICE_STORAGE"
    }
}

fn read<T: DeserializeOwned + Default>(store: &Store, persona: Option<&str>) -> Reply<T> {
    store
        .read_cloud_voice(persona)
        .map_err(|_| "VOICE_STORAGE")?
        .map(|value| serde_json::from_value(value).map_err(|_| "VOICE_STORAGE"))
        .transpose()
        .map(|settings| settings.unwrap_or_default())
}

#[derive(Serialize)]
pub struct CloudVoiceView {
    settings: SpeechSettings,
    binding: SpeechBinding,
    configured: bool,
}

fn view(store: &Store, host: &CloudVoiceHost, persona: &str) -> Reply<CloudVoiceView> {
    Ok(CloudVoiceView {
        settings: read(store, None)?,
        binding: read(store, Some(persona))?,
        configured: host.vault.has().map_err(key_error)?,
    })
}

#[tauri::command]
pub async fn cloud_voice_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVoiceHost>,
    persona: String,
) -> Reply<CloudVoiceView> {
    let store = active(&window, &state, &persona)?;
    view(&store, &host, &persona)
}

#[tauri::command]
pub async fn cloud_voice_save(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVoiceHost>,
    persona: String,
    settings: SpeechSettings,
    binding: SpeechBinding,
    key: Option<String>,
    delete_key: bool,
) -> Reply<CloudVoiceView> {
    let _guard = host.busy.try_lock().map_err(|_| "VOICE_BUSY")?;
    settings.validate()?;
    binding.validate()?;
    let store = active(&window, &state, &persona)?;
    let old: SpeechSettings = read(&store, None)?;
    let previous = match host.vault.get() {
        Ok(value) => Some(value),
        Err(VaultError::Missing) => None,
        Err(error) => return Err(key_error(error)),
    };
    let key = key
        .map(Secret::new)
        .transpose()
        .map_err(|_| "VOICE_CLOUD_KEY_FORMAT")?;
    if (key.is_some() && delete_key)
        || (binding.enabled && (delete_key || (key.is_none() && previous.is_none())))
    {
        return Err("VOICE_CLOUD_KEY_REQUIRED");
    }
    if key.is_none()
        && previous.is_some()
        && !delete_key
        && (old.endpoint != settings.endpoint || old.provider != settings.provider)
    {
        return Err("VOICE_CLOUD_REENTER_KEY");
    }
    if let Some(key) = &key {
        host.vault.set(key).map_err(key_error)?;
    } else if delete_key && previous.is_some() {
        host.vault.delete().map_err(key_error)?;
    }
    let write = store.write_cloud_voice_configuration(
        &persona,
        serde_json::to_value(&settings).map_err(|_| "VOICE_STORAGE")?,
        serde_json::to_value(&binding).map_err(|_| "VOICE_STORAGE")?,
    );
    if write.is_err() {
        if key.is_some() || delete_key {
            match previous {
                Some(previous) => host.vault.set(&previous).map_err(key_error)?,
                None => host.vault.delete().map_err(key_error)?,
            }
        }
        return Err("VOICE_STORAGE");
    }
    view(&store, &host, &persona)
}

#[tauri::command]
pub async fn cloud_voice_auto(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, CloudVoiceHost>,
    persona: String,
) -> Reply<bool> {
    let store = active(&window, &state, &persona)?;
    let binding: SpeechBinding = read(&store, Some(&persona))?;
    Ok(!store
        .voice_preferences(&persona)
        .map_err(|_| "VOICE_STORAGE")?
        .muted
        && binding.enabled
        && binding.auto
        && host.vault.has().map_err(key_error)?)
}
