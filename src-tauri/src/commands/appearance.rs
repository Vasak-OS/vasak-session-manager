//! Los colores del inicio de sesión.
//!
//! El greeter corre antes de que exista una sesión, así que no hay
//! configuración de usuario que leer: los colores salen de dos archivos que
//! deja la aplicación de configuración —con permisos de administrador— junto al
//! del fondo.
//!
//! * `theme` → `dark` o `light`, una línea.
//! * `scheme.json` → el documento del esquema de colores **copiado entero**, no
//!   su id: los esquemas del usuario viven en `~/.config/vasak/schemes`, y ese
//!   directorio no existe para el greeter, que corre sin sesión y con otro
//!   usuario.
//!
//! Hasta que estos archivos existan, el inicio de sesión se dibuja con la
//! paleta oscura que trae compilada. Un archivo ilegible o roto también cae en
//! esa paleta: es una pantalla de la que no se puede salir, y quedarse sin
//! colores sería quedarse sin poder entrar.

use std::io::Read;
use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

const CONFIG_DIR: &str = "/etc/vasak-session-manager";
const THEME_FILE: &str = "theme";
const SCHEME_FILE: &str = "scheme.json";

/// Un esquema son unos cientos de colores en texto; cualquier cosa más grande
/// que esto no es un esquema, y este archivo se lee antes del primer dibujado.
const MAX_SCHEME_BYTES: u64 = 256 * 1024;

/// Lo que la página necesita para pintarse.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Appearance {
    /// `dark` o `light`. Nunca vacío: si no hay nada configurado, `dark`.
    pub theme: String,
    /// El esquema tal cual, para que el frontend saque los colores igual que
    /// hace el plugin de configuración en el resto de las aplicaciones.
    pub scheme: Option<Value>,
}

fn ruta(archivo: &str) -> PathBuf {
    PathBuf::from(CONFIG_DIR).join(archivo)
}

/// La primera línea con algo, sin espacios alrededor.
fn primera_linea(contenido: &str) -> Option<&str> {
    contenido
        .lines()
        .map(str::trim)
        .find(|linea| !linea.is_empty())
}

/// `dark` y `light` son los dos valores que existen; cualquier otra cosa es un
/// archivo escrito a mano con un error, y se ignora en silencio.
fn tema_valido(valor: &str) -> Option<&'static str> {
    match valor.trim().to_ascii_lowercase().as_str() {
        "dark" => Some("dark"),
        "light" => Some("light"),
        _ => None,
    }
}

/// Un esquema sirve si tiene las dos variantes con la sección `ui`: el frontend
/// lee de las dos —los colores claros y los oscuros van a variables distintas—
/// y si falta una, media interfaz queda sin color.
fn esquema_utilizable(documento: &Value) -> bool {
    let colores = &documento["colors"];

    ["dark", "light"]
        .iter()
        .all(|variante| colores[variante]["ui"].is_object())
}

fn leer_acotado(archivo: &str, maximo: u64) -> Option<String> {
    let camino = ruta(archivo);
    let handle = std::fs::File::open(&camino).ok()?;

    // Acotado en la lectura y no después: `read_to_string` de un archivo enorme
    // ya ocupó la memoria cuando uno se entera del tamaño.
    let mut contenido = String::new();
    handle.take(maximo + 1).read_to_string(&mut contenido).ok()?;

    if contenido.len() as u64 > maximo {
        eprintln!("[apariencia] {} es más grande que el máximo, se ignora", camino.display());
        return None;
    }

    Some(contenido)
}

fn leer_tema() -> String {
    leer_acotado(THEME_FILE, 64)
        .as_deref()
        .and_then(primera_linea)
        .and_then(tema_valido)
        .unwrap_or("dark")
        .to_string()
}

fn leer_esquema() -> Option<Value> {
    let contenido = leer_acotado(SCHEME_FILE, MAX_SCHEME_BYTES)?;

    match serde_json::from_str::<Value>(&contenido) {
        Ok(documento) if esquema_utilizable(&documento) => Some(documento),
        Ok(_) => {
            eprintln!("[apariencia] {SCHEME_FILE} no tiene las dos variantes de color, se ignora");
            None
        }
        Err(error) => {
            eprintln!("[apariencia] {SCHEME_FILE} no es JSON válido: {error}");
            None
        }
    }
}

#[tauri::command]
pub fn get_appearance() -> Appearance {
    Appearance {
        theme: leer_tema(),
        scheme: leer_esquema(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn el_tema_acepta_las_dos_palabras_y_nada_mas() {
        assert_eq!(tema_valido("dark"), Some("dark"));
        assert_eq!(tema_valido("light"), Some("light"));
        assert_eq!(tema_valido("  Dark\n"), Some("dark"), "espacios y mayúsculas");
        assert_eq!(tema_valido("oscuro"), None);
        assert_eq!(tema_valido(""), None);
    }

    #[test]
    fn la_primera_linea_con_algo_es_la_que_vale() {
        assert_eq!(primera_linea("\n\n  light  \ndark\n"), Some("light"));
        assert_eq!(primera_linea("   \n\t\n"), None);
        assert_eq!(primera_linea(""), None);
    }

    #[test]
    fn un_esquema_necesita_las_dos_variantes() {
        let completo = json!({
            "colors": {
                "dark": { "ui": { "background": "#1e1e2e" } },
                "light": { "ui": { "background": "#eff1f5" } }
            }
        });
        assert!(esquema_utilizable(&completo));
    }

    #[test]
    fn un_esquema_con_una_sola_variante_no_sirve() {
        let a_medias = json!({ "colors": { "dark": { "ui": { "background": "#1e1e2e" } } } });

        assert!(
            !esquema_utilizable(&a_medias),
            "media interfaz quedaría sin color"
        );
    }

    #[test]
    fn un_esquema_sin_ui_no_sirve() {
        let sin_ui = json!({
            "colors": {
                "dark": { "terminal": {} },
                "light": { "terminal": {} }
            }
        });

        assert!(!esquema_utilizable(&sin_ui));
    }

    #[test]
    fn un_documento_vacio_no_sirve() {
        assert!(!esquema_utilizable(&json!({})));
    }

    #[test]
    fn sin_archivos_el_tema_es_oscuro() {
        // No hay `/etc/vasak-session-manager/theme` en la máquina donde corren
        // los tests, y eso es justamente el caso que se prueba: una instalación
        // que nunca configuró el inicio de sesión.
        if ruta(THEME_FILE).exists() {
            return;
        }

        assert_eq!(leer_tema(), "dark");
    }
}
