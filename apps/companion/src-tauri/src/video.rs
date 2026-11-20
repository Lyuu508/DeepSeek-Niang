use companion_core::{storage::Store, video::VideoTask};
use companion_video::{CancellationToken, Failure, VideoRunner, prepare_video};
use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tauri::{State, WebviewWindow};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, OFN_DONTADDTORECENT, OFN_EXPLORER, OFN_FILEMUSTEXIST,
    OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

type Reply<T> = Result<T, &'static str>;

struct Active {
    id: String,
    cancel: CancellationToken,
}

pub struct VideoHost {
    runner: VideoRunner,
    active: Mutex<Option<Active>>,
    speech_model: Mutex<Option<PathBuf>>,
    picking: AtomicBool,
}

impl VideoHost {
    pub fn new(store: Arc<Mutex<Store>>, scratch: PathBuf, clock: Instant) -> Self {
        // The Store's process lock is held before cleaning abandoned per-run scratch.
        if let Ok(items) = std::fs::read_dir(&scratch) {
            for item in items.flatten() {
                if item.file_name().to_string_lossy().starts_with("analysis-")
                    && item
                        .file_type()
                        .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
                {
                    let _ = std::fs::remove_dir_all(item.path());
                }
            }
        }
        Self {
            runner: VideoRunner::new(store, scratch, clock),
            active: Mutex::new(None),
            speech_model: Mutex::new(None),
            picking: AtomicBool::new(false),
        }
    }

    pub fn stop(&self) {
        if let Ok(active) = self.active.lock()
            && let Some(job) = active.as_ref()
        {
            job.cancel.cancel();
        }
    }
}

fn require_main(window: &WebviewWindow) -> Reply<()> {
    if window.label() != "main" {
        return Err("VIDEO_WINDOW");
    }
    Ok(())
}

fn app_error(err: companion_core::Error) -> &'static str {
    Failure::from(err).0
}

struct PickerGuard<'a>(&'a AtomicBool);
impl Drop for PickerGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

async fn choose(window: &WebviewWindow, host: &VideoHost, model: bool) -> Reply<Option<PathBuf>> {
    if host.picking.swap(true, Ordering::Acquire) {
        return Err("VIDEO_PICKER_BUSY");
    }
    let _guard = PickerGuard(&host.picking);
    let parent = window.hwnd().map_err(|_| "VIDEO_WINDOW")?.0 as isize;
    let (send, recv) = std::sync::mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let title = if model {
                "选择本机 Whisper 模型"
            } else {
                "选择有权分析的视频"
            };
            let filter = if model {
                "Whisper GGML 模型\0*.bin\0\0"
            } else {
                "视频文件\0*.mp4;*.mov;*.m4v;*.mkv;*.webm\0\0"
            };
            let title: Vec<_> = title.encode_utf16().chain([0]).collect();
            let filter: Vec<_> = filter.encode_utf16().collect();
            let mut path = vec![0_u16; 32768];
            let mut open = OPENFILENAMEW {
                lStructSize: size_of::<OPENFILENAMEW>() as u32,
                hwndOwner: parent as _,
                lpstrFilter: filter.as_ptr(),
                lpstrFile: path.as_mut_ptr(),
                nMaxFile: path.len() as u32,
                lpstrTitle: title.as_ptr(),
                Flags: OFN_DONTADDTORECENT
                    | OFN_NOCHANGEDIR
                    | OFN_EXPLORER
                    | OFN_FILEMUSTEXIST
                    | OFN_PATHMUSTEXIST,
                ..Default::default()
            };
            // All buffers live for the entire modal call; the picker returns a single local selection.
            let selected = unsafe { GetOpenFileNameW(&mut open) } != 0;
            let output = if selected {
                let end = path.iter().position(|&c| c == 0).unwrap_or(path.len());
                Ok(Some(PathBuf::from(OsString::from_wide(&path[..end]))))
            } else if unsafe { CommDlgExtendedError() } == 0 {
                Ok(None)
            } else {
                Err("VIDEO_PICKER_FAILED")
            };
            let _ = send.send(output);
        })
        .map_err(|_| "VIDEO_WINDOW")?;
    tauri::async_runtime::spawn_blocking(move || recv.recv().map_err(|_| "VIDEO_PICKER_FAILED")?)
        .await
        .map_err(|_| "VIDEO_PICKER_FAILED")?
}

#[tauri::command]
pub async fn video_list(
    window: WebviewWindow,
    host: State<'_, Arc<VideoHost>>,
    persona: String,
) -> Reply<Vec<VideoTask>> {
    require_main(&window)?;
    let store = host
        .runner
        .store
        .lock()
        .map_err(|_| "VIDEO_STORAGE_ERROR")?;
    if store.active_persona_id().map_err(app_error)? != persona {
        return Err("VIDEO_PERSONA_CHANGED");
    }
    store.list_local_videos(&persona).map_err(app_error)
}

#[tauri::command]
pub async fn video_select_whisper(
    window: WebviewWindow,
    host: State<'_, Arc<VideoHost>>,
) -> Reply<Option<String>> {
    require_main(&window)?;
    let Some(path) = choose(&window, &host, true).await? else {
        return Ok(None);
    };
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("bin"))
    {
        return Err("VIDEO_SPEECH_MODEL_REQUIRED");
    }
    let name = path
        .file_name()
        .ok_or("VIDEO_SPEECH_MODEL_REQUIRED")?
        .to_string_lossy()
        .into_owned();
    *host
        .speech_model
        .lock()
        .map_err(|_| "VIDEO_STORAGE_ERROR")? = Some(path);
    Ok(Some(name))
}

#[tauri::command]
pub async fn video_import(
    window: WebviewWindow,
    host: State<'_, Arc<VideoHost>>,
    persona: String,
    model: String,
    authorized: bool,
) -> Reply<Option<String>> {
    require_main(&window)?;
    if !authorized {
        return Err("VIDEO_PERMISSION_REQUIRED");
    }
    if !companion_video::supported_model(&model) {
        return Err("VIDEO_MODEL_INVALID");
    }
    let Some(path) = choose(&window, &host, false).await? else {
        return Ok(None);
    };
    let speech = host
        .speech_model
        .lock()
        .map_err(|_| "VIDEO_STORAGE_ERROR")?
        .clone();
    let (media, models) = prepare_video(path, model, speech)
        .await
        .map_err(|error| error.0)?;
    let id = host
        .runner
        .store
        .lock()
        .map_err(|_| "VIDEO_STORAGE_ERROR")?
        .import_local_video(&persona, &media, &models)
        .map_err(app_error)?;
    Ok(Some(id))
}

#[tauri::command]
pub async fn video_control(
    window: WebviewWindow,
    host: State<'_, Arc<VideoHost>>,
    persona: String,
    id: String,
    action: String,
) -> Reply<()> {
    require_main(&window)?;
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err("VIDEO_TASK_STATE");
    }
    let host = Arc::clone(host.inner());
    let mut active = host.active.lock().map_err(|_| "VIDEO_STORAGE_ERROR")?;
    let mut store = host
        .runner
        .store
        .lock()
        .map_err(|_| "VIDEO_STORAGE_ERROR")?;
    match action.as_str() {
        "start" => {
            if active.is_some() {
                return Err("VIDEO_BUSY");
            }
            let work = store.video_work(&persona, &id).map_err(app_error)?;
            let lease = store
                .start_video(&persona, &id, host.runner.tick())
                .map_err(app_error)?;
            let cancel = CancellationToken::new();
            *active = Some(Active {
                id,
                cancel: cancel.clone(),
            });
            drop(store);
            drop(active);
            tauri::async_runtime::spawn(async move {
                host.runner.run(persona, lease, work, cancel).await;
                if let Ok(mut active) = host.active.lock() {
                    *active = None;
                }
            });
        }
        "pause" | "cancel" => {
            store
                .control_video(&persona, &id, action == "cancel")
                .map_err(app_error)?;
            if let Some(job) = active.as_ref().filter(|job| job.id == id) {
                job.cancel.cancel();
            }
        }
        "save_memory" => store.remember_video(&persona, &id).map_err(app_error)?,
        _ => return Err("VIDEO_TASK_STATE"),
    }
    Ok(())
}
