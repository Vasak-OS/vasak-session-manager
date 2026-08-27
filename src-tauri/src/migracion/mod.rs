//! Que una actualización llegue a las cuentas que ya existen, sin pisar nada.
//!
//! # El problema
//!
//! Los valores por omisión del escritorio se instalan en `/etc/skel`, que **sólo
//! alcanza a las cuentas nuevas**. Todo lo que el paquete agregue después —un atajo
//! de teclado, un plugin, una opción nueva— no llega nunca a quien ya estaba
//! usando el sistema. Medido en una máquina real: el `wayfire.ini` del usuario
//! tenía 115 asignaciones y el del paquete 97, con 42 líneas distintas.
//!
//! Las dos salidas fáciles son malas. Copiar el archivo del paquete encima borra
//! todo lo que la persona configuró. No hacer nada deja el escritorio congelado en
//! la versión en la que se instaló.
//!
//! # Cómo se resuelve
//!
//! Se **agregan** las claves que faltan y no se toca ninguna que ya esté. Tres
//! reglas, y las tres importan:
//!
//! 1. **Un valor que ya está no se cambia nunca**, aunque el paquete traiga otro.
//!    Si alguien puso `duration = 300`, queda 300.
//! 2. **Nada se borra**, ni siquiera lo que el paquete dejó de traer: puede ser
//!    algo que la persona quiere conservar, y no es nuestro decidirlo.
//! 3. **Lo que se ofreció una vez no se vuelve a ofrecer.** Sin esto, una opción
//!    borrada a propósito reaparecería en cada inicio de sesión.
//!
//! Los archivos se tocan por líneas, no parseando y reescribiendo: el `wayfire.ini`
//! del escritorio está lleno de comentarios que explican por qué cada opción está
//! donde está, y perderlos sería perder la mitad del archivo.
//!
//! # Lo que **no** hace
//!
//! No toca los archivos que no son clave/valor. `.zshrc`, `.profile` y `.xinitrc`
//! son programas, y no hay forma de fusionarlos por clave: para que sus
//! actualizaciones lleguen, lo que corresponde es que el paquete deje un `source`
//! de un archivo suyo, y eso se arregla en el paquete y no acá.
//!
//! Tampoco toca los JSON como `vasak.conf`: los escribe la aplicación de ajustes,
//! que es la que sabe qué valores por omisión le corresponden a cada versión.

pub mod estado;
pub mod ini;
pub mod lineas;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Los archivos que se fusionan, relativos al hogar.
///
/// Una lista fija y no «todo lo que haya en skel»: cada archivo entra cuando se
/// comprobó que fusionarlo por clave tiene sentido. Un `.face` o un `.zshrc`
/// fusionados por clave serían un desastre silencioso.
pub const ARCHIVOS: [&str; 4] = [
    ".config/wayfire.ini",
    ".config/gtk-3.0/settings.ini",
    ".config/gtk-4.0/settings.ini",
    ".config/Trolltech.conf",
];

// `mimeapps.list` **no** está en la lista, a propósito. Ese archivo es donde vive
// lo que la persona eligió, y los valores por omisión del sistema van en
// `/usr/share/applications/mimeapps.list`, que alcanza a todas las cuentas sin
// migrar nada. Metiéndolos en el archivo del usuario se ensucia la capa que es suya
// y encima queda pegado: si el paquete después cambia un valor por omisión, el que
// se copió una vez le gana para siempre.

/// De dónde salen los valores por omisión.
pub const SKEL: &str = "/etc/skel";

/// La sección con la que se anotan las líneas de los archivos que son programas.
///
/// Con un nombre que no puede ser una sección de un INI de verdad, para que no
/// choque con una clave que se llame igual.
pub const SECCION_DE_LINEA: &str = "@linea";

/// Qué pasó con un archivo.
#[derive(Debug, PartialEq, Eq)]
pub struct Resultado {
    pub archivo: String,
    pub agregadas: Vec<(String, String)>,
    pub motivo: Option<String>,
}

impl Resultado {
    fn saltado(archivo: &str, motivo: &str) -> Self {
        Self {
            archivo: archivo.to_string(),
            agregadas: Vec::new(),
            motivo: Some(motivo.to_string()),
        }
    }
}

/// Fusiona un archivo y devuelve el texto nuevo, o `None` si no hay nada que
/// cambiar.
///
/// Pura: no toca el disco. Lo que la hace probable, y lo que permite el modo de
/// prueba que no escribe nada.
pub fn fusionar_texto(
    relativo: &str,
    usuario: &str,
    paquete: &str,
    ya: &HashSet<estado::Clave>,
) -> (Option<String>, Vec<(String, String)>) {
    let ofrecida = |seccion: &str, clave: &str| {
        ya.contains(&(relativo.to_string(), seccion.to_string(), clave.to_string()))
    };
    let (texto, agregado) = ini::fusionar(usuario, paquete, &ofrecida);
    if agregado.claves.is_empty() {
        return (None, Vec::new());
    }
    (Some(texto), agregado.claves)
}

/// Recorre los archivos y aplica lo que falte.
///
/// `escribir` en `false` no toca nada y sirve para ver qué haría. El registro de lo
/// ofrecido se actualiza **sólo** si se escribió: anotar sin aplicar haría que la
/// opción no se ofreciera nunca más sin haber llegado jamás.
///
/// La ruta del registro se recibe en lugar de deducirse acá. Deducirla adentro
/// ataba la lógica a una variable de entorno del proceso, y con eso dos hogares
/// distintos compartían registro — que es exactamente lo que hizo fallar una
/// prueba y lo que en producción pasaría con dos sesiones anidadas.
pub fn aplicar(
    hogar: &Path,
    skel: &Path,
    ruta_estado: &Path,
    escribir: bool,
) -> Vec<Resultado> {
    // Un error al leer el registro **no** es un registro vacío.
    //
    // Con `unwrap_or_default()`, un `PermissionDenied` o un `EIO` se volvían un
    // conjunto vacío, y entonces todo lo que alguien había borrado a propósito se
    // le volvía a agregar. Eso es justo lo contrario del prometido. Que no exista
    // sí es un registro vacío —es la primera vez—, cualquier otra cosa es motivo
    // para no tocar nada.
    let mut resultados = Vec::new();
    let mut ya = match std::fs::read_to_string(ruta_estado) {
        Ok(texto) => estado::leer(&texto),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => HashSet::new(),
        Err(e) => {
            resultados.push(Resultado::saltado(
                estado::NOMBRE,
                &format!("no se pudo leer el registro, no se toca nada: {e}"),
            ));
            return resultados;
        }
    };
    let mut hubo_cambios = false;

    for relativo in ARCHIVOS {
        let del_paquete = skel.join(relativo);
        let del_usuario = hogar.join(relativo);

        let Ok(paquete) = std::fs::read_to_string(&del_paquete) else {
            resultados.push(Resultado::saltado(relativo, "el paquete no lo trae"));
            continue;
        };
        // Si la cuenta no lo tiene, no se inventa: puede que no use ese componente,
        // y crearle configuración que no pidió es justo lo contrario de esto.
        let Ok(usuario) = std::fs::read_to_string(&del_usuario) else {
            resultados.push(Resultado::saltado(relativo, "la cuenta no lo tiene"));
            continue;
        };

        let (nuevo, agregadas) = fusionar_texto(relativo, &usuario, &paquete, &ya);
        let Some(nuevo) = nuevo else {
            resultados.push(Resultado {
                archivo: relativo.to_string(),
                agregadas: Vec::new(),
                motivo: None,
            });
            continue;
        };

        if escribir {
            if let Err(e) = escritura_atomica(&del_usuario, &nuevo) {
                resultados.push(Resultado::saltado(relativo, &format!("no se pudo escribir: {e}")));
                continue;
            }
            for (seccion, clave) in &agregadas {
                ya.insert((relativo.to_string(), seccion.clone(), clave.clone()));
            }
            hubo_cambios = true;
        }

        resultados.push(Resultado {
            archivo: relativo.to_string(),
            agregadas,
            motivo: None,
        });
    }

    // Los archivos que son programas: una sola línea que carga el del paquete. Ver
    // `lineas`. La marca del registro usa la sección `@linea` para que no choque
    // con una clave de un INI que se llame igual.
    for insercion in lineas::INSERCIONES {
        let del_usuario = hogar.join(insercion.archivo);
        let Ok(usuario) = std::fs::read_to_string(&del_usuario) else {
            resultados.push(Resultado::saltado(insercion.archivo, "la cuenta no lo tiene"));
            continue;
        };

        let marca = (
            insercion.archivo.to_string(),
            SECCION_DE_LINEA.to_string(),
            insercion.marca.to_string(),
        );
        if ya.contains(&marca) {
            continue;
        }

        let Some(nuevo) = lineas::asegurar(&usuario, &insercion) else {
            resultados.push(Resultado {
                archivo: insercion.archivo.to_string(),
                agregadas: Vec::new(),
                motivo: None,
            });
            continue;
        };

        if escribir {
            if let Err(e) = escritura_atomica(&del_usuario, &nuevo) {
                resultados.push(Resultado::saltado(
                    insercion.archivo,
                    &format!("no se pudo escribir: {e}"),
                ));
                continue;
            }
            ya.insert(marca);
            hubo_cambios = true;
        }

        resultados.push(Resultado {
            archivo: insercion.archivo.to_string(),
            agregadas: vec![(SECCION_DE_LINEA.to_string(), insercion.marca.to_string())],
            motivo: None,
        });
    }

    if escribir && hubo_cambios {
        if let Some(dir) = ruta_estado.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = escritura_atomica(ruta_estado, &estado::escribir(&ya));
    }

    resultados
}

/// Escribe a un temporal y renombra.
///
/// La configuración del escritorio se toca **mientras la sesión arranca**: si el
/// proceso se muere a mitad de un `write`, el archivo queda truncado y el
/// compositor arranca sin configuración o no arranca. El renombrado es atómico
/// dentro del mismo sistema de archivos, así que o está el viejo o está el nuevo.
fn escritura_atomica(destino: &Path, contenido: &str) -> std::io::Result<()> {
    let temporal = con_sufijo(destino, ".vasak-nuevo");
    std::fs::write(&temporal, contenido)?;
    std::fs::rename(&temporal, destino)
}

fn con_sufijo(ruta: &Path, sufijo: &str) -> PathBuf {
    let mut nombre = ruta.file_name().unwrap_or_default().to_os_string();
    nombre.push(sufijo);
    ruta.with_file_name(nombre)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escenario(quien: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("migracion-{}-{quien}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let hogar = base.join("hogar");
        let skel = base.join("skel");
        for d in [&hogar, &skel] {
            std::fs::create_dir_all(d.join(".config")).unwrap();
        }
        (hogar, skel, base.join("estado.tsv"))
    }

    fn poner(dir: &Path, relativo: &str, contenido: &str) {
        let ruta = dir.join(relativo);
        std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
        std::fs::write(ruta, contenido).unwrap();
    }

    #[test]
    fn el_modo_de_prueba_no_toca_nada() {
        // Es lo que permite ver qué haría antes de dejarlo suelto en cada inicio de
        // sesión.
        let (hogar, skel, registro) = escenario("prueba");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nduration = 150\nopen_animation = fade\n");

        let r = aplicar(&hogar, &skel, &registro, false);
        let wayfire = r.iter().find(|x| x.archivo == ".config/wayfire.ini").unwrap();
        assert_eq!(wayfire.agregadas.len(), 1);

        // El archivo sigue igual y no se creó el registro.
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert_eq!(despues, "[animate]\nduration = 300\n");
        assert!(!registro.exists());
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn aplicar_agrega_y_anota() {
        let (hogar, skel, registro) = escenario("aplicar");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nduration = 150\nopen_animation = fade\n");

        aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert!(despues.contains("duration = 300"), "el valor de la persona queda");
        assert!(despues.contains("open_animation = fade"), "y lo nuevo entra");

        // Y una segunda pasada no vuelve a agregar nada.
        let r2 = aplicar(&hogar, &skel, &registro, true);
        assert!(r2.iter().all(|x| x.agregadas.is_empty()), "{r2:?}");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn lo_borrado_a_proposito_no_vuelve() {
        // El caso que define todo el diseño: si esto falla, el escritorio insiste
        // para siempre con una opción que alguien sacó.
        let (hogar, skel, registro) = escenario("borrado");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nduration = 150\nopen_animation = fade\n");

        aplicar(&hogar, &skel, &registro, true);

        // La persona la borra.
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");

        let r = aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert!(!despues.contains("open_animation"), "volvió: {despues}");
        assert!(r.iter().all(|x| x.agregadas.is_empty()));
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn no_se_le_crea_configuracion_que_no_tenia() {
        // Puede que no use ese componente; crearle el archivo es lo contrario de
        // «sin pisar».
        let (hogar, skel, registro) = escenario("sin-archivo");
        poner(&skel, ".config/wayfire.ini", "[core]\nplugins = expo\n");

        aplicar(&hogar, &skel, &registro, true);
        assert!(!hogar.join(".config/wayfire.ini").exists());
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn un_archivo_que_el_paquete_no_trae_se_saltea() {
        let (hogar, skel, registro) = escenario("sin-skel");
        poner(&hogar, ".config/wayfire.ini", "[core]\nplugins = expo\n");

        let r = aplicar(&hogar, &skel, &registro, true);
        let wayfire = r.iter().find(|x| x.archivo == ".config/wayfire.ini").unwrap();
        assert!(wayfire.motivo.as_deref() == Some("el paquete no lo trae"), "{wayfire:?}");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn no_se_anota_lo_que_no_se_escribio() {
        // Anotar sin aplicar haría que la opción no se ofreciera nunca más sin
        // haber llegado jamás: el peor de los dos mundos.
        let (hogar, skel, registro) = escenario("no-anota");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nopen_animation = fade\n");

        aplicar(&hogar, &skel, &registro, false);
        assert!(!registro.exists());

        // Y aplicando de verdad después, sí llega.
        aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert!(despues.contains("open_animation = fade"));
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn la_escritura_no_deja_el_archivo_a_medias() {
        // Esto corre mientras la sesión arranca: un archivo truncado deja al
        // compositor sin configuración. Se comprueba que no quede el temporal
        // suelto, que es la señal de que el renombrado se hizo.
        let (hogar, skel, registro) = escenario("atomica");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nopen_animation = fade\n");

        aplicar(&hogar, &skel, &registro, true);
        let sobrantes: Vec<_> = std::fs::read_dir(hogar.join(".config"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("vasak-nuevo"))
            .collect();
        assert!(sobrantes.is_empty(), "quedó un temporal: {sobrantes:?}");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn mimeapps_no_se_migra() {
        // Es donde vive lo que la persona eligió. Los valores por omisión del
        // sistema van en `/usr/share/applications/mimeapps.list`, que alcanza a
        // todas las cuentas sin migrar nada; copiarlos al archivo del usuario los
        // deja pegados y le ganan para siempre a los del paquete.
        assert!(!ARCHIVOS.iter().any(|a| a.contains("mimeapps")));
    }

    #[test]
    fn la_lista_de_archivos_es_solo_clave_valor() {
        // Un `.zshrc` o un `.face` fusionados por clave serían un desastre
        // silencioso: el primero es un programa y el segundo una imagen.
        for a in ARCHIVOS {
            assert!(
                a.ends_with(".ini") || a.ends_with(".conf"),
                "{a} no parece un archivo de clave/valor"
            );
            assert!(!a.starts_with('/'), "{a} tiene que ser relativo al hogar");
            assert!(!a.contains(".."), "{a}");
        }
    }

    #[test]
    fn los_archivos_que_son_programas_reciben_la_linea() {
        // No se fusionan por clave —son programas— así que lo único que se asegura
        // es la línea que carga el archivo del paquete.
        let (hogar, skel, registro) = escenario("lineas");
        poner(&hogar, ".zshrc", "export EDITOR=vim\n");

        aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".zshrc")).unwrap();
        assert!(despues.contains("/usr/share/vasak/shell/zshrc"), "{despues}");
        assert!(despues.contains("export EDITOR=vim"), "no se perdió lo suyo");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn la_linea_quitada_a_proposito_no_vuelve() {
        // Igual que con las claves: si alguien la sacó, se respeta.
        let (hogar, skel, registro) = escenario("linea-quitada");
        poner(&hogar, ".zshrc", "export EDITOR=vim\n");

        aplicar(&hogar, &skel, &registro, true);
        poner(&hogar, ".zshrc", "export EDITOR=vim\n");

        aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".zshrc")).unwrap();
        assert!(!despues.contains("/usr/share/vasak/shell/zshrc"), "volvió: {despues}");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn no_se_le_crea_un_zshrc_a_quien_no_lo_tiene() {
        // Puede no usar zsh; crearle el archivo sería inventarle configuración.
        let (hogar, skel, registro) = escenario("sin-zshrc");
        aplicar(&hogar, &skel, &registro, true);
        assert!(!hogar.join(".zshrc").exists());
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn la_seccion_de_linea_no_puede_confundirse_con_una_de_ini() {
        // Si se llamara como una sección real, una clave homónima haría que la
        // línea no se agregara nunca, o al revés.
        assert!(SECCION_DE_LINEA.starts_with('@'));
        assert!(!ARCHIVOS.iter().any(|a| a.contains(SECCION_DE_LINEA)));
    }

    #[test]
    fn un_registro_ilegible_hace_que_no_se_toque_nada() {
        // Con `unwrap_or_default()`, un error de permisos se volvía un registro
        // vacío y todo lo que alguien había borrado a propósito se le volvía a
        // agregar: lo contrario de lo que este módulo promete.
        let (hogar, skel, _) = escenario("registro-ilegible");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nopen_animation = fade\n");

        // Un directorio donde va el archivo: leerlo da `IsADirectory`, no
        // `NotFound`, que es lo que distingue «primera vez» de «algo anda mal».
        let registro = hogar.join("registro-que-es-directorio");
        std::fs::create_dir_all(&registro).unwrap();

        let r = aplicar(&hogar, &skel, &registro, true);
        assert!(
            r.iter().any(|x| x.motivo.as_deref().is_some_and(|m| m.contains("registro"))),
            "{r:?}"
        );
        // Y el archivo quedó intacto.
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert_eq!(despues, "[animate]\nduration = 300\n");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }

    #[test]
    fn la_primera_vez_no_es_un_error() {
        // Que el registro no exista es lo normal la primera vez, y ahí sí hay que
        // aplicar.
        let (hogar, skel, registro) = escenario("primera-vez");
        poner(&hogar, ".config/wayfire.ini", "[animate]\nduration = 300\n");
        poner(&skel, ".config/wayfire.ini", "[animate]\nopen_animation = fade\n");
        assert!(!registro.exists());

        aplicar(&hogar, &skel, &registro, true);
        let despues = std::fs::read_to_string(hogar.join(".config/wayfire.ini")).unwrap();
        assert!(despues.contains("open_animation = fade"), "{despues}");
        let _ = std::fs::remove_dir_all(hogar.parent().unwrap());
    }
}
