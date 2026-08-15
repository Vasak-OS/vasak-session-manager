//! The wallpaper behind the login screen.
//!
//! The greeter runs before anyone has logged in, so there is no user setting to
//! read: the system wallpaper shipped with VasakOS is what it shows. An
//! administrator can point it somewhere else with a one-line file, which is
//! also where the settings application will write once choosing a login
//! wallpaper is exposed in the interface.
//!
//! The image is handed to the page as a `data:` URL. The webview cannot read
//! `file://` from its own origin, and opening the filesystem to it for the sake
//! of a background is a poor trade on a screen that handles passwords.

use std::path::{Path, PathBuf};

/// An override, one absolute path per file, first non-empty line wins.
const OVERRIDE_FILE: &str = "/etc/vasak-session-manager/background";

/// The wallpaper from `vasakos-wallpapers`, and the plain names an
/// administrator would expect to work if they drop a file in themselves.
const DEFAULTS: &[&str] = &[
    "/usr/share/backgrounds/vasakos/default.jpg",
    "/usr/share/backgrounds/vasakos/default.png",
    "/usr/share/backgrounds/vasakos/default.webp",
];

/// Big enough for a photographic wallpaper, small enough that a stray file
/// cannot stall the first paint of the login screen: the whole image crosses
/// the IPC boundary base64-encoded before anything is drawn.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

fn configured_path() -> Option<PathBuf> {
    let content = std::fs::read_to_string(OVERRIDE_FILE).ok()?;

    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(PathBuf::from)
}

fn is_usable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.len() > 0 && meta.len() <= MAX_BYTES)
        .unwrap_or(false)
}

/// `null` when there is no wallpaper to show; the page then falls back to its
/// own background colour rather than to a blank screen.
#[tauri::command]
pub fn get_background() -> Option<String> {
    configured_path()
        .into_iter()
        .chain(DEFAULTS.iter().map(PathBuf::from))
        .find(|path| is_usable(path))
        .and_then(|path| crate::users::image_data_url(&path))
}
