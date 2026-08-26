//! The list of people who can log in.
//!
//! Enumerated through NSS (`getpwent`) rather than by reading `/etc/passwd`, so
//! accounts that live in LDAP, SSSD or systemd-homed appear too.

use serde::Serialize;
use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Fallback range for human accounts when `/etc/login.defs` says nothing.
const DEFAULT_UID_MIN: u32 = 1000;
const DEFAULT_UID_MAX: u32 = 60000;

/// Avatars are read straight into the page as data URLs, so refuse anything
/// large enough to bloat the greeter's memory or stall its first paint.
const MAX_AVATAR_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize, Clone)]
pub struct SystemUser {
    pub name: String,
    /// The person's own name, from the GECOS field. Empty when unset — the UI
    /// falls back to the username rather than showing a placeholder.
    pub real_name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
    /// `data:` URL for the avatar, or `null` when the user has none.
    pub avatar: Option<String>,
}

/// `getpwent` walks a process-global cursor, so two threads iterating at once
/// would interleave and lose entries.
fn passwd_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

fn c_str_to_string(ptr: *const libc::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

/// Reads a numeric setting from `/etc/login.defs`.
///
/// Distributions move these boundaries — hardcoding 1000 hides accounts on a
/// system configured differently, and the greeter is the one place where an
/// invisible account means the person cannot log in at all.
fn login_defs_value(key: &str) -> Option<u32> {
    let content = std::fs::read_to_string("/etc/login.defs").ok()?;
    content.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix(key)?;
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// The GECOS field is comma-separated (`full name,room,work phone,…`); only the
/// first part is the person's name.
fn real_name_from_gecos(gecos: &str) -> String {
    gecos.split(',').next().unwrap_or("").trim().to_string()
}

fn is_login_shell(shell: &str) -> bool {
    !shell.is_empty()
        && !shell.ends_with("nologin")
        && !shell.ends_with("/false")
        && shell != "/bin/sync"
}

/// Finds an avatar for `user`, preferring the one the system already manages.
///
/// AccountsService is checked first because it is world-readable by design;
/// `~/.face` usually is not reachable at all, since the greeter runs as an
/// unprivileged user that cannot enter other people's home directories.
pub(crate) fn avatar_path(user: &str, home: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("/var/lib/AccountsService/icons").join(user),
        Path::new(home).join(".face"),
        Path::new(home).join(".face.icon"),
    ];

    candidates.into_iter().find(|path| {
        std::fs::metadata(path)
            .map(|meta| meta.is_file() && meta.len() > 0 && meta.len() <= MAX_AVATAR_BYTES)
            .unwrap_or(false)
    })
}

/// Reads an image into a `data:` URL, or `None` when it is not an image the
/// webview can draw. Shared with the login background, which cannot reach the
/// filesystem from the page either.
pub fn image_data_url(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;

    // Sniff the container rather than trusting the extension: AccountsService
    // icons have no extension at all.
    let mime = if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") {
        "image/svg+xml"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        return None;
    };

    Some(format!("data:{mime};base64,{}", STANDARD.encode(&bytes)))
}

#[tauri::command]
pub fn get_users() -> Vec<SystemUser> {
    let uid_min = login_defs_value("UID_MIN").unwrap_or(DEFAULT_UID_MIN);
    let uid_max = login_defs_value("UID_MAX").unwrap_or(DEFAULT_UID_MAX);

    let _guard = passwd_lock().lock();
    let mut users = Vec::new();

    unsafe {
        libc::setpwent();
        loop {
            let entry = libc::getpwent();
            if entry.is_null() {
                break;
            }
            let entry = &*entry;

            let uid = entry.pw_uid;
            if uid < uid_min || uid > uid_max {
                continue;
            }

            let shell = c_str_to_string(entry.pw_shell);
            if !is_login_shell(&shell) {
                continue;
            }

            let name = c_str_to_string(entry.pw_name);
            let home = c_str_to_string(entry.pw_dir);

            users.push(SystemUser {
                real_name: real_name_from_gecos(&c_str_to_string(entry.pw_gecos)),
                avatar: avatar_path(&name, &home).as_deref().and_then(image_data_url),
                name,
                uid,
                gid: entry.pw_gid,
                home,
                shell,
            });
        }
        libc::endpwent();
    }

    // Alphabetical by the name that is actually displayed, so the list does not
    // reshuffle between boots the way NSS order can.
    users.sort_by(|a, b| {
        let left = if a.real_name.is_empty() { &a.name } else { &a.real_name };
        let right = if b.real_name.is_empty() { &b.name } else { &b.real_name };
        left.to_lowercase().cmp(&right.to_lowercase())
    });

    users
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_gecos_field_is_the_persons_name() {
        assert_eq!(real_name_from_gecos("Ada Lovelace,,,"), "Ada Lovelace");
        assert_eq!(real_name_from_gecos("  Ada  "), "Ada");
        assert_eq!(real_name_from_gecos(""), "");
        // Some systems put the username there with no name at all.
        assert_eq!(real_name_from_gecos(",,,"), "");
    }

    #[test]
    fn accounts_without_a_usable_shell_are_not_offered() {
        assert!(is_login_shell("/bin/bash"));
        assert!(is_login_shell("/usr/bin/zsh"));
        assert!(!is_login_shell("/usr/sbin/nologin"));
        assert!(!is_login_shell("/sbin/nologin"));
        assert!(!is_login_shell("/bin/false"));
        assert!(!is_login_shell(""));
    }
}
