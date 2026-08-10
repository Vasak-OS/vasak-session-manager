//! Discovery of the desktop sessions offered on the login screen.

use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Debug, Clone)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub comment: String,
    pub exec: String,
    pub path: String,
    /// `"wayland"` or `"x11"`.
    pub session_type: String,
    /// `DesktopNames`, used to set `XDG_CURRENT_DESKTOP` for the session.
    pub desktop_names: Vec<String>,
}

/// A parsed `[Desktop Entry]` group.
struct DesktopEntry {
    name: String,
    comment: String,
    exec: String,
    try_exec: String,
    hidden: bool,
    desktop_names: Vec<String>,
}

/// Parses a session `.desktop` file.
///
/// Only the `[Desktop Entry]` group is read. The previous version scanned every
/// line in the file, so a `Name=` inside a trailing action group could win over
/// the real one and the session would be listed under the wrong label.
fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
    let content = fs::read_to_string(path).ok()?;

    let mut entry = DesktopEntry {
        name: String::new(),
        comment: String::new(),
        exec: String::new(),
        try_exec: String::new(),
        hidden: false,
        desktop_names: Vec::new(),
    };

    let mut in_entry_group = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') {
            in_entry_group = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry_group {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());

        // Localised keys look like `Name[es]`. The greeter runs before any user
        // locale is known, so the untranslated key is the right one to take.
        if key.contains('[') {
            continue;
        }

        match key {
            "Name" => entry.name = value.to_string(),
            "Comment" => entry.comment = value.to_string(),
            "Exec" => entry.exec = value.to_string(),
            "TryExec" => entry.try_exec = value.to_string(),
            // Either flag means "don't offer this to the user".
            "Hidden" | "NoDisplay" => {
                if value.eq_ignore_ascii_case("true") {
                    entry.hidden = true;
                }
            }
            "DesktopNames" => {
                entry.desktop_names = value
                    .split(';')
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            _ => {}
        }
    }

    if entry.name.is_empty() || entry.exec.is_empty() || entry.hidden {
        return None;
    }
    Some(entry)
}

/// Honours `TryExec`: an entry whose program is not installed is listed by
/// several desktops but cannot start, and picking it would drop the user back
/// at the login screen with no explanation.
fn program_exists(try_exec: &str) -> bool {
    if try_exec.is_empty() {
        return true;
    }

    if try_exec.contains('/') {
        return Path::new(try_exec).exists();
    }

    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(try_exec).exists()))
        .unwrap_or(false)
}

#[tauri::command]
pub fn get_sessions() -> Vec<Session> {
    let mut sessions = Vec::new();

    for (dir_path, session_type) in [
        ("/usr/share/wayland-sessions", "wayland"),
        ("/usr/share/xsessions", "x11"),
    ] {
        let Ok(entries) = fs::read_dir(dir_path) else {
            continue;
        };

        for dir_entry in entries.flatten() {
            let path = dir_entry.path();
            if path.extension().is_none_or(|ext| ext != "desktop") {
                continue;
            }

            let Some(entry) = parse_desktop_file(&path) else {
                continue;
            };
            if !program_exists(&entry.try_exec) {
                continue;
            }

            sessions.push(Session {
                id: dir_entry.file_name().to_string_lossy().to_string(),
                name: entry.name,
                comment: entry.comment,
                exec: entry.exec,
                path: path.to_string_lossy().to_string(),
                session_type: session_type.to_string(),
                desktop_names: entry.desktop_names,
            });
        }
    }

    // Wayland first, then alphabetical — VasakOS is a Wayland desktop, and the
    // first entry is what gets preselected.
    sessions.sort_by(|a, b| {
        (a.session_type != "wayland")
            .cmp(&(b.session_type != "wayland"))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_entry(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::Builder::new()
            .suffix(".desktop")
            .tempfile()
            .expect("temp file");
        file.write_all(contents.as_bytes()).expect("write");
        file
    }

    #[test]
    fn reads_the_desktop_entry_group() {
        let file = write_entry(
            "[Desktop Entry]\n\
             Name=Vasak\n\
             Comment=The VasakOS desktop\n\
             Exec=vasak-session\n\
             DesktopNames=VasakOS;wlroots;\n",
        );
        let entry = parse_desktop_file(file.path()).expect("parsed");

        assert_eq!(entry.name, "Vasak");
        assert_eq!(entry.comment, "The VasakOS desktop");
        assert_eq!(entry.exec, "vasak-session");
        assert_eq!(entry.desktop_names, vec!["VasakOS", "wlroots"]);
    }

    /// Keys after the entry group belong to other groups and used to overwrite
    /// the session's real name.
    #[test]
    fn later_groups_do_not_override_the_entry() {
        let file = write_entry(
            "[Desktop Entry]\n\
             Name=Vasak\n\
             Exec=vasak-session\n\
             \n\
             [Desktop Action New]\n\
             Name=New Window\n\
             Exec=vasak-session --new\n",
        );
        let entry = parse_desktop_file(file.path()).expect("parsed");

        assert_eq!(entry.name, "Vasak");
        assert_eq!(entry.exec, "vasak-session");
    }

    #[test]
    fn hidden_entries_are_not_offered() {
        for flag in ["Hidden=true", "NoDisplay=true"] {
            let file = write_entry(&format!(
                "[Desktop Entry]\nName=Hidden\nExec=nope\n{flag}\n"
            ));
            assert!(parse_desktop_file(file.path()).is_none(), "{flag}");
        }
    }

    #[test]
    fn localised_names_are_ignored_since_no_locale_is_known_yet() {
        let file = write_entry(
            "[Desktop Entry]\nName[es]=Escritorio\nName=Desktop\nExec=run\n",
        );
        assert_eq!(parse_desktop_file(file.path()).unwrap().name, "Desktop");
    }

    #[test]
    fn an_entry_without_a_command_is_not_a_session() {
        let file = write_entry("[Desktop Entry]\nName=Nameless\n");
        assert!(parse_desktop_file(file.path()).is_none());
    }

    #[test]
    fn try_exec_gates_on_the_program_being_installed() {
        assert!(program_exists(""), "no TryExec means no restriction");
        assert!(program_exists("/bin/sh"));
        assert!(!program_exists("/nonexistent/definitely-not-here"));
        assert!(!program_exists("definitely-not-a-real-program-xyz"));
    }
}
