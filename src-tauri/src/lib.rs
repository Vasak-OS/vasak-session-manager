#[path = "./commands/users.rs"]
mod users;
#[path = "./commands/sessions.rs"]
mod sessions;
#[path = "./commands/auth.rs"]
mod auth;
#[path = "./commands/power.rs"]
mod power;
#[path = "./commands/keyboard.rs"]
mod keyboard;
#[path = "./commands/state.rs"]
mod state;
#[path = "./commands/screens.rs"]
mod screens;
#[path = "./commands/background.rs"]
mod background;
mod lock;

pub use lock::run as run_lock;

/// Where the translations live.
///
/// The i18n plugin only probes paths relative to the executable and the working
/// directory, neither of which exists once the binary is installed in
/// /usr/bin — a packaged greeter would render raw keys on the login screen.
pub(crate) fn locales_dir() -> Option<String> {
    [
        std::path::PathBuf::from("locales"),
        std::path::PathBuf::from("src-tauri/locales"),
        std::path::PathBuf::from("/usr/share/vasak-session-manager/locales"),
    ]
    .into_iter()
    .find(|path| path.is_dir())
    .map(|path| path.to_string_lossy().into_owned())
}

/// Startup language.
///
/// The greeter runs before anyone has logged in, so there is no user locale
/// yet; greetd's own environment is all there is. Spanish is the fallback,
/// matching the rest of VasakOS.
pub(crate) fn default_locale() -> String {
    let raw = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();

    match raw.split(['_', '.', '@']).next().unwrap_or("") {
        "en" => "en".to_string(),
        _ => "es".to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_i18n_vsk::init_with_path(
            Some(default_locale()),
            locales_dir(),
        ))
        .invoke_handler(tauri::generate_handler![
            users::get_users,
            sessions::get_sessions,
            auth::login,
            power::poweroff,
            power::reboot,
            power::suspend,
            keyboard::get_keyboard_layout,
            state::get_last_login,
            state::set_last_login,
            screens::get_screens,
            background::get_background
        ])
        .setup(|app| {
            // La ventana se crea acá y no en tauri.conf.json porque este
            // paquete tiene dos binarios: lo que se declara en la
            // configuración lo crea Tauri en los dos, y la pantalla de inicio
            // aparecía encima del bloqueo.
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::default(),
            )
            .title("vasak-session-manager")
            .inner_size(800.0, 600.0)
            .decorations(false)
            .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
