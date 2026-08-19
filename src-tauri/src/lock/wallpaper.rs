//! The wallpaper behind the lock screen: the one on the desktop.
//!
//! The greeter shows the system wallpaper because it runs before anyone has
//! logged in and there is no personal setting to read. Here there is: the
//! session belongs to somebody, and the screen that covers it should be the one
//! they chose, not a flat colour that announces "this is a different program".
//!
//! It goes to the page as a `data:` URL for the same reason as in the greeter:
//! the webview cannot read `file://` from its own origin, and opening the
//! filesystem to a screen that handles passwords is a poor trade.

use std::path::PathBuf;

/// What the desktop is showing, from the same configuration it reads.
fn from_config() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let path = std::env::var("VASAK_CONFIG_PATH")
        .unwrap_or_else(|_| format!("{home}/.config/vasak/vasak.conf"));

    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;

    let first = config.pointer("/desktop/wallpaper/0")?.as_str()?;
    let first = first.strip_prefix("file://").unwrap_or(first);

    (!first.is_empty()).then(|| PathBuf::from(first))
}

/// `null` when there is nothing to show; the page then falls back to its own
/// background colour rather than to a blank screen.
#[tauri::command]
pub fn lock_background() -> Option<String> {
    from_config()
        .into_iter()
        // The same wallpaper the greeter falls back to, so a session that never
        // set one is still not a flat rectangle.
        .chain(std::iter::once(PathBuf::from(
            "/usr/share/backgrounds/vasakos/default.jpg",
        )))
        .find(|path| path.is_file())
        .and_then(|path| crate::users::image_data_url(&path))
}
