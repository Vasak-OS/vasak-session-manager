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
use tauri::webview::Color;

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
    handle
        .take(maximo + 1)
        .read_to_string(&mut contenido)
        .ok()?;

    if contenido.len() as u64 > maximo {
        eprintln!(
            "[apariencia] {} es más grande que el máximo, se ignora",
            camino.display()
        );
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

/// El fondo con el que se pinta la ventana nativa antes del primer dibujado del
/// webview, para que no se vea el blanco por omisión de WebKitGTK.
///
/// `background_color` es el color del tema del esquema, con el fondo compilado
/// de respaldo. Separado de la lectura de archivos para poder probarlo sin
/// `/etc`.
fn resolver_fondo(tema: &str, esquema: Option<&Value>) -> Color {
    esquema
        .and_then(|esquema| esquema["colors"][tema]["ui"]["background"].as_str())
        .and_then(hex_a_color)
        .unwrap_or_else(|| fondo_compilado(tema))
}

/// Fondo nativo del greeter. El greeter corre sin sesión, así que su tema sale
/// de `/etc`, igual que `get_appearance`.
pub fn greeter_background_color() -> Color {
    resolver_fondo(&leer_tema(), leer_esquema().as_ref())
}

/// Fondo nativo del bloqueo. La página del lock **no** usa el tema de `/etc`:
/// `main.ts` la arranca con la clase `dark` y recién después el gestor de
/// configuración del usuario aplica su esquema, así que el primer cuadro —el
/// hueco que el fondo nativo tapa— siempre es oscuro. Por eso el nativo es el
/// oscuro compilado y no `/etc`: con `/etc` en `light` arrancaría casi blanco
/// bajo una sesión oscura.
pub fn lock_background_color() -> Color {
    fondo_compilado("dark")
}

/// `#rrggbb` a color opaco. Los esquemas guardan el fondo así; cualquier otra
/// forma (nombre, `rgba()`, longitud rara) cae al fondo compilado.
fn hex_a_color(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    // `len()` cuenta bytes: sin el filtro ASCII, un fondo con un carácter
    // multibyte haría que `&h[0..2]` cortara a mitad de carácter y entrara en
    // pánico, tumbando el greeter y el lock en el arranque.
    if !h.is_ascii() || h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color(r, g, b, 255))
}

/// La misma paleta que trae compilada `style.css` (`--ui-background`), para
/// cuando no hay esquema en `/etc`.
fn fondo_compilado(tema: &str) -> Color {
    if tema == "light" {
        Color(0xef, 0xf1, 0xf5, 255)
    } else {
        Color(0x1e, 0x1e, 0x2e, 255)
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
        assert_eq!(
            tema_valido("  Dark\n"),
            Some("dark"),
            "espacios y mayúsculas"
        );
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

    #[test]
    fn un_hex_valido_se_vuelve_color_opaco() {
        assert_eq!(hex_a_color("#1e1e2e"), Some(Color(0x1e, 0x1e, 0x2e, 255)));
        assert_eq!(hex_a_color("#EFF1F5"), Some(Color(0xef, 0xf1, 0xf5, 255)));
    }

    #[test]
    fn un_hex_mal_formado_no_da_color() {
        assert_eq!(hex_a_color("1e1e2e"), None, "sin #");
        assert_eq!(hex_a_color("#fff"), None, "corto");
        assert_eq!(hex_a_color("#1e1e2e80"), None, "con alfa");
        assert_eq!(hex_a_color("#1e1e2g"), None, "dígito no hex");
        assert_eq!(hex_a_color(""), None);
    }

    #[test]
    fn un_hex_con_caracteres_multibyte_no_hace_panico() {
        // seis bytes, pero un corte a mitad de la «é»
        assert_eq!(hex_a_color("#aéaaa"), None);
    }

    #[test]
    fn el_fondo_compilado_sigue_al_tema() {
        assert_eq!(fondo_compilado("light"), Color(0xef, 0xf1, 0xf5, 255));
        assert_eq!(fondo_compilado("dark"), Color(0x1e, 0x1e, 0x2e, 255));
        assert_eq!(fondo_compilado("otro"), Color(0x1e, 0x1e, 0x2e, 255));
    }

    #[test]
    fn el_fondo_sale_del_esquema_segun_el_tema_activo() {
        let esquema = json!({
            "colors": {
                "dark": { "ui": { "background": "#101010" } },
                "light": { "ui": { "background": "#fafafa" } }
            }
        });

        assert_eq!(
            resolver_fondo("dark", Some(&esquema)),
            Color(0x10, 0x10, 0x10, 255)
        );
        assert_eq!(
            resolver_fondo("light", Some(&esquema)),
            Color(0xfa, 0xfa, 0xfa, 255)
        );
    }

    #[test]
    fn sin_esquema_o_con_fondo_roto_cae_al_compilado() {
        assert_eq!(resolver_fondo("dark", None), Color(0x1e, 0x1e, 0x2e, 255));
        assert_eq!(resolver_fondo("light", None), Color(0xef, 0xf1, 0xf5, 255));

        // Esquema presente pero con el fondo mal escrito: el respaldo, no un pánico.
        let roto = json!({ "colors": { "dark": { "ui": { "background": "rojo" } } } });
        assert_eq!(
            resolver_fondo("dark", Some(&roto)),
            Color(0x1e, 0x1e, 0x2e, 255)
        );
    }

    #[test]
    fn el_bloqueo_siempre_arranca_oscuro() {
        // La página del lock fuerza la clase `dark` antes de aplicar el tema del
        // usuario, así que el fondo nativo tiene que ser el oscuro y no `/etc`.
        assert_eq!(lock_background_color(), Color(0x1e, 0x1e, 0x2e, 255));
    }

    #[test]
    fn el_greeter_sin_config_cae_al_oscuro() {
        // Mismo caso que `sin_archivos_el_tema_es_oscuro`: sin los archivos de
        // `/etc`, el greeter arranca con el oscuro compilado.
        if ruta(THEME_FILE).exists() || ruta(SCHEME_FILE).exists() {
            return;
        }

        assert_eq!(greeter_background_color(), Color(0x1e, 0x1e, 0x2e, 255));
    }
}
