//! Drive the visible pet HWND inside its monitor's work area.
use super::{DesktopPet, gait::Gait, geometry};
use serde::Serialize;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{Manager, PhysicalPosition, WebviewWindow, ipc::Channel};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GUITHREADINFO, GetGUIThreadInfo, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
};

#[derive(Clone, Copy, Serialize)]
pub struct Motion {
    walking: bool,
    facing_left: bool,
}

#[derive(Default)]
struct WalkState {
    gait: Gait,
    last_tick: Option<Instant>,
    hold_until: Option<Instant>,
    listener: Option<Channel<Motion>>,
}

impl WalkState {
    fn motion(&self) -> Motion {
        Motion {
            walking: self.gait.pose.walking,
            facing_left: self.gait.pose.facing_left,
        }
    }

    fn notify(&mut self) {
        if let Some(listener) = &self.listener
            && listener.send(self.motion()).is_err()
        {
            self.listener = None;
        }
    }

    fn rest(&mut self) {
        let moving = self.gait.pose.walking;
        self.gait.rest();
        if moving {
            self.notify();
        }
    }
}

pub struct Roamer {
    alive: AtomicBool,
    allowed: AtomicBool,
    system_reduced: AtomicBool,
    scheduled: AtomicBool,
    state: Mutex<WalkState>,
}

impl Roamer {
    pub fn new(allowed: bool) -> Self {
        Self {
            alive: AtomicBool::new(true),
            allowed: AtomicBool::new(allowed),
            system_reduced: AtomicBool::new(false),
            scheduled: AtomicBool::new(false),
            state: Mutex::new(WalkState::default()),
        }
    }

    fn enabled(&self) -> bool {
        self.allowed.load(Ordering::Relaxed) && !self.system_reduced.load(Ordering::Relaxed)
    }

    pub fn set_allowed(&self, allowed: bool) {
        if self.allowed.swap(allowed, Ordering::Relaxed) != allowed {
            self.pause(Duration::ZERO);
        }
    }

    pub fn set_system_reduced(&self, reduced: bool) {
        if self.system_reduced.swap(reduced, Ordering::Relaxed) != reduced {
            self.pause(Duration::ZERO);
        }
    }

    pub fn pause(&self, duration: Duration) {
        if let Ok(mut state) = self.state.lock() {
            let now = Instant::now();
            let until = now + duration;
            state.hold_until = Some(state.hold_until.map_or(until, |held| held.max(until)));
            state.last_tick = Some(now);
            state.rest();
        }
    }

    pub fn walking(&self) -> bool {
        self.state.lock().is_ok_and(|state| state.gait.pose.walking)
    }

    pub fn subscribe(&self, channel: Channel<Motion>) -> tauri::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| tauri::Error::FailedToReceiveMessage)?;
        channel.send(state.motion())?;
        state.listener = Some(channel);
        Ok(())
    }

    pub fn stop(&self) {
        self.alive.store(false, Ordering::Release);
        if let Ok(mut state) = self.state.lock() {
            state.listener = None;
        }
    }

    pub fn start(self: &Arc<Self>, window: WebviewWindow) -> std::io::Result<()> {
        let session = Arc::clone(self);
        thread::Builder::new()
            .name("desktop-pet-roam".into())
            .spawn(move || {
                while session.alive.load(Ordering::Acquire) {
                    if session.enabled() && !session.scheduled.swap(true, Ordering::Relaxed) {
                        let worker = Arc::clone(&session);
                        let pet = window.clone();
                        if window
                            .run_on_main_thread(move || {
                                if worker.alive.load(Ordering::Acquire) {
                                    let runtime = pet.state::<Arc<DesktopPet>>();
                                    if let Ok(_operation) = runtime.operations.try_lock() {
                                        worker.tick(
                                            &pet,
                                            runtime.expanded.load(Ordering::Relaxed),
                                            runtime.controls.load(Ordering::Relaxed),
                                        );
                                    }
                                }
                                worker.scheduled.store(false, Ordering::Relaxed);
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    // One pending tick at most, even if the UI thread is temporarily busy.
                    thread::sleep(Duration::from_millis(if !session.enabled() {
                        320
                    } else if session.walking() {
                        42
                    } else {
                        90
                    }));
                }
            })?;
        Ok(())
    }

    fn tick(&self, window: &WebviewWindow, expanded: bool, controls: bool) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let now = Instant::now();
        let elapsed = state
            .last_tick
            .replace(now)
            .map_or(Duration::ZERO, |at| now.duration_since(at));
        if !self.enabled()
            || state.listener.is_none()
            || expanded
            || state.hold_until.is_some_and(|until| now < until)
            || !window.is_visible().unwrap_or(false)
        {
            state.rest();
            return;
        }
        let (Ok(hwnd), Ok(point), Ok(size), Ok(Some(monitor))) = (
            window.hwnd(),
            window.outer_position(),
            window.outer_size(),
            window.current_monitor(),
        ) else {
            state.rest();
            return;
        };
        let hovering = window
            .cursor_position()
            .is_ok_and(|cursor| geometry::pointer_over_pet(cursor, point, size, controls));
        // Windows tracks the native move loop while the user drags the character or its handle.
        let mut gui = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: gui is a correctly sized writable struct and hwnd belongs to this window.
        let dragging = unsafe { GetGUIThreadInfo(0, &mut gui) } != 0 && gui.hwndMoveSize == hwnd.0;
        if hovering || dragging {
            state.rest();
            return;
        }
        let (left, right, top, bottom) = geometry::travel_bounds(&monitor, size);
        let previous = state.gait.pose;
        let speed = 52.0 * f64::from(size.width) / geometry::WIDTH;
        let x = state.gait.step(elapsed, point.x, left, right, speed);
        let destination = PhysicalPosition::new(x, point.y.clamp(top, bottom));
        if destination != point {
            // SAFETY: this tick runs on the owning window thread. Movement leaves focus,
            // z-order and dimensions alone.
            let moved = unsafe {
                SetWindowPos(
                    hwnd.0,
                    std::ptr::null_mut(),
                    destination.x,
                    destination.y,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
                )
            } != 0;
            if !moved {
                state.gait.rest();
            }
        }
        if previous != state.gait.pose {
            state.notify();
        }
    }
}
