//! The shape of the screens the greeter is drawn on.
//!
//! The greeter is hosted by Cage, a kiosk compositor, and Cage gives its client
//! a single surface covering the whole output layout — there is no way to open
//! one window per monitor, because Cage shows exactly one window at a time.
//! So the multi-monitor login screen is drawn *inside* that one surface: the
//! page paints a panel per monitor and puts the login box on one of them.
//!
//! For that to line up with the physical screens the page needs the layout in
//! the same units it draws in — CSS pixels, which on GTK are logical pixels
//! with the layout origin at the window's top-left corner. Tauri reports
//! monitors in physical pixels relative to the compositor's layout, so both
//! conversions happen here rather than being guessed at in the frontend.

use serde::Serialize;

#[derive(Serialize)]
pub struct Screen {
    /// Position in the layout, stable across reloads. Monitor names are not:
    /// they are frequently absent or repeated ("Unknown").
    pub index: usize,
    pub name: String,
    /// Logical pixels, relative to the top-left corner of the greeter surface.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// The screen to fall back to when the pointer is not on any of them.
    pub primary: bool,
}

#[derive(Serialize, Default)]
pub struct ScreenLayout {
    /// Bounding box of every screen, in the same logical pixels.
    pub width: i32,
    pub height: i32,
    pub screens: Vec<Screen>,
}

/// Physical monitor geometry as logical pixels.
///
/// A monitor rendered at 200% covers twice the pixels it occupies on the page,
/// so without dividing by the scale factor the panels of a mixed-DPI setup
/// would land beside their screens instead of on them.
fn logical_rect(monitor: &tauri::Monitor) -> (i32, i32, i32, i32) {
    let scale = if monitor.scale_factor() > 0.0 {
        monitor.scale_factor()
    } else {
        1.0
    };
    let position = monitor.position();
    let size = monitor.size();

    (
        (f64::from(position.x) / scale).round() as i32,
        (f64::from(position.y) / scale).round() as i32,
        (f64::from(size.width) / scale).round() as i32,
        (f64::from(size.height) / scale).round() as i32,
    )
}

/// Which screen the login box belongs on when the pointer has not been moved.
///
/// GDK only reports a primary monitor on X11; under Wayland it reports none at
/// all, so the monitor sitting at the origin of the layout is taken instead —
/// which is where compositors put the one they consider first.
fn primary_index(monitors: &[tauri::Monitor], primary: Option<&tauri::Monitor>) -> usize {
    if let Some(primary) = primary {
        let target = logical_rect(primary);
        if let Some(index) = monitors
            .iter()
            .position(|monitor| logical_rect(monitor) == target)
        {
            return index;
        }
    }

    monitors
        .iter()
        .enumerate()
        .min_by_key(|(_, monitor)| {
            let (x, y, _, _) = logical_rect(monitor);
            (i64::from(x).pow(2) + i64::from(y).pow(2), x, y)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Fails softly: a greeter that cannot describe the screens still has to draw a
/// login box, and an empty layout tells the page to fill the window with one.
#[tauri::command]
pub fn get_screens(window: tauri::Window) -> ScreenLayout {
    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return ScreenLayout::default();
    }

    let primary = window.primary_monitor().ok().flatten();
    let primary = primary_index(&monitors, primary.as_ref());

    let rects: Vec<(i32, i32, i32, i32)> = monitors.iter().map(logical_rect).collect();

    // The layout does not have to start at (0, 0) — a monitor placed above or
    // to the left of the first one gives it negative coordinates, and the
    // surface still begins at the top-left of the bounding box.
    let origin_x = rects.iter().map(|rect| rect.0).min().unwrap_or(0);
    let origin_y = rects.iter().map(|rect| rect.1).min().unwrap_or(0);

    let screens: Vec<Screen> = monitors
        .iter()
        .zip(&rects)
        .enumerate()
        .map(|(index, (monitor, &(x, y, width, height)))| Screen {
            index,
            name: monitor.name().cloned().unwrap_or_default(),
            x: x - origin_x,
            y: y - origin_y,
            width,
            height,
            primary: index == primary,
        })
        .collect();

    ScreenLayout {
        width: screens
            .iter()
            .map(|screen| screen.x + screen.width)
            .max()
            .unwrap_or(0),
        height: screens
            .iter()
            .map(|screen| screen.y + screen.height)
            .max()
            .unwrap_or(0),
        screens,
    }
}
