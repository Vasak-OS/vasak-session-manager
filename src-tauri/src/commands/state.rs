//! Remembers who logged in last and with which session.
//!
//! Every other display manager does this, and without it the login screen
//! preselects the alphabetically-first account on a shared machine, so the
//! usual person has to pick themselves from the list on every boot.
//!
//! Only the username and the session file name are stored — never a password —
//! in the greeter's own state directory, which no regular user can write to.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct LastLogin {
    pub username: Option<String>,
    pub session_id: Option<String>,
}

fn state_path() -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;

    Some(dir.join("vasak-session-manager").join("last-login.json"))
}

#[tauri::command]
pub fn get_last_login() -> LastLogin {
    state_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Records the selection. Failure is not reported to the caller: the greeter
/// still works perfectly without this, and a read-only state directory must not
/// turn into an error message on the login screen.
#[tauri::command]
pub fn set_last_login(username: String, session_id: String) {
    let Some(path) = state_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let record = LastLogin {
        username: Some(username),
        session_id: Some(session_id),
    };

    if let Ok(serialised) = serde_json::to_string(&record) {
        let _ = std::fs::write(path, serialised);
    }
}
