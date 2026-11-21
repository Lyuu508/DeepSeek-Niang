#[cfg(target_os = "windows")]
pub(crate) mod windows;

use companion_update::{Download, Latest, Platform, UpdateView, Updater};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State, WebviewWindow};
use tokio::sync::Mutex;

type Reply<T> = Result<T, &'static str>;
const CURRENT: &str = env!("CARGO_PKG_VERSION");
const PLATFORM: Platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Platform::WindowsX64
} else if cfg!(all(target_os = "android", target_arch = "aarch64")) {
    Platform::AndroidArm64
} else {
    Platform::Unavailable
};

#[derive(Default)]
pub struct UpdateHost(Mutex<Session>);
#[derive(Default)]
struct Session {
    latest: Option<Latest>,
    ready: Option<Download>,
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("companion-install-update")
        .setup(|app, _api| {
            app.manage(UpdateHost::default());
            #[cfg(target_os = "android")]
            app.manage(AndroidUpdate(_api.register_android_plugin(
                "io.local.deepseekcompanion",
                "CompanionUpdatePlugin",
            )?));
            Ok(())
        })
        .build()
}

#[cfg(target_os = "android")]
struct AndroidUpdate<R: Runtime>(tauri::plugin::PluginHandle<R>);

fn require_main(window: &WebviewWindow) -> Reply<()> {
    (window.label() == "main")
        .then_some(())
        .ok_or("UPDATE_WINDOW")
}

#[derive(Serialize)]
pub struct Checked {
    current_version: &'static str,
    latest: Option<UpdateView>,
    ready: bool,
    previous_install_failed: bool,
}

#[tauri::command]
pub async fn update_check(window: WebviewWindow, host: State<'_, UpdateHost>) -> Reply<Checked> {
    require_main(&window)?;
    let mut session = host.0.try_lock().map_err(|_| "UPDATE_BUSY")?;
    let latest = Updater::new()?.check(CURRENT, PLATFORM).await?;
    if !session.ready.as_ref().is_some_and(|value| {
        latest.as_ref().is_some_and(|release| {
            release.view.can_download && release.view.version == value.version
        })
    }) {
        session.ready = None;
    }
    session.latest = latest;
    let previous_install_failed = window
        .app_handle()
        .path()
        .app_cache_dir()
        .ok()
        .is_some_and(|path| path.join("companion-updates/install-failed").is_file());
    Ok(Checked {
        current_version: CURRENT,
        latest: session.latest.as_ref().map(|release| release.view.clone()),
        ready: session.ready.is_some(),
        previous_install_failed,
    })
}

#[tauri::command]
pub async fn update_download(
    window: WebviewWindow,
    host: State<'_, UpdateHost>,
    version: String,
    confirmed: bool,
    on_progress: tauri::ipc::Channel<u8>,
) -> Reply<()> {
    require_main(&window)?;
    if !confirmed {
        return Err("UPDATE_CONFIRM_REQUIRED");
    }
    let mut session = host.0.try_lock().map_err(|_| "UPDATE_BUSY")?;
    let latest = session
        .latest
        .as_ref()
        .filter(|value| value.view.version == version)
        .ok_or("UPDATE_CHECK_REQUIRED")?;
    let folder = window
        .app_handle()
        .path()
        .app_cache_dir()
        .map_err(|_| "UPDATE_STORAGE_FAILED")?
        .join("companion-updates");
    let download = Updater::new()?
        .download(latest, &folder, |percent| {
            let _ = on_progress.send(percent);
        })
        .await?;
    verify(window.app_handle(), &download).await?;
    session.ready = Some(download);
    Ok(())
}

async fn verify(_app: &AppHandle, download: &Download) -> Reply<()> {
    let path = download.path.clone();
    let sha = download.sha256.clone();
    tauri::async_runtime::spawn_blocking(move || -> Reply<()> {
        companion_update::verify_file(&path, &sha)?;
        #[cfg(target_os = "windows")]
        windows::verify(&path)?;
        Ok(())
    })
    .await
    .map_err(|_| "UPDATE_SIGNATURE_INVALID")??;
    #[cfg(target_os = "android")]
    android_action(_app, download, "verifyUpdate")?;
    Ok(())
}

#[cfg(target_os = "android")]
fn android_action(app: &AppHandle, download: &Download, action: &str) -> Reply<&'static str> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    struct Outcome {
        status: String,
    }
    let result: Outcome = app.state::<AndroidUpdate<tauri::Wry>>().0.run_mobile_plugin(
        action, serde_json::json!({"path":download.path,"version":download.version,"sha256":download.sha256}),
    ).map_err(|_| "UPDATE_INSTALLER_FAILED")?;
    match result.status.as_str() {
        "verified" => Ok("verified"),
        "permission_required" => Ok("permission_required"),
        "installer_open" => Ok("installer_open"),
        "UPDATE_ANDROID_IDENTITY" => Err("UPDATE_ANDROID_IDENTITY"),
        "UPDATE_CHECKSUM_INVALID" => Err("UPDATE_CHECKSUM_INVALID"),
        _ => Err("UPDATE_INSTALLER_FAILED"),
    }
}

#[tauri::command]
pub async fn update_install(
    window: WebviewWindow,
    host: State<'_, UpdateHost>,
    app_state: State<'_, super::AppState>,
    version: String,
    confirmed: bool,
) -> Reply<&'static str> {
    require_main(&window)?;
    if !confirmed {
        return Err("UPDATE_CONFIRM_REQUIRED");
    }
    if app_state.0.active_request().is_some() {
        return Err("UPDATE_CHAT_BUSY");
    }
    let mut session = host.0.try_lock().map_err(|_| "UPDATE_BUSY")?;
    if !session
        .ready
        .as_ref()
        .is_some_and(|value| value.version == version)
    {
        return Err("UPDATE_CHECK_REQUIRED");
    }
    let download = session.ready.take().ok_or("UPDATE_CHECK_REQUIRED")?;
    let result = async {
        verify(window.app_handle(), &download).await?;
        let directory = window
            .app_handle()
            .path()
            .app_data_dir()
            .map_err(|_| "UPDATE_BACKUP_FAILED")?
            .join("update-backups");
        let store = app_state.0.store.clone();
        tokio::task::spawn_blocking(move || {
            store
                .lock()
                .map_err(|_| "UPDATE_BACKUP_FAILED")?
                .backup_for_upgrade(&directory)
                .map_err(|_| "UPDATE_BACKUP_FAILED")
        })
        .await
        .map_err(|_| "UPDATE_BACKUP_FAILED")??;
        if app_state.0.active_request().is_some() {
            return Err("UPDATE_CHAT_BUSY");
        }
        launch(window.app_handle(), &download)
    }
    .await;
    match result {
        Ok("closing_for_install") => {
            download.retain();
            window.app_handle().exit(0);
        }
        _ => session.ready = Some(download),
    }
    result
}

fn launch(app: &AppHandle, download: &Download) -> Reply<&'static str> {
    #[cfg(not(target_os = "android"))]
    let _ = app;
    #[cfg(target_os = "android")]
    {
        android_action(app, download, "installUpdate")
    }
    #[cfg(target_os = "windows")]
    {
        windows::start(download)?;
        Ok("closing_for_install")
    }
    #[cfg(not(any(target_os = "android", target_os = "windows")))]
    {
        Err("UPDATE_PLATFORM_UNSUPPORTED")
    }
}
