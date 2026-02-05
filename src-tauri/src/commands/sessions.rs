use std::fs;
use std::path::Path;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Serialize, Debug)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub path: String,
    pub session_type: String, // "wayland" or "x11"
}

fn parse_desktop_file(path: &Path) -> Option<(String, String)> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;

    for line in content.lines() {
        if line.starts_with("Name=") && name.is_none() {
            name = Some(line.trim_start_matches("Name=").to_string());
        }
        if line.starts_with("Exec=") && exec.is_none() {
            exec = Some(line.trim_start_matches("Exec=").to_string());
        }
    }

    if let (Some(n), Some(e)) = (name, exec) {
        Some((n, e))
    } else {
        None
    }
}

#[tauri::command]
pub fn get_sessions() -> Vec<Session> {
    let mut sessions = Vec::new();
    
    let dirs = vec![
        ("/usr/share/wayland-sessions", "wayland"),
        ("/usr/share/xsessions", "x11"),
    ];

    for (dir_path, s_type) in dirs {
        if !Path::new(dir_path).exists() {
            continue;
        }

        for entry in WalkDir::new(dir_path).min_depth(1).max_depth(1) {
            if let Ok(entry) = entry {
                if entry.path().extension().map_or(false, |ext| ext == "desktop") {
                    if let Some((name, exec)) = parse_desktop_file(entry.path()) {
                         sessions.push(Session {
                             id: entry.file_name().to_string_lossy().to_string(),
                             name,
                             exec,
                             path: entry.path().to_string_lossy().to_string(),
                             session_type: s_type.to_string(),
                         });
                    }
                }
            }
        }
    }
    
    sessions
}
