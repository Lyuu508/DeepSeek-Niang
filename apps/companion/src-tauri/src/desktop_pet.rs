//! Windows desktop pet. Shares the application's selected persona and ChatService.
mod gait;
mod geometry;
mod roaming;
mod startup;

use crate::AppState;
use companion_chat::{
    ChatFailure,
    service::{ChatEvent, Observer},
};
use companion_core::{
    chat::{StartChat, now_ms},
    companion::{
        excerpt,
        pet::{PetPlacement, PetPreferences, PetSnapshot},
    },
};
use roaming::{Motion, Roamer};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
    ipc::Channel,
    menu::MenuBuilder,
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    webview::Color,
};
use uuid::Uuid;

const PET: &str = "pet";
const MAIN: &str = "main";
const TRAY: &str = "desktop-pet";
const OPEN_CHAT: &str = "desktop-pet:open-chat";
const SETTINGS: &str = "desktop-pet:settings";
const CHAT_EVENT: &str = "desktop-pet:chat";

#[derive(Default)]
struct DesktopPet {
    operations: Mutex<()>,
    expanded: AtomicBool,
    controls: AtomicBool,
    region_size: AtomicU64,
    last_position_check: Mutex<Option<Instant>>,
    reply: Mutex<Option<QuickReply>>,
    roamer: Mutex<Option<Arc<Roamer>>>,
}

impl DesktopPet {
    fn roamer(&self) -> Option<Arc<Roamer>> {
        self.roamer.lock().ok()?.clone()
    }
}

#[derive(Serialize)]
pub struct ViewSettings {
    #[serde(flatten)]
    preferences: PetPreferences,
    tray_available: bool,
    startup_enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Setting {
    Visible(bool),
    AlwaysOnTop(bool),
    ScalePercent(u16),
    Roam(bool),
    ReduceMotion(bool),
    Startup(bool),
    ResetPosition,
    PetOnly,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Action {
    Drag,
    Hover(bool),
    Interact,
    SystemReducedMotion(bool),
    Panel(bool),
    OpenChat,
    Hide,
    ToggleTop,
    ToggleRoam,
    ScaleStep(i8),
    ResetPosition,
}

#[derive(Clone, Serialize)]
struct QuickReply {
    persona_id: String,
    request_id: String,
    phase: &'static str,
    preview: String,
    error: Option<ChatFailure>,
}

#[derive(Serialize)]
struct Appearance {
    always_on_top: bool,
    scale_percent: u16,
    roam: bool,
    reduce_motion: bool,
}

#[derive(Serialize)]
pub struct Status {
    #[serde(flatten)]
    pet: PetSnapshot,
    settings: Appearance,
    reply: Option<QuickReply>,
}

type Outcome<T> = Result<T, ChatFailure>;

fn window_error(_: tauri::Error) -> ChatFailure {
    ChatFailure::new("PET_WINDOW_ERROR")
}
fn storage_error() -> ChatFailure {
    ChatFailure::new("STORAGE_ERROR")
}
fn require_window(window: &WebviewWindow, label: &str) -> Outcome<()> {
    if window.label() == label {
        Ok(())
    } else {
        Err(ChatFailure::new("INVALID_INPUT"))
    }
}

fn save(app: &AppHandle, prefs: &PetPreferences) -> Outcome<()> {
    app.state::<AppState>()
        .0
        .store
        .lock()
        .map_err(|_| storage_error())?
        .save_pet_preferences(prefs)?;
    Ok(())
}

fn settings(app: &AppHandle) -> Outcome<ViewSettings> {
    let mut preferences = app
        .state::<AppState>()
        .0
        .store
        .lock()
        .map_err(|_| storage_error())?
        .pet_preferences()?;
    // A failed display attempt must not report a window that is not present.
    preferences.visible = app
        .get_webview_window(PET)
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    Ok(ViewSettings {
        preferences,
        tray_available: app.tray_by_id(TRAY).is_some(),
        startup_enabled: startup::enabled(&app.config().identifier).ok(),
    })
}

fn work_monitor(app: &AppHandle, placement: &PetPlacement) -> Outcome<Monitor> {
    let monitors = app.available_monitors().map_err(window_error)?;
    monitors
        .into_iter()
        .find(|m| placement.monitor.is_some() && m.name() == placement.monitor.as_ref())
        .or(app.primary_monitor().map_err(window_error)?)
        .ok_or_else(|| ChatFailure::new("PET_WINDOW_ERROR"))
}

fn fit(
    app: &AppHandle,
    window: &WebviewWindow,
    prefs: &PetPreferences,
    expanded: bool,
) -> Outcome<()> {
    let monitor = work_monitor(app, &prefs.placement)?;
    let (point, size, zoom) =
        geometry::layout(&monitor, &prefs.placement, prefs.scale_percent, expanded);
    window.set_position(point).map_err(window_error)?;
    window.set_size(size).map_err(window_error)?;
    window.set_zoom(zoom).map_err(window_error)?;
    queue_region(window, size, expanded)?;
    Ok(())
}

fn queue_region(
    window: &WebviewWindow,
    size: tauri::PhysicalSize<u32>,
    expanded: bool,
) -> Outcome<()> {
    let runtime = window.state::<Arc<DesktopPet>>();
    let controls = runtime.controls.load(Ordering::Relaxed);
    let key = u64::from(size.width) << 32
        | u64::from(size.height) << 2
        | u64::from(expanded) << 1
        | u64::from(controls);
    if runtime.region_size.swap(key, Ordering::Relaxed) == key {
        return Ok(());
    }
    let target = window.clone();
    window
        .run_on_main_thread(move || geometry::apply_region(&target, size, expanded, controls))
        .map_err(window_error)
}

fn remember_position(window: &WebviewWindow, prefs: &mut PetPreferences) -> Outcome<bool> {
    let point = window.outer_position().map_err(window_error)?;
    let size = window.outer_size().map_err(window_error)?;
    let monitor = window
        .available_monitors()
        .map_err(window_error)?
        .into_iter()
        .find(|m| geometry::contains_window_center(m, point, size));
    if let Some(monitor) = monitor {
        prefs.placement = geometry::remember(&monitor, point, size);
        Ok(true)
    } else {
        Ok(false)
    }
}

fn allowed_navigation(app: &AppHandle, url: &tauri::Url) -> bool {
    if url.path() != "/pet.html" {
        return false;
    }
    if url.scheme() == "https" && url.host_str() == Some("tauri.localhost") && url.port().is_none()
    {
        return true;
    }
    cfg!(debug_assertions)
        && app.config().build.dev_url.as_ref().is_some_and(|dev| {
            dev.origin() == url.origin() && url.username().is_empty() && url.password().is_none()
        })
}

fn ensure_window(app: &AppHandle, prefs: &PetPreferences) -> Outcome<WebviewWindow> {
    if let Some(window) = app.get_webview_window(PET) {
        return Ok(window);
    }
    let runtime = app.state::<Arc<DesktopPet>>();
    runtime.expanded.store(false, Ordering::Relaxed);
    runtime.controls.store(false, Ordering::Relaxed);
    runtime.region_size.store(0, Ordering::Relaxed);
    let roamer = Arc::new(Roamer::new(prefs.roam && !prefs.reduce_motion));
    // The renderer can subscribe as soon as its WebView starts loading.
    *runtime.roamer.lock().map_err(|_| storage_error())? = Some(Arc::clone(&roamer));
    let handle = app.clone();
    let window = WebviewWindowBuilder::new(app, PET, WebviewUrl::App("pet.html".into()))
        .title("DeepSeek娘桌宠")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .background_color(Color(0, 0, 0, 0))
        .use_https_scheme(true)
        .skip_taskbar(true)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .focused(false)
        .visible(false)
        .inner_size(geometry::WIDTH, geometry::COMPACT_HEIGHT)
        .always_on_top(prefs.always_on_top)
        .on_navigation(move |url| allowed_navigation(&handle, url))
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .build()
        .map_err(window_error)?;
    let listener = window.clone();
    let lifetime = Arc::clone(&roamer);
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(size) => {
            let expanded = listener
                .state::<Arc<DesktopPet>>()
                .expanded
                .load(Ordering::Relaxed);
            let _ = queue_region(&listener, *size, expanded);
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let app = listener.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = configure(&app, Setting::Visible(false));
            });
        }
        WindowEvent::Destroyed => lifetime.stop(),
        _ => {}
    });
    if let Err(error) = fit(app, &window, prefs, false) {
        let _ = window.destroy();
        return Err(error);
    }
    if roamer.start(window.clone()).is_err() {
        let _ = window.destroy();
        return Err(ChatFailure::new("PET_WINDOW_ERROR"));
    }
    Ok(window)
}

fn open_chat(app: &AppHandle) -> Outcome<()> {
    let window = app
        .get_webview_window(MAIN)
        .ok_or_else(|| ChatFailure::new("PET_WINDOW_ERROR"))?;
    window.show().map_err(window_error)?;
    window.unminimize().map_err(window_error)?;
    app.emit_to(MAIN, OPEN_CHAT, ()).map_err(window_error)?;
    window.set_focus().map_err(window_error)
}

fn read_preferences(app: &AppHandle) -> Outcome<PetPreferences> {
    Ok(app
        .state::<AppState>()
        .0
        .store
        .lock()
        .map_err(|_| storage_error())?
        .pet_preferences()?)
}

fn configure(app: &AppHandle, setting: Setting) -> Outcome<ViewSettings> {
    let runtime = app.state::<Arc<DesktopPet>>();
    let _operation = runtime
        .operations
        .lock()
        .map_err(|_| window_error(tauri::Error::FailedToReceiveMessage))?;
    let mut prefs = read_preferences(app)?;
    let mut hide_main = false;
    match setting {
        Setting::Startup(enabled) => {
            startup::set(&app.config().identifier, enabled)
                .map_err(|_| ChatFailure::new("PET_STARTUP_ERROR"))?;
            return settings(app);
        }
        Setting::Visible(visible) => prefs.visible = visible,
        Setting::AlwaysOnTop(top) => prefs.always_on_top = top,
        Setting::ScalePercent(percent) => prefs.scale_percent = percent,
        Setting::Roam(roam) => prefs.roam = roam,
        Setting::ReduceMotion(reduced) => prefs.reduce_motion = reduced,
        Setting::ResetPosition => prefs.placement = PetPlacement::default(),
        Setting::PetOnly => {
            if app.tray_by_id(TRAY).is_none() {
                return Err(ChatFailure::new("PET_WINDOW_ERROR"));
            }
            prefs.visible = true;
            hide_main = true;
        }
    }
    prefs.validate()?;
    if let Some(window) = app.get_webview_window(PET) {
        if !matches!(setting, Setting::ResetPosition) {
            remember_position(&window, &mut prefs)?;
        }
        if prefs.visible {
            if let Some(roamer) = runtime.roamer() {
                roamer.set_allowed(prefs.roam && !prefs.reduce_motion);
                if matches!(setting, Setting::ScalePercent(_) | Setting::ResetPosition) {
                    roamer.pause(Duration::ZERO);
                }
            }
            window
                .set_always_on_top(prefs.always_on_top)
                .map_err(window_error)?;
            if matches!(setting, Setting::ScalePercent(_) | Setting::ResetPosition) {
                fit(
                    app,
                    &window,
                    &prefs,
                    runtime.expanded.load(Ordering::Relaxed),
                )?;
            }
            window.show().map_err(window_error)?;
        } else {
            // If the pet was the only visible window, keep a way back even when the tray is unavailable.
            if app.tray_by_id(TRAY).is_none() {
                open_chat(app)?;
            }
            window.destroy().map_err(window_error)?;
        }
    } else if prefs.visible {
        ensure_window(app, &prefs)?.show().map_err(window_error)?;
    }
    save(app, &prefs)?;
    if hide_main && let Some(main) = app.get_webview_window(MAIN) {
        main.hide().map_err(window_error)?;
    }
    let _ = app.emit_to(MAIN, SETTINGS, ());
    settings(app)
}

pub fn install(app: &AppHandle) {
    app.manage(Arc::new(DesktopPet::default()));
    let tray = (|| -> tauri::Result<()> {
        let menu = MenuBuilder::new(app)
            .text("pet-chat", "打开聊天")
            .text("pet-toggle", "显示 / 隐藏桌宠")
            .text("pet-recover", "找回桌宠位置")
            .separator()
            .text("pet-exit", "退出 DeepSeek娘")
            .build()?;
        let mut builder = TrayIconBuilder::with_id(TRAY)
            .menu(&menu)
            .tooltip("DeepSeek娘")
            .show_menu_on_left_click(false)
            .on_menu_event(|app, event| {
                let action = event.id().as_ref();
                if action == "pet-exit" {
                    app.exit(0);
                    return;
                }
                let selected = action.to_owned();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let result = match selected.as_str() {
                        "pet-chat" => open_chat(&app),
                        "pet-toggle" => configure(
                            &app,
                            Setting::Visible(app.get_webview_window(PET).is_none()),
                        )
                        .map(|_| ()),
                        "pet-recover" => configure(&app, Setting::ResetPosition)
                            .and_then(|_| configure(&app, Setting::Visible(true)))
                            .map(|_| ()),
                        _ => Ok(()),
                    };
                    if result.is_err() {
                        let _ = open_chat(&app);
                    }
                });
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    let _ = open_chat(tray.app_handle());
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;
        Ok(())
    })();
    if tray.is_err() {
        eprintln!("PET_TRAY_UNAVAILABLE");
    }
    if let Ok(prefs) = read_preferences(app)
        && prefs.visible
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = configure(&app, Setting::Visible(true));
        });
    }
}

#[tauri::command]
pub async fn desktop_pet_settings(window: WebviewWindow) -> Outcome<ViewSettings> {
    require_window(&window, MAIN)?;
    settings(window.app_handle())
}

#[tauri::command]
pub async fn desktop_pet_configure(
    window: WebviewWindow,
    setting: Setting,
) -> Outcome<ViewSettings> {
    require_window(&window, MAIN)?;
    configure(window.app_handle(), setting)
}

#[tauri::command]
pub async fn pet_snapshot(window: WebviewWindow) -> Outcome<Status> {
    require_window(&window, PET)?;
    let app = window.app_handle();
    let runtime = app.state::<Arc<DesktopPet>>();
    // Save while at rest or after a manual move, without writing for every walking step.
    if let Ok(_operation) = runtime.operations.try_lock()
        && runtime.roamer().is_none_or(|roamer| !roamer.walking())
        && let Ok(mut checked) = runtime.last_position_check.lock()
        && checked.is_none_or(|at| at.elapsed() >= Duration::from_secs(2))
    {
        *checked = Some(Instant::now());
        let mut prefs = read_preferences(app)?;
        let previous = prefs.placement.clone();
        if !remember_position(&window, &mut prefs)? {
            fit(
                app,
                &window,
                &prefs,
                runtime.expanded.load(Ordering::Relaxed),
            )?;
        } else if previous != prefs.placement {
            save(app, &prefs)?;
        }
    }
    let service = &app.state::<AppState>().0;
    let generation = service.active_generation();
    let pet = service
        .store
        .lock()
        .map_err(|_| storage_error())?
        .pet_snapshot(
            now_ms(),
            generation
                .as_ref()
                .map(|(request, conversation)| (request.as_str(), conversation.as_str())),
        )?;
    let reply = runtime
        .reply
        .lock()
        .map_err(|_| storage_error())?
        .as_ref()
        .filter(|v| v.persona_id == pet.persona_id)
        .cloned();
    let prefs = read_preferences(app)?;
    Ok(Status {
        pet,
        settings: Appearance {
            always_on_top: prefs.always_on_top,
            scale_percent: prefs.scale_percent,
            roam: prefs.roam,
            reduce_motion: prefs.reduce_motion,
        },
        reply,
    })
}

#[tauri::command]
pub async fn pet_motion(window: WebviewWindow, channel: Channel<Motion>) -> Outcome<()> {
    require_window(&window, PET)?;
    window
        .state::<Arc<DesktopPet>>()
        .roamer()
        .ok_or_else(|| ChatFailure::new("PET_WINDOW_ERROR"))?
        .subscribe(channel)
        .map_err(window_error)
}

#[tauri::command]
pub async fn pet_action(window: WebviewWindow, action: Action) -> Outcome<()> {
    require_window(&window, PET)?;
    let app = window.app_handle();
    let runtime = app.state::<Arc<DesktopPet>>();
    let roamer = runtime.roamer();
    let pause = |duration| {
        if let Some(roamer) = &roamer {
            roamer.pause(duration);
        }
    };
    match action {
        Action::Drag => {
            pause(Duration::from_secs(1));
            window.start_dragging().map_err(window_error)
        }
        Action::Hover(hover) => {
            let _operation = runtime.operations.lock().map_err(|_| storage_error())?;
            runtime.controls.store(hover, Ordering::Relaxed);
            if hover {
                pause(Duration::ZERO);
            }
            queue_region(
                &window,
                window.outer_size().map_err(window_error)?,
                runtime.expanded.load(Ordering::Relaxed),
            )
        }
        Action::Interact => {
            pause(Duration::from_millis(5000));
            Ok(())
        }
        Action::SystemReducedMotion(reduced) => {
            if let Some(roamer) = &roamer {
                roamer.set_system_reduced(reduced);
            }
            Ok(())
        }
        Action::OpenChat => {
            pause(Duration::from_secs(2));
            open_chat(app)
        }
        Action::Hide => configure(app, Setting::Visible(false)).map(|_| ()),
        Action::ResetPosition => configure(app, Setting::ResetPosition).map(|_| ()),
        Action::ToggleTop => configure(
            app,
            Setting::AlwaysOnTop(!read_preferences(app)?.always_on_top),
        )
        .map(|_| ()),
        Action::ToggleRoam => {
            configure(app, Setting::Roam(!read_preferences(app)?.roam)).map(|_| ())
        }
        Action::ScaleStep(step @ (-1 | 1)) => {
            let scale = i32::from(read_preferences(app)?.scale_percent) + 10 * i32::from(step);
            configure(app, Setting::ScalePercent(scale.clamp(75, 150) as u16)).map(|_| ())
        }
        Action::ScaleStep(_) => Err(ChatFailure::new("INVALID_INPUT")),
        Action::Panel(open) => {
            let _operation = runtime.operations.lock().map_err(|_| storage_error())?;
            if runtime.expanded.load(Ordering::Relaxed) == open {
                return Ok(());
            }
            pause(Duration::ZERO);
            let mut prefs = read_preferences(app)?;
            remember_position(&window, &mut prefs)?;
            let previous = runtime.expanded.swap(open, Ordering::Relaxed);
            if let Err(error) = fit(app, &window, &prefs, open) {
                runtime.expanded.store(previous, Ordering::Relaxed);
                return Err(error);
            }
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn pet_send(
    window: WebviewWindow,
    persona_id: String,
    request_id: String,
    text: String,
) -> Outcome<()> {
    require_window(&window, PET)?;
    let text = text.trim();
    if text.is_empty()
        || text.chars().count() > 500
        || text.contains('\0')
        || Uuid::parse_str(&request_id).is_err()
    {
        return Err(ChatFailure::new("INVALID_INPUT"));
    }
    let app = window.app_handle();
    let service = Arc::clone(&app.state::<AppState>().0);
    if !service.vault.has()? {
        return Err(ChatFailure::new("KEY_NOT_CONFIGURED"));
    }
    let thread = {
        let store = service.store.lock().map_err(|_| storage_error())?;
        if store.active_persona_id()? != persona_id {
            return Err(ChatFailure::new("PET_PERSONA_CHANGED"));
        }
        store
            .personas()?
            .into_iter()
            .find(|p| p.id == persona_id)
            .map(|p| p.main_conversation_id)
            .ok_or_else(|| ChatFailure::new("PET_PERSONA_CHANGED"))?
    };
    let runtime = Arc::clone(app.state::<Arc<DesktopPet>>().inner());
    let main = app.clone();
    let owner = persona_id.clone();
    let id = request_id.clone();
    let observer: Observer = Arc::new(move |event| {
        if let Ok(mut state) = runtime.reply.lock() {
            *state = Some(quick_reply(&owner, &id, &event));
        }
        let _ = main.emit_to(MAIN, CHAT_EVENT, event);
        // The run belongs to ChatService, including if the user hides the pet while it is running.
        true
    });
    service.start_for_active_persona(
        StartChat {
            request_id,
            conversation_id: thread,
            text: Some(text.into()),
            regenerate_user_id: None,
        },
        &persona_id,
        observer,
    )?;
    Ok(())
}

fn quick_reply(persona_id: &str, request_id: &str, event: &ChatEvent) -> QuickReply {
    let (phase, text, error) = match event {
        ChatEvent::Started { .. } => ("sending", "", None),
        ChatEvent::Delta { content, .. } => ("streaming", content.as_str(), None),
        ChatEvent::Finished { message, error, .. } => {
            let phase = match message.state.as_str() {
                "complete" => "complete",
                "interrupted" => "interrupted",
                _ => "failed",
            };
            (
                phase,
                message.content.as_str(),
                error.clone().filter(|e| e.code != "CANCELLED"),
            )
        }
        ChatEvent::Failed { error, .. } => ("failed", "", Some(error.clone())),
    };
    QuickReply {
        persona_id: persona_id.into(),
        request_id: request_id.into(),
        phase,
        preview: excerpt(text, 160),
        error,
    }
}

#[tauri::command]
pub async fn pet_stop(window: WebviewWindow, request_id: String) -> Outcome<()> {
    require_window(&window, PET)?;
    let runtime = window.state::<Arc<DesktopPet>>();
    let service = Arc::clone(&window.state::<AppState>().0);
    let persona = service
        .store
        .lock()
        .map_err(|_| storage_error())?
        .active_persona_id()?;
    let authorized = runtime
        .reply
        .lock()
        .map_err(|_| storage_error())?
        .as_ref()
        .is_some_and(|reply| {
            reply.persona_id == persona
                && reply.request_id == request_id
                && matches!(reply.phase, "sending" | "streaming")
        });
    if !authorized {
        return Err(ChatFailure::new("INVALID_INPUT"));
    }
    service.stop_and_wait(&request_id).await?;
    Ok(())
}
