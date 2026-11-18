//! Window placement uses the monitor's work area, including on monitors left of the primary.
use companion_core::companion::pet::PetPlacement;
use tauri::{Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRoundRectRgn, DeleteObject, RGN_OR, SetWindowRgn,
};

pub const WIDTH: f64 = 224.0;
pub const COMPACT_HEIGHT: f64 = 190.0;
const PANEL_HEIGHT: i32 = 216;
pub const EXPANDED_HEIGHT: f64 = COMPACT_HEIGHT + PANEL_HEIGHT as f64;
const TRAVEL: i64 = 10_000;

pub fn contains_window_center(
    monitor: &Monitor,
    point: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> bool {
    let area = monitor.work_area();
    let x = i64::from(point.x) + i64::from(size.width) / 2;
    let y = i64::from(point.y) + i64::from(size.height) / 2;
    let left = i64::from(area.position.x);
    let top = i64::from(area.position.y);
    x >= left
        && x < left + i64::from(area.size.width)
        && y >= top
        && y < top + i64::from(area.size.height)
}

fn anchor(origin: i32, position: i32, dimension: u32, window: u32) -> u16 {
    let available = i64::from(dimension.saturating_sub(window));
    if available == 0 {
        return 0;
    }
    (((i64::from(position) - i64::from(origin)).clamp(0, available) * TRAVEL + available / 2)
        / available) as u16
}

pub fn remember(
    monitor: &Monitor,
    point: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> PetPlacement {
    let area = monitor.work_area();
    PetPlacement {
        monitor: monitor.name().map(ToString::to_string),
        horizontal: anchor(area.position.x, point.x, area.size.width, size.width),
        vertical: anchor(area.position.y, point.y, area.size.height, size.height),
    }
}

fn coordinate(origin: i32, dimension: u32, window: u32, fraction: u16) -> i32 {
    let available = i64::from(dimension.saturating_sub(window));
    (i64::from(origin) + available * i64::from(fraction.min(10_000)) / TRAVEL)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

pub fn travel_bounds(monitor: &Monitor, size: PhysicalSize<u32>) -> (i32, i32, i32, i32) {
    let area = monitor.work_area();
    let (x, y) = (area.position.x, area.position.y);
    (
        x,
        coordinate(x, area.size.width, size.width, 10_000),
        y,
        coordinate(y, area.size.height, size.height, 10_000),
    )
}

pub fn layout(
    monitor: &Monitor,
    placement: &PetPlacement,
    scale_percent: u16,
    expanded: bool,
) -> (PhysicalPosition<i32>, PhysicalSize<u32>, f64) {
    let work = monitor.work_area();
    let dpi = monitor.scale_factor();
    let height = if expanded {
        EXPANDED_HEIGHT
    } else {
        COMPACT_HEIGHT
    };
    let zoom = (f64::from(scale_percent) / 100.0)
        .min(f64::from(work.size.width) / dpi / WIDTH)
        .min(f64::from(work.size.height) / dpi / height);
    let size = PhysicalSize::new(
        (WIDTH * dpi * zoom).round().max(1.0) as u32,
        (height * dpi * zoom).round().max(1.0) as u32,
    );
    let position = PhysicalPosition::new(
        coordinate(
            work.position.x,
            work.size.width,
            size.width,
            placement.horizontal,
        ),
        coordinate(
            work.position.y,
            work.size.height,
            size.height,
            placement.vertical,
        ),
    );
    (position, size, zoom)
}

// Keep the idle hit area close to the silhouette. The lower band also covers the whale tail.
const BODY: [(i32, i32, i32, i32, i32); 2] = [(48, 15, 176, 156, 46), (44, 82, 180, 153, 24)];
const BUTTONS: [(i32, i32, i32, i32, i32); 2] = [(23, 3, 201, 33, 17), (20, 146, 204, 185, 24)];

pub fn pointer_over_pet(
    cursor: PhysicalPosition<f64>,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    controls: bool,
) -> bool {
    let scale = f64::from(size.width) / WIDTH;
    let (x, y) = (
        (cursor.x - f64::from(position.x)) / scale,
        (cursor.y - f64::from(position.y)) / scale,
    );
    BODY.iter()
        .chain(
            BUTTONS
                .iter()
                .take(if controls { BUTTONS.len() } else { 0 }),
        )
        .any(|&(l, t, r, b, _)| {
            x >= f64::from(l) && x < f64::from(r) && y >= f64::from(t) && y < f64::from(b)
        })
}

/// Empty desktop space outside the visible regions remains clickable.
pub fn apply_region(
    window: &WebviewWindow,
    size: PhysicalSize<u32>,
    expanded: bool,
    controls: bool,
) {
    let Ok(handle) = window.hwnd() else {
        return;
    };
    let scale = f64::from(size.width) / WIDTH;
    if scale <= 0.0 || size.height == 0 {
        return;
    }
    let panel = [(4, 4, 220, 211, 26)];
    let height = if expanded { PANEL_HEIGHT } else { 0 };
    let rects = BODY
        .iter()
        .chain(BUTTONS.iter().take(if controls || expanded {
            BUTTONS.len()
        } else {
            0
        }))
        .map(|&(l, t, r, b, round)| (l, t + height, r, b + height, round))
        .chain(panel.into_iter().take(usize::from(expanded)));
    let px = |value: i32| (f64::from(value) * scale).round() as i32;
    // SAFETY: HWND came from this active WebView. Each HRGN is released once, except on
    // successful SetWindowRgn where Windows takes ownership. Calls run on the window thread.
    unsafe {
        let region = CreateRoundRectRgn(0, 0, 0, 0, 0, 0);
        if region.is_null() {
            return;
        }
        for (l, t, r, b, round) in rects {
            let piece = CreateRoundRectRgn(px(l), px(t), px(r), px(b), px(round), px(round));
            if !piece.is_null() {
                CombineRgn(region, region, piece, RGN_OR);
                DeleteObject(piece);
            }
        }
        if SetWindowRgn(handle.0, region, 1) == 0 {
            DeleteObject(region);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_round_trips_on_a_negative_origin_monitor() {
        let fraction = anchor(-1920, -780, 1920, 300);
        assert!((coordinate(-1920, 1920, 300, fraction) - (-780)).abs() <= 1);
        assert_eq!(coordinate(200, 100, 300, 10_000), 200);
        assert_eq!(coordinate(-800, 1400, 300, 10_000), 300);
        assert_eq!(anchor(-800, -1000, 1400, 300), 0);
        assert_eq!(anchor(-800, 5000, 1400, 300), 10_000);
    }
}
