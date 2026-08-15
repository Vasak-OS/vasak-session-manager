//! Remembers who logged in last and which session each account uses.
//!
//! Every other display manager does this, and without it the login screen
//! preselects the alphabetically-first account on a shared machine, so the
//! usual person has to pick themselves from the list on every boot.
//!
//! The session is remembered *per account*, not once for the machine: on a
//! shared computer one person's choice of desktop would otherwise become
//! everybody's, and the next person would silently log into the wrong one.
//!
//! Only user names and session file names are stored — never a password — in a
//! directory that only the greeter can read.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct LastLogin {
    /// The account that signed in last, preselected on the next boot.
    pub username: Option<String>,
    /// Session chosen by each account, keyed by user name.
    #[serde(default)]
    pub sessions: HashMap<String, String>,
    /// The machine-wide session of the previous format. Read so an existing
    /// file still means something after an upgrade; never written again.
    #[serde(default, skip_serializing)]
    pub session_id: Option<String>,
}

/// Where the state file may live, best first.
///
/// The greeter account's home directory is `/` on an Arch system, which it
/// cannot write to, so the launcher points `HOME` at a temporary directory —
/// and anything written there is gone by the next boot, which is exactly when
/// this file is needed. `/var/lib/vasak-session-manager` is created for the
/// greeter by tmpfiles; the rest are fallbacks for running it by hand during
/// development.
fn state_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("/var/lib/vasak-session-manager")];

    if let Some(dir) = std::env::var_os("XDG_STATE_HOME") {
        paths.push(PathBuf::from(dir).join("vasak-session-manager"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(
            PathBuf::from(home)
                .join(".local/state")
                .join("vasak-session-manager"),
        );
    }

    paths
        .into_iter()
        .map(|dir| dir.join("last-login.json"))
        .collect()
}

/// Carries the pre-0.2.1 machine-wide session over to the account it belonged
/// to, so the first login after an upgrade still remembers what it should.
fn migrate(state: &mut LastLogin) {
    if let (true, Some(username), Some(session)) = (
        state.sessions.is_empty(),
        state.username.clone(),
        state.session_id.take(),
    ) {
        state.sessions.insert(username, session);
    }
}

fn read_state() -> LastLogin {
    let mut state: LastLogin = state_paths()
        .into_iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();

    migrate(&mut state);
    state
}

#[tauri::command]
pub fn get_last_login() -> LastLogin {
    read_state()
}

/// Records the selection. Failure is not reported to the caller: the greeter
/// still works perfectly without this, and a read-only state directory must not
/// turn into an error message on the login screen.
#[tauri::command]
pub fn set_last_login(username: String, session_id: String) {
    let mut state = read_state();
    state.sessions.insert(username.clone(), session_id);
    state.username = Some(username);

    let Ok(serialised) = serde_json::to_string(&state) else {
        return;
    };

    for path in state_paths() {
        let Some(parent) = path.parent() else {
            continue;
        };
        if !parent.is_dir() && std::fs::create_dir_all(parent).is_err() {
            continue;
        }

        // Written beside the real file and renamed over it: this runs while
        // greetd is starting the session and about to kill the greeter, and a
        // half-written file would be unreadable JSON on the next boot — the
        // greeter would forget everything instead of one login.
        let temporary = path.with_extension("json.new");
        if std::fs::write(&temporary, &serialised).is_ok() {
            if std::fs::rename(&temporary, &path).is_ok() {
                return;
            }
            let _ = std::fs::remove_file(&temporary);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> LastLogin {
        let mut state: LastLogin = serde_json::from_str(raw).unwrap();
        migrate(&mut state);
        state
    }

    #[test]
    fn the_session_of_an_older_state_file_is_kept_for_its_account() {
        let state = parse(r#"{"username":"ada","session_id":"wayfire.desktop"}"#);

        assert_eq!(state.username.as_deref(), Some("ada"));
        assert_eq!(
            state.sessions.get("ada").map(String::as_str),
            Some("wayfire.desktop")
        );
    }

    #[test]
    fn a_state_file_that_already_remembers_per_account_is_left_alone() {
        let state = parse(
            r#"{"username":"ada","session_id":"old.desktop",
                "sessions":{"ada":"wayfire.desktop","bob":"plasma.desktop"}}"#,
        );

        assert_eq!(
            state.sessions.get("ada").map(String::as_str),
            Some("wayfire.desktop")
        );
        assert_eq!(
            state.sessions.get("bob").map(String::as_str),
            Some("plasma.desktop")
        );
    }

    /// Two people on one machine must not overwrite each other's desktop.
    #[test]
    fn each_account_keeps_its_own_session() {
        let mut state = LastLogin::default();
        state.sessions.insert("ada".into(), "wayfire.desktop".into());
        state.sessions.insert("bob".into(), "plasma.desktop".into());

        let round_tripped = parse(&serde_json::to_string(&state).unwrap());

        assert_eq!(round_tripped.sessions.len(), 2);
        assert_eq!(
            round_tripped.sessions.get("ada").map(String::as_str),
            Some("wayfire.desktop")
        );
    }

    /// The written file never carries the old field back, so an upgrade does
    /// not keep re-applying a session that has since been changed.
    #[test]
    fn the_machine_wide_session_is_not_written_back() {
        let state = parse(r#"{"username":"ada","session_id":"wayfire.desktop"}"#);
        let serialised = serde_json::to_string(&state).unwrap();

        assert!(!serialised.contains("session_id"));
    }

    #[test]
    fn a_missing_or_unreadable_file_is_simply_nothing_remembered() {
        let state = LastLogin::default();

        assert!(state.username.is_none());
        assert!(state.sessions.is_empty());
    }
}
