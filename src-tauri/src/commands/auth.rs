use pam::Client;
use serde::Serialize;
use std::ffi::CString;
use std::os::unix::process::CommandExt;
use std::process::Command;

use nix::unistd::{initgroups, setgid, setuid, Uid, User};

/// PAM service name. Ships as /etc/pam.d/vasak-session-manager (see packaging).
const PAM_SERVICE: &str = "vasak-session-manager";

#[derive(Serialize)]
pub struct AuthResult {
    success: bool,
    message: String,
}

#[tauri::command]
pub fn authenticate(username: String, password: String) -> AuthResult {
    let mut client = match Client::with_password(PAM_SERVICE) {
        Ok(c) => c,
        Err(e) => {
            return AuthResult {
                success: false,
                message: format!("PAM init failed: {:?}", e),
            }
        }
    };

    client.conversation_mut().set_credentials(username, password);

    match client.authenticate() {
        Ok(_) => AuthResult {
            success: true,
            message: "Authenticated".to_string(),
        },
        Err(e) => AuthResult {
            success: false,
            message: format!("Auth failed: {:?}", e),
        },
    }
}

/// Map a nix errno to an io::Error for use inside `pre_exec`.
fn nix_to_io(e: nix::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(e as i32)
}

/// Strip XDG desktop-entry field codes (e.g. `%U`, `%k`, `%i`) from an Exec
/// line, collapsing `%%` to `%`. Session `Exec=` lines usually have none, but
/// stripping them avoids passing bogus arguments to the compositor.
fn strip_field_codes(cmd: &str) -> Vec<String> {
    cmd.split_whitespace()
        .filter_map(|tok| {
            if tok.len() == 2 && tok.starts_with('%') && tok != "%%" {
                None
            } else {
                Some(tok.replace("%%", "%"))
            }
        })
        .collect()
}

#[tauri::command]
pub fn launch_session(username: String, cmd: String, session_type: String) -> Result<(), String> {
    let user = User::from_name(&username)
        .map_err(|e| format!("Lookup failed: {}", e))?
        .ok_or("User not found")?;

    // Launching a session requires root to drop to the target user.
    if !Uid::current().is_root() {
        return Err("vasak-session-manager backend must run as root to launch sessions.".to_string());
    }

    let uid = user.uid;
    let gid = user.gid;
    let home = user.dir.clone();
    let shell = user.shell.clone();
    let runtime_dir = format!("/run/user/{}", uid.as_raw());

    let parts = strip_field_codes(&cmd);
    let program = parts.first().ok_or("Empty command")?.clone();
    let args: Vec<String> = parts.into_iter().skip(1).collect();

    let mut command = Command::new(&program);
    command
        .args(&args)
        .env("USER", &username)
        .env("LOGNAME", &username)
        .env("HOME", &home)
        .env("SHELL", &shell)
        .env("XDG_RUNTIME_DIR", &runtime_dir)
        .env("XDG_SESSION_TYPE", &session_type);

    // Drop privileges in the child in the correct order: initialise the user's
    // supplementary groups and set the gid *while still root*, then drop the uid
    // last. Rust's Command::uid/gid do NOT set supplementary groups, so a user
    // launched that way would lose audio/video/input/wheel membership.
    let user_c = CString::new(username.clone()).map_err(|_| "invalid username".to_string())?;
    unsafe {
        command.pre_exec(move || {
            initgroups(&user_c, gid).map_err(nix_to_io)?;
            setgid(gid).map_err(nix_to_io)?;
            setuid(uid).map_err(nix_to_io)?;
            Ok(())
        });
    }

    // NOTE: this spawns the session as a child of the greeter and does NOT open
    // a PAM session (pam_open_session/setcred) nor hand off the TTY/DRM master
    // from cage to the new compositor. A robust greeter needs a persistent root
    // session daemon (greetd-style) to own the PAM transaction and the seat.
    // Tracked as a pending architectural decision.
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to launch: {}", e))
}
