//! greetd IPC client.
//!
//! vasak-session-manager runs as an unprivileged greetd greeter. It does NOT do
//! PAM or privilege dropping itself — it collects the username/password/session
//! and drives greetd over its Unix-socket JSON protocol (`$GREETD_SOCK`). greetd
//! (running as root) owns the PAM transaction, the seat and the TTY, and starts
//! the user session after this process exits.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Upper bound on any single exchange with greetd.
///
/// PAM deliberately delays a rejected password, so this has to be generous —
/// but without it a greetd that never answers leaves the login screen stuck on
/// "Authenticating…" with no way out short of a hard reboot.
const IO_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Request {
    CreateSession { username: String },
    PostAuthMessageResponse { response: Option<String> },
    StartSession { cmd: Vec<String>, env: Vec<String> },
    CancelSession,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Response {
    Success,
    Error {
        error_type: String,
        description: String,
    },
    AuthMessage {
        auth_message_type: String,
        auth_message: String,
    },
}

/// Send one request and read one response. Messages are a native-endian u32
/// length prefix followed by the JSON payload.
fn roundtrip(stream: &mut UnixStream, req: &Request) -> Result<Response, String> {
    let payload = serde_json::to_vec(req).map_err(|e| e.to_string())?;
    let len = (payload.len() as u32).to_ne_bytes();
    stream.write_all(&len).map_err(|e| e.to_string())?;
    stream.write_all(&payload).map_err(|e| e.to_string())?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let n = u32::from_ne_bytes(len_buf) as usize;
    let mut buf = vec![0u8; n];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;
    serde_json::from_slice(&buf).map_err(|e| e.to_string())
}

/// Strip XDG desktop-entry field codes (e.g. `%U`, `%k`) from an Exec line,
/// collapsing `%%` to `%`.
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

/// Wraps an X11 session so it actually starts.
///
/// greetd hands the command straight to `exec`, with no X server behind it, so
/// an `Exec=` line taken from `/usr/share/xsessions` would die immediately —
/// X11 sessions were listed on the login screen but could never be used.
/// `startx` brings up the server and then runs the session as its client;
/// `env` is in between because `startx` only accepts an absolute path there.
fn wrap_for_session_type(session_type: &str, cmd: Vec<String>) -> Vec<String> {
    if session_type != "x11" {
        return cmd;
    }

    let mut wrapped = vec!["startx".to_string(), "/usr/bin/env".to_string()];
    wrapped.extend(cmd);
    wrapped
}

/// Builds the environment the desktop needs to identify itself.
///
/// Without these, applications cannot tell which desktop they are running
/// under: portals pick the wrong backend, and menus and theming fall back to
/// generic defaults.
fn session_environment(
    session_id: &str,
    session_type: &str,
    desktop_names: &[String],
) -> Vec<String> {
    // `foo.desktop` → `foo`, which is what DESKTOP_SESSION has always meant.
    let session_name = Path::new(session_id)
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| session_id.to_string());

    let current_desktop = if desktop_names.is_empty() {
        session_name.clone()
    } else {
        desktop_names.join(":")
    };

    vec![
        format!("XDG_SESSION_TYPE={session_type}"),
        format!("XDG_SESSION_DESKTOP={session_name}"),
        format!("DESKTOP_SESSION={session_name}"),
        format!("XDG_CURRENT_DESKTOP={current_desktop}"),
    ]
}

/// Drives one full greetd exchange: authenticate, then start the session.
///
/// On success the process exits so greetd can take over, so this only ever
/// returns on failure.
fn run_login(
    username: String,
    password: String,
    cmd: String,
    session_id: String,
    session_type: String,
    desktop_names: Vec<String>,
) -> Result<(), String> {
    let sock = std::env::var("GREETD_SOCK")
        .map_err(|_| "GREETD_SOCK not set (not running under greetd)".to_string())?;
    let mut stream =
        UnixStream::connect(&sock).map_err(|e| format!("cannot connect to greetd: {e}"))?;

    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|e| format!("cannot set greetd socket timeouts: {e}"))?;

    let mut resp = roundtrip(
        &mut stream,
        &Request::CreateSession {
            username: username.clone(),
        },
    )?;

    // Answer the PAM conversation. For password auth this is a single `secret`
    // prompt, but handle the general case defensively.
    let mut password = Some(password);
    loop {
        match resp {
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => {
                let reply = match auth_message_type.as_str() {
                    "secret" => password.take(),
                    "visible" => Some(username.clone()),
                    _ => {
                        // info / error: surface the text (e.g. account expiry) and
                        // acknowledge with no input.
                        eprintln!("[greetd:{auth_message_type}] {auth_message}");
                        None
                    }
                };
                resp = roundtrip(&mut stream, &Request::PostAuthMessageResponse { response: reply })?;
            }
            Response::Success => break,
            Response::Error {
                error_type,
                description,
            } => {
                let _ = roundtrip(&mut stream, &Request::CancelSession);
                return Err(format!("{error_type}: {description}"));
            }
        }
    }

    let cmd_vec = strip_field_codes(&cmd);
    if cmd_vec.is_empty() {
        let _ = roundtrip(&mut stream, &Request::CancelSession);
        return Err("Empty session command".to_string());
    }

    let request = Request::StartSession {
        cmd: wrap_for_session_type(&session_type, cmd_vec),
        env: session_environment(&session_id, &session_type, &desktop_names),
    };

    match roundtrip(&mut stream, &request)? {
        // greetd starts the session once this greeter exits.
        Response::Success => std::process::exit(0),
        Response::Error {
            error_type,
            description,
        } => Err(format!("{error_type}: {description}")),
        Response::AuthMessage { .. } => Err("unexpected auth message after start".to_string()),
    }
}

/// Authenticate `username` and start the chosen session via greetd.
///
/// Runs off the main thread: PAM makes a rejected password wait several seconds
/// on purpose, and doing that inline froze the whole login screen — the typed
/// password stayed on screen and the button never showed it was working.
#[tauri::command]
pub async fn login(
    username: String,
    password: String,
    cmd: String,
    session_id: String,
    session_type: String,
    desktop_names: Vec<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_login(username, password, cmd, session_id, session_type, desktop_names)
    })
    .await
    .map_err(|e| format!("login task failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_codes_are_removed_from_the_command() {
        assert_eq!(strip_field_codes("gnome-session %U"), vec!["gnome-session"]);
        assert_eq!(
            strip_field_codes("run --flag %k file"),
            vec!["run", "--flag", "file"]
        );
        // `%%` is a literal percent sign, not a field code.
        assert_eq!(strip_field_codes("run 100%%"), vec!["run", "100%"]);
    }

    #[test]
    fn wayland_sessions_run_unwrapped() {
        let cmd = vec!["vasak-session".to_string()];
        assert_eq!(wrap_for_session_type("wayland", cmd.clone()), cmd);
    }

    #[test]
    fn x11_sessions_are_started_under_an_x_server() {
        assert_eq!(
            wrap_for_session_type("x11", vec!["i3".to_string(), "--flag".to_string()]),
            vec!["startx", "/usr/bin/env", "i3", "--flag"]
        );
    }

    #[test]
    fn the_session_identifies_itself_to_applications() {
        let env = session_environment(
            "vasak.desktop",
            "wayland",
            &["VasakOS".to_string(), "wlroots".to_string()],
        );

        assert!(env.contains(&"XDG_SESSION_TYPE=wayland".to_string()));
        assert!(env.contains(&"XDG_SESSION_DESKTOP=vasak".to_string()));
        assert!(env.contains(&"DESKTOP_SESSION=vasak".to_string()));
        assert!(env.contains(&"XDG_CURRENT_DESKTOP=VasakOS:wlroots".to_string()));
    }

    /// Plenty of session files omit DesktopNames; the session still has to be
    /// identifiable, so the file name stands in.
    #[test]
    fn a_session_without_desktop_names_falls_back_to_its_file_name() {
        let env = session_environment("sway.desktop", "wayland", &[]);
        assert!(env.contains(&"XDG_CURRENT_DESKTOP=sway".to_string()));
    }
}
