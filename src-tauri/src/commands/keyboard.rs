//! What keyboard layout the login screen is actually typing in.
//!
//! The greeter cannot change the keymap — the compositor owns it, and the
//! launcher script sets it up from the system configuration. What the greeter
//! can do is *say* which layout is active, because a password typed on an
//! unexpected layout is invisible in a password field and looks exactly like a
//! forgotten password.

use serde::Serialize;

#[derive(Serialize)]
pub struct KeyboardLayout {
    /// Every layout the compositor was given, in switching order.
    pub layouts: Vec<String>,
    /// True when more than one is available, so the UI can mention the shortcut.
    pub switchable: bool,
}

/// Reads `Option "<key>" "<value>"` out of the X keyboard configuration, which
/// is what `localectl` writes and what the launcher reads too.
fn xorg_keyboard_option(key: &str) -> Option<String> {
    let content = std::fs::read_to_string("/etc/X11/xorg.conf.d/00-keyboard.conf").ok()?;

    content.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("Option")?.trim_start();
        let mut parts = rest.split('"').filter(|part| !part.trim().is_empty());

        let found_key = parts.next()?;
        if !found_key.eq_ignore_ascii_case(key) {
            return None;
        }
        let value = parts.next()?.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn vconsole_keymap() -> Option<String> {
    let content = std::fs::read_to_string("/etc/vconsole.conf").ok()?;

    content.lines().find_map(|line| {
        let value = line.trim().strip_prefix("KEYMAP=")?.trim_matches('"');
        // Console keymaps carry variants the XKB names don't have ("es-dvorak").
        let base = value.split('-').next()?.trim();
        (!base.is_empty()).then(|| base.to_string())
    })
}

fn parse_layouts(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

#[tauri::command]
pub fn get_keyboard_layout() -> KeyboardLayout {
    // The launcher exports this before starting the compositor, so it is the
    // most direct answer: it is literally what the keymap was built from.
    let layouts = std::env::var("XKB_DEFAULT_LAYOUT")
        .ok()
        .map(|value| parse_layouts(&value))
        .filter(|layouts| !layouts.is_empty())
        .or_else(|| xorg_keyboard_option("XkbLayout").map(|value| parse_layouts(&value)))
        .or_else(|| vconsole_keymap().map(|value| vec![value]))
        .unwrap_or_else(|| vec!["us".to_string()]);

    KeyboardLayout {
        switchable: layouts.len() > 1,
        layouts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_layout_list_is_split_on_commas() {
        assert_eq!(parse_layouts("es,us"), vec!["es", "us"]);
        assert_eq!(parse_layouts(" es , us "), vec!["es", "us"]);
        assert_eq!(parse_layouts("es"), vec!["es"]);
        assert!(parse_layouts("").is_empty());
        assert!(parse_layouts(" , ").is_empty());
    }
}
