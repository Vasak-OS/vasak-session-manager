use pam_auth::Authenticator;
use serde::Serialize;
use std::process::Command;
use std::os::unix::process::CommandExt;
use nix::unistd::{Uid, Gid, User, Group};
use std::ffi::CString;

#[derive(Serialize)]
pub struct AuthResult {
    success: bool,
    message: String,
}

#[tauri::command]
pub fn authenticate(username: String, password: String) -> AuthResult {
    // Note: This requires the process to be able to use PAM. 
    // Usually requires root or setuid helper, relying on /etc/pam.d/ logic.
    // For "vdm", we assume a service name. "login" or a custom "vdm".
    // We'll use "login" for broad compatibility if "vdm" is not present, but better "vdm" or "system-auth".
    // pam-auth defaults to "system-auth"? No, constructor needs service name.
    
    // Using "login" service usually works for simple auth tests on Linux.
    let service = "login"; 
    
    match Authenticator::with_password(&service) {
        Ok(mut auth) => {
            auth.get_handler().set_credentials(&username, &password);
            match auth.authenticate() {
                Ok(_) => AuthResult { success: true, message: "Authenticated".to_string() },
                Err(e) => AuthResult { success: false, message: format!("Auth failed: {}", e) },
            }
        },
        Err(e) => AuthResult { success: false, message: format!("PAM init failed: {}", e) },
    }
}

#[tauri::command]
pub fn launch_session(username: String, cmd: String, session_type: String) -> Result<(), String> {
    // Resolve User Info
    let user = User::from_name(&username)
        .map_err(|e| format!("Lookup failed: {}", e))?
        .ok_or("User not found")?;
    
    // We will spawn the session. 
    // Important: This process must be running as root to setuid.
    // If we are not root, this will fail.
    
    if !Uid::current().is_root() {
        return Err("VDM backend must run as root to launch sessions.".to_string());
    }

    // Set environment variables
    // Should be done in the child process
    
    // Command splitting is naive here. Real DMs handle arguments carefully.,
    // session_cmd might be "gnome-session" or "/usr/bin/startplasma-wayland"
    
    // We use std::process::Command and pre_exec to drop privileges.
    
    let uid = user.uid;
    let gid = user.gid;
    let home = user.dir;
    let shell = user.shell;
    
    // Fork and exec logic handled by CommandExt::uid/gid?
    // Rust's CommandExt::uid() sets the user ID.
    
    // We also need to Initialize PAM session? 
    // pam-auth crate does authenticate, but 'open_session' is also needed for setting up limits, env, etc.
    // `pam-auth` crate provided here is simple. Simple implementation might skip full PAM session handling 
    // (creating directories, XDG_RUNTIME checking via PAM) which is risky but fits "MVP".
    // However, we at least ensure we drop privs.
    
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("Empty command".to_string());
    }
    let program = parts[0];
    let args = &parts[1..];
    
    let mut child = Command::new(program);
    child.args(args);
    
    // Set Env
    child.env("USER", &username);
    child.env("LOGNAME", &username);
    child.env("HOME", &home);
    child.env("SHELL", &shell);
    // XDG_RUNTIME_DIR is usually crucial for Wayland. 
    // /run/user/$UID
    let runtime_dir = format!("/run/user/{}", uid);
    child.env("XDG_RUNTIME_DIR", &runtime_dir);
    // PATH? Keep system path or user specific? Usually allow shell to set or keep inherited + additions.
    
    unsafe {
        child.uid(uid.as_raw());
        child.gid(gid.as_raw());
    }
    
    // Detach? We might want to wait or just spawn.
    // If we block verify, the UI freezes.
    // We should spawn and let it run.
    
    match child.spawn() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to launch: {}", e))
    }
}
