//! Qué claves ya se le ofrecieron a esta cuenta.
//!
//! Es lo que separa «actualizar sin pisar» de «insistir para siempre». Sin este
//! registro, una opción que la persona **borró a propósito** volvería a aparecer en
//! el próximo inicio de sesión, y en el siguiente, y no habría forma de sacársela
//! de encima salvo dejar de usar el escritorio.
//!
//! El formato es una línea por clave, con tabuladores: legible, `grep`-eable y
//! fácil de editar a mano si alguien quiere que una opción se le vuelva a ofrecer.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// El nombre del archivo dentro del directorio de estado.
pub const NOMBRE: &str = "config-aplicada.tsv";

/// Una clave ya ofrecida: de qué archivo, de qué sección y cuál.
pub type Clave = (String, String, String);

/// Dónde vive el registro.
///
/// En `XDG_STATE_HOME` y no en `~/.config`: esto no es configuración que alguien
/// vaya a editar para cambiar cómo se ve el escritorio, es estado del programa. Y
/// si va en `~/.config`, un respaldo de la configuración se lo lleva y al
/// restaurarlo en otra máquina se saltearían opciones que nunca se ofrecieron ahí.
pub fn ruta(hogar: &Path) -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| hogar.join(".local/state"))
        .join("vasak")
        .join(NOMBRE)
}

/// Lee el registro. Un archivo que no existe es un registro vacío.
pub fn leer(texto: &str) -> HashSet<Clave> {
    texto
        .lines()
        .filter_map(|linea| {
            let mut campos = linea.split('\t');
            let archivo = campos.next()?.trim();
            let seccion = campos.next()?.trim();
            let clave = campos.next()?.trim();
            (!archivo.is_empty() && !clave.is_empty()).then(|| {
                (archivo.to_string(), seccion.to_string(), clave.to_string())
            })
        })
        .collect()
}

/// Serializa el registro, ordenado para que el archivo no cambie de orden entre
/// arranques y un `diff` sirva para algo.
pub fn escribir(claves: &HashSet<Clave>) -> String {
    let mut ordenadas: Vec<&Clave> = claves.iter().collect();
    ordenadas.sort();
    let mut texto = String::from(
        "# Claves que VasakOS ya le ofreció a esta cuenta.\n\
         # Se anotan para no volver a agregar lo que se borró a propósito.\n\
         # Borrar una línea hace que esa opción se vuelva a ofrecer.\n",
    );
    for (archivo, seccion, clave) in ordenadas {
        texto.push_str(&format!("{archivo}\t{seccion}\t{clave}\n"));
    }
    texto
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_lee_lo_que_se_escribe() {
        let mut claves = HashSet::new();
        claves.insert((".config/wayfire.ini".into(), "animate".into(), "duration".into()));
        claves.insert((".config/wayfire.ini".into(), "core".into(), "plugins".into()));
        claves.insert((".config/mimeapps.list".into(), String::new(), "x".into()));

        assert_eq!(leer(&escribir(&claves)), claves);
    }

    #[test]
    fn los_comentarios_no_son_claves() {
        // El archivo se explica solo arriba; esas líneas no son claves.
        let leidas = leer(&escribir(&HashSet::new()));
        assert!(leidas.is_empty(), "{leidas:?}");
    }

    #[test]
    fn una_linea_incompleta_se_saltea_sin_llevarse_el_resto() {
        // Un archivo cortado a la mitad —el disco se llenó, la sesión se cayó— no
        // puede hacer que se reofrezca todo lo que ya se había ofrecido.
        let texto = ".config/a.ini\tsec\tclave\nlinea-sin-tabuladores\n.config/b.ini\ts\tk\n";
        let leidas = leer(texto);
        assert_eq!(leidas.len(), 2, "{leidas:?}");
    }

    #[test]
    fn el_orden_es_estable() {
        // Si no, el archivo cambia de orden en cada arranque y un `diff` no sirve
        // para ver qué se agregó.
        let mut claves = HashSet::new();
        for i in 0..20 {
            claves.insert((format!("a{i}.ini"), "s".into(), format!("k{i}")));
        }
        assert_eq!(escribir(&claves), escribir(&claves));
    }

    #[test]
    fn una_seccion_vacia_se_conserva() {
        // `mimeapps.list` tiene asignaciones sin sección, y la cadena vacía es una
        // sección legítima: si se perdiera, esas claves se reofrecerían siempre.
        let mut claves = HashSet::new();
        claves.insert((".config/mimeapps.list".into(), String::new(), "x".into()));
        assert_eq!(leer(&escribir(&claves)), claves);
    }

    #[test]
    fn el_estado_no_va_en_config() {
        // En `~/.config` un respaldo de la configuración se lo llevaría, y al
        // restaurarlo en otra máquina se saltearían opciones que nunca se
        // ofrecieron ahí.
        let hogar = Path::new("/home/quien");
        let r = ruta(hogar);
        let como_texto = r.to_string_lossy();
        assert!(!como_texto.contains("/.config/"), "{como_texto}");
        assert!(como_texto.ends_with(NOMBRE));
    }

    #[test]
    fn sin_xdg_state_home_se_usa_el_lugar_de_la_especificacion() {
        // Con la variable puesta a algo relativo tampoco: una ruta relativa haría
        // que el estado dependiera del directorio desde donde arrancó la sesión.
        let hogar = Path::new("/home/quien");
        let previo = std::env::var_os("XDG_STATE_HOME");
        // SEGURO: la prueba corre en un solo hilo por el candado de abajo.
        let _guardia = GUARDIA.lock().unwrap();
        unsafe { std::env::remove_var("XDG_STATE_HOME") };
        assert_eq!(ruta(hogar), PathBuf::from("/home/quien/.local/state/vasak").join(NOMBRE));

        unsafe { std::env::set_var("XDG_STATE_HOME", "relativa/nope") };
        assert_eq!(ruta(hogar), PathBuf::from("/home/quien/.local/state/vasak").join(NOMBRE));

        match previo {
            Some(v) => unsafe { std::env::set_var("XDG_STATE_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_STATE_HOME") },
        }
    }

    /// El entorno es global al proceso, así que las pruebas que lo tocan van de a una.
    static GUARDIA: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
