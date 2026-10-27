//! The wallpaper behind the lock screen: by default the one on the desktop.
//!
//! The greeter shows the system wallpaper because it runs before anyone has
//! logged in and there is no personal setting to read. Here there is: the
//! session belongs to somebody, and the screen that covers it should be the one
//! they chose, not a flat colour that announces "this is a different program".
//!
//! `lock.background.source` lets that choice be the desktop wallpaper (the
//! default), a picture of their own (`lock.background.image`), or the scheme's
//! flat colour — which is the one case where this returns `null` on purpose.
//!
//! It goes to the page as a `data:` URL for the same reason as in the greeter:
//! the webview cannot read `file://` from its own origin, and opening the
//! filesystem to a screen that handles passwords is a poor trade.

use std::path::PathBuf;

/// Lo que la sección `lock` elige como fondo. Sin esa sección, el del escritorio,
/// como siempre.
enum Background {
    /// Color liso del esquema: la página no dibuja ninguna imagen.
    Color,
    /// Una ruta a probar —la imagen propia o el fondo del escritorio—; `None`
    /// cuando la config no la trae, para que caiga al respaldo.
    Image(Option<PathBuf>),
}

/// La ruta en un puntero de la config, sin `file://` y descartando la cadena vacía.
fn path_at(config: &serde_json::Value, pointer: &str) -> Option<PathBuf> {
    let value = config.pointer(pointer)?.as_str()?;
    let value = value.strip_prefix("file://").unwrap_or(value);
    (!value.is_empty()).then(|| PathBuf::from(value))
}

/// Qué fondo pide la configuración. Lee `vasak.conf` una sola vez; si no se puede
/// leer, se comporta como hoy (el fondo del escritorio, con su respaldo).
fn from_config() -> Background {
    let Some(config) = read_config() else {
        return Background::Image(None);
    };

    // La pantalla de bloqueo corre como el usuario dueño de la sesión, así que
    // una ruta en su `vasak.conf` alcanza: no hace falta copiar nada como el
    // greeter.
    match config
        .pointer("/lock/background/source")
        .and_then(|v| v.as_str())
    {
        Some("color") => Background::Color,
        Some("image") => Background::Image(path_at(&config, "/lock/background/image")),
        // "desktop" o ausente: el fondo del escritorio, como siempre.
        _ => Background::Image(path_at(&config, "/desktop/wallpaper/0")),
    }
}

/// `vasak.conf` como JSON, o `None` si no se puede leer ni parsear.
fn read_config() -> Option<serde_json::Value> {
    let home = std::env::var("HOME").ok()?;
    let path = std::env::var("VASAK_CONFIG_PATH")
        .unwrap_or_else(|_| format!("{home}/.config/vasak/vasak.conf"));
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// `null` when there is nothing to show; the page then falls back to its own
/// background colour rather than to a blank screen.
#[tauri::command]
pub fn lock_background() -> Option<String> {
    let configured = match from_config() {
        // El color liso es a propósito: devolver `null` deja ver el color del
        // esquema, no el respaldo.
        Background::Color => return None,
        Background::Image(path) => path,
    };

    configured
        .into_iter()
        // The same wallpaper the greeter falls back to, so a session that never
        // set one is still not a flat rectangle.
        .chain(std::iter::once(PathBuf::from(
            "/usr/share/backgrounds/vasakos/default.jpg",
        )))
        .find(|path| path.is_file())
        .and_then(|path| crate::users::image_data_url(&path))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;
    use std::sync::{Mutex, MutexGuard};

    /// `VASAK_CONFIG_PATH` es global al proceso: las pruebas que lo tocan van de a una.
    static GUARDIA: Mutex<()> = Mutex::new(());

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];

    /// Un directorio temporal con su `vasak.conf`, apuntado por `VASAK_CONFIG_PATH`
    /// mientras viva.
    struct Config {
        dir: tempfile::TempDir,
        _guardia: MutexGuard<'static, ()>,
    }

    impl Config {
        fn nueva() -> Self {
            let guardia = GUARDIA.lock().unwrap_or_else(|e| e.into_inner());
            let dir = tempfile::tempdir().unwrap();
            std::env::set_var("VASAK_CONFIG_PATH", dir.path().join("vasak.conf"));
            Self {
                dir,
                _guardia: guardia,
            }
        }

        fn escribe(&self, contenido: impl ToString) {
            std::fs::write(self.dir.path().join("vasak.conf"), contenido.to_string()).unwrap();
        }

        /// Un archivo que `image_data_url` reconoce por su cabecera.
        fn imagen(&self, nombre: &str, cabecera: &[u8]) -> PathBuf {
            let ruta = self.dir.path().join(nombre);
            std::fs::write(&ruta, cabecera).unwrap();
            ruta
        }
    }

    impl Drop for Config {
        fn drop(&mut self) {
            std::env::remove_var("VASAK_CONFIG_PATH");
        }
    }

    #[test]
    fn con_color_liso_no_hay_imagen_aunque_el_escritorio_tenga_fondo() {
        let config = Config::nueva();
        let escritorio = config.imagen("escritorio.jpg", JPEG);
        config.escribe(json!({
            "desktop": {"wallpaper": [escritorio]},
            "lock": {"background": {"source": "color"}}
        }));

        assert!(matches!(from_config(), Background::Color));
        assert_eq!(lock_background(), None);
    }

    #[test]
    fn con_imagen_propia_se_elige_esa_ruta_y_no_la_del_escritorio() {
        let config = Config::nueva();
        let propia = config.imagen("propia.png", PNG);
        let escritorio = config.imagen("escritorio.jpg", JPEG);
        config.escribe(json!({
            "desktop": {"wallpaper": [escritorio]},
            "lock": {"background": {"source": "image", "image": propia}}
        }));

        assert!(matches!(from_config(), Background::Image(Some(p)) if p == propia));
        let url = lock_background().expect("la imagen propia existe");
        assert!(url.starts_with("data:image/png;base64,"), "{url}");
    }

    #[test]
    fn la_imagen_propia_acepta_el_prefijo_file() {
        let config = Config::nueva();
        let propia = config.imagen("propia.png", PNG);
        config.escribe(json!({
            "lock": {"background": {"source": "image", "image": format!("file://{}", propia.display())}}
        }));

        assert!(matches!(from_config(), Background::Image(Some(p)) if p == propia));
    }

    #[test]
    fn con_imagen_sin_ruta_o_vacia_cae_al_respaldo_sin_romper() {
        for lock in [
            json!({"background": {"source": "image"}}),
            json!({"background": {"source": "image", "image": ""}}),
            json!({"background": {"source": "image", "image": 7}}),
        ] {
            let config = Config::nueva();
            config.escribe(json!({ "lock": lock }));
            assert!(matches!(from_config(), Background::Image(None)));
            // Queda el respaldo del sistema, si existe; lo que importa es que no entre en pánico.
            let _ = lock_background();
        }
    }

    #[test]
    fn con_el_escritorio_o_sin_source_toma_el_fondo_del_escritorio() {
        for lock in [
            json!({"background": {"source": "desktop"}}),
            json!({"background": {}}),
            json!({}),
            json!({"background": {"source": "video"}}),
        ] {
            let config = Config::nueva();
            let escritorio = config.imagen("escritorio.jpg", JPEG);
            config.escribe(json!({"desktop": {"wallpaper": [escritorio]}, "lock": lock}));

            assert!(matches!(from_config(), Background::Image(Some(p)) if p == escritorio));
            let url = lock_background().expect("el fondo del escritorio existe");
            assert!(url.starts_with("data:image/jpeg;base64,"), "{url}");
        }
    }

    #[test]
    fn sin_la_seccion_lock_el_fondo_es_el_del_escritorio_como_siempre() {
        let config = Config::nueva();
        let escritorio = config.imagen("escritorio.jpg", JPEG);
        config.escribe(json!({"desktop": {"wallpaper": [escritorio]}}));

        assert!(matches!(from_config(), Background::Image(Some(p)) if p == escritorio));
    }

    #[test]
    fn una_config_ilegible_cae_al_respaldo_sin_entrar_en_panico() {
        for contenido in ["", "no es json {", "[1, 2]", "null"] {
            let config = Config::nueva();
            config.escribe(contenido);
            assert!(
                matches!(from_config(), Background::Image(None)),
                "{contenido:?}"
            );
            let _ = lock_background();
        }
    }

    #[test]
    fn una_config_que_no_existe_cae_al_respaldo_sin_entrar_en_panico() {
        let _config = Config::nueva();

        assert!(matches!(from_config(), Background::Image(None)));
        let _ = lock_background();
    }
}
