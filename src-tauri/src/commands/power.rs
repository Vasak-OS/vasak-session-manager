use std::process::Command;

/// Run a `systemctl` power action. The session manager runs as root, so these
/// succeed directly via logind.
fn systemctl(action: &str) -> Result<(), String> {
    let status = Command::new("systemctl")
        .arg(action)
        .status()
        .map_err(|e| format!("failed to run systemctl {action}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("systemctl {action} exited with {status}"))
    }
}

#[tauri::command]
pub fn poweroff() -> Result<(), String> {
    systemctl("poweroff")
}

#[tauri::command]
pub fn reboot() -> Result<(), String> {
    systemctl("reboot")
}

#[tauri::command]
pub fn suspend() -> Result<(), String> {
    systemctl("suspend")
}
