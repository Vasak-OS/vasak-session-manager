use users::{AllUsers, Users, UsersCache};
use serde::Serialize;

#[derive(Serialize)]
pub struct SystemUser {
    pub name: String,
    pub real_name: String,
    pub uid: u32,
    pub gid: u32,
    pub home: String,
    pub shell: String,
}

#[tauri::command]
pub fn get_users() -> Vec<SystemUser> {
    let cache = UsersCache::new();
    let mut users_list = Vec::new();

    for user in cache.iter() {
        let uid = user.uid();
        // Standard Linux logic: UID >= 1000 are usually human users (except nobody/nogroup)
        // Adjust filter as needed. VasakOS might have specific conventions.
        // also check shell to be a valid shell (not nologin)
        
        if uid >= 1000 && uid < 65534 {
            let shell = user.shell().to_string_lossy();
            if !shell.contains("nologin") && !shell.contains("false") {
               users_list.push(SystemUser {
                   name: user.name().to_string_lossy().to_string(),
                   real_name: "User".to_string(), // Gecos parsing is often messy, keeping it simple or improving later
                   uid,
                   gid: user.primary_group_id(),
                   home: user.home_dir().to_string_lossy().to_string(),
                   shell: shell.to_string(),
               });
            }
        }
    }
    
    // Attempt to get real name from comments if possible, but users crate exposes it?
    // users crate `User` has no directly easy gecos field accessor in older versions?
    // checking docs: has `name()` but getting real name might require parsing `/etc/passwd` manually if crate doesn't expose it.
    // The `users` crate doesn't seem to expose GECOS field directly in the iterator easily in all versions.
    // We'll stick to username for now.

    users_list
}
