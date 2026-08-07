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

/// Authenticate `username` and start `cmd` via greetd. On success this process
/// exits so greetd can start the session; on failure it returns the error.
#[tauri::command]
pub fn login(
    username: String,
    password: String,
    cmd: String,
    session_type: String,
) -> Result<(), String> {
    let sock = std::env::var("GREETD_SOCK")
        .map_err(|_| "GREETD_SOCK not set (not running under greetd)".to_string())?;
    let mut stream =
        UnixStream::connect(&sock).map_err(|e| format!("cannot connect to greetd: {e}"))?;

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
    let env = vec![format!("XDG_SESSION_TYPE={session_type}")];

    match roundtrip(&mut stream, &Request::StartSession { cmd: cmd_vec, env })? {
        // greetd starts the session once this greeter exits.
        Response::Success => std::process::exit(0),
        Response::Error {
            error_type,
            description,
        } => Err(format!("{error_type}: {description}")),
        Response::AuthMessage { .. } => Err("unexpected auth message after start".to_string()),
    }
}
