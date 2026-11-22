//! El color con el que se pinta la ventana nativa antes del primer dibujado del
//! webview.
//!
//! Sin esto, WebKitGTK muestra su fondo blanco por omisión hasta que la página
//! pinta: un flash blanco de milisegundos en una pantalla que casi siempre está
//! sobre un fondo oscuro. Fijando el fondo nativo al color del tema, el primer
//! cuadro ya tiene el color correcto en vez de blanco.
//!
//! La selección del color vive acá, separada de la lectura de archivos (que
//! queda en `commands::appearance`), para poder probarla sin tocar `/etc`.

use serde_json::Value;
use tauri::webview::Color;

/// El fondo del tema, sacado del esquema; si no hay esquema o el color está mal
/// escrito, la paleta compilada de respaldo.
pub fn resolver(tema: &str, esquema: Option<&Value>) -> Color {
    esquema
        .and_then(|esquema| esquema["colors"][tema]["ui"]["background"].as_str())
        .and_then(hex_a_color)
        .unwrap_or_else(|| fondo_compilado(tema))
}

/// Fondo nativo del bloqueo. La página del lock **no** usa el tema de `/etc`:
/// `main.ts` la arranca con la clase `dark` y recién después el gestor de
/// configuración del usuario aplica su esquema, así que el primer cuadro —el
/// hueco que el fondo nativo tapa— siempre es oscuro. Por eso es el oscuro
/// compilado y no `/etc`: con `/etc` en `light` bajo una sesión oscura
/// arrancaría casi blanco.
pub fn lock() -> Color {
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
            resolver("dark", Some(&esquema)),
            Color(0x10, 0x10, 0x10, 255)
        );
        assert_eq!(
            resolver("light", Some(&esquema)),
            Color(0xfa, 0xfa, 0xfa, 255)
        );
    }

    #[test]
    fn sin_esquema_o_con_fondo_roto_cae_al_compilado() {
        assert_eq!(resolver("dark", None), Color(0x1e, 0x1e, 0x2e, 255));
        assert_eq!(resolver("light", None), Color(0xef, 0xf1, 0xf5, 255));

        // Esquema presente pero con el fondo mal escrito: el respaldo, no un pánico.
        let roto = json!({ "colors": { "dark": { "ui": { "background": "rojo" } } } });
        assert_eq!(resolver("dark", Some(&roto)), Color(0x1e, 0x1e, 0x2e, 255));
    }

    #[test]
    fn el_bloqueo_siempre_arranca_oscuro() {
        assert_eq!(lock(), Color(0x1e, 0x1e, 0x2e, 255));
    }
}
