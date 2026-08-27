//! Asegurar que una línea esté en un archivo que es un programa.
//!
//! `.zshrc`, `.profile` y `.xinitrc` no se pueden fusionar por clave: son
//! programas, y el orden y las condiciones importan. Medido en una máquina real: el
//! `.zshrc` del usuario tenía 26 asignaciones contra 1 del paquete, con 118 líneas
//! distintas. No hay forma de mezclarlos sin romper algo.
//!
//! La salida es no mezclarlos. El paquete pone su contenido en un archivo **suyo**,
//! bajo `/usr/share`, y en el del usuario va una sola línea que lo carga. Así una
//! versión nueva del escritorio cambia su archivo y llega a todo el mundo sin tocar
//! el del usuario nunca más. Lo único que hay que asegurar es esa línea.
//!
//! Va **arriba** y no al final, después del shebang: lo que la persona escribió
//! viene después y por lo tanto gana. Al final, el archivo del paquete pisaría lo
//! que ella puso, que es exactamente lo que no queremos.

/// Una línea que tiene que estar en un archivo del usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insercion {
    /// El archivo, relativo al hogar.
    pub archivo: &'static str,
    /// Con qué se reconoce que ya está.
    ///
    /// Una marca corta y estable, no el texto entero: así se puede mejorar el
    /// comentario que la acompaña sin que la línea se agregue de nuevo.
    pub marca: &'static str,
    /// Lo que se agrega, comentario incluido.
    pub texto: &'static str,
}

/// Las inserciones que el escritorio necesita.
pub const INSERCIONES: [Insercion; 2] = [
    Insercion {
        archivo: ".zshrc",
        marca: "/usr/share/vasak/shell/zshrc",
        texto: "# La configuración de zsh que trae VasakOS. Se carga desde acá para que las\n\
                # mejoras del escritorio lleguen sin tocar este archivo, que es tuyo. Lo que\n\
                # escribas debajo gana, porque corre después.\n\
                [ -r /usr/share/vasak/shell/zshrc ] && . /usr/share/vasak/shell/zshrc\n",
    },
    Insercion {
        archivo: ".profile",
        marca: "/usr/share/vasak/shell/profile",
        texto: "# El entorno que trae VasakOS. Se carga desde acá para que las mejoras del\n\
                # escritorio lleguen sin tocar este archivo, que es tuyo. Lo que escribas\n\
                # debajo gana, porque corre después.\n\
                [ -r /usr/share/vasak/shell/profile ] && . /usr/share/vasak/shell/profile\n",
    },
];

/// Si el archivo ya carga lo del paquete.
///
/// Se busca la marca en cualquier parte, **comentarios incluidos**: si alguien la
/// dejó comentada, es que no la quiere, y volver a agregarla sería insistir.
pub fn ya_esta(usuario: &str, marca: &str) -> bool {
    usuario.contains(marca)
}

/// Agrega la línea arriba del archivo, o `None` si ya estaba.
///
/// Después del shebang, si hay: una línea antes del `#!` lo deja de ser y el
/// archivo pasa a ejecutarse con otra shell.
pub fn asegurar(usuario: &str, insercion: &Insercion) -> Option<String> {
    if ya_esta(usuario, insercion.marca) {
        return None;
    }

    // Los terminadores del archivo se conservan, igual que en `ini`: `str::lines()`
    // se come tanto `\n` como `\r\n`, y reconstruir con `\n` a secas convierte un
    // archivo con terminadores de Windows entero. Y el salto final se deja como
    // estaba: agregarle uno es cambiar el archivo por algo que nadie pidió.
    let terminador = super::ini::terminador_de(usuario);
    let mut lineas: Vec<&str> = usuario.lines().collect();
    let donde = usize::from(lineas.first().is_some_and(|l| l.starts_with("#!")));

    let nuevas: Vec<&str> = insercion.texto.lines().collect();
    for (i, l) in nuevas.iter().enumerate() {
        lineas.insert(donde + i, l);
    }
    // Una línea en blanco para separar lo nuestro de lo que ya estaba.
    if lineas.get(donde + nuevas.len()).is_some_and(|l| !l.trim().is_empty()) {
        lineas.insert(donde + nuevas.len(), "");
    }

    let mut texto = lineas.join(terminador);
    if usuario.ends_with('\n') || usuario.is_empty() {
        texto.push_str(terminador);
    }
    Some(texto)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zshrc() -> &'static Insercion {
        INSERCIONES.iter().find(|i| i.archivo == ".zshrc").unwrap()
    }

    #[test]
    fn la_linea_se_agrega_arriba_asi_lo_de_la_persona_gana() {
        // Al final, el archivo del paquete pisaría lo que ella puso.
        let usuario = "export EDITOR=vim\nalias ll='ls -la'\n";
        let r = asegurar(usuario, zshrc()).expect("se agrega");
        let pos_nuestra = r.find("/usr/share/vasak/shell/zshrc").unwrap();
        let pos_suya = r.find("export EDITOR=vim").unwrap();
        assert!(pos_nuestra < pos_suya, "{r}");
    }

    #[test]
    fn el_shebang_sigue_siendo_la_primera_linea() {
        // Una línea antes del `#!` lo deja de ser, y el archivo pasa a ejecutarse
        // con otra shell.
        let usuario = "#!/bin/zsh\nexport EDITOR=vim\n";
        let r = asegurar(usuario, zshrc()).expect("se agrega");
        assert!(r.starts_with("#!/bin/zsh\n"), "{r}");
        assert!(r.contains("/usr/share/vasak/shell/zshrc"));
    }

    #[test]
    fn no_se_agrega_dos_veces() {
        // Esto corre en cada inicio de sesión: sin esto el archivo crecería una
        // copia por arranque.
        let usuario = "export EDITOR=vim\n";
        let una = asegurar(usuario, zshrc()).expect("se agrega");
        assert_eq!(asegurar(&una, zshrc()), None);
    }

    #[test]
    fn comentada_cuenta_como_puesta() {
        // Si alguien la dejó comentada es que no la quiere, y volver a agregarla
        // sería insistir con algo que ya rechazó.
        let usuario = "# [ -r /usr/share/vasak/shell/zshrc ] && . /usr/share/vasak/shell/zshrc\n";
        assert_eq!(asegurar(usuario, zshrc()), None);
    }

    #[test]
    fn nada_de_lo_que_ya_estaba_se_pierde() {
        let usuario = "#!/bin/zsh\n# mi comentario\nexport A=1\n\nexport B=2\n";
        let r = asegurar(usuario, zshrc()).expect("se agrega");
        for linea in ["# mi comentario", "export A=1", "export B=2"] {
            assert!(r.contains(linea), "se perdió {linea}: {r}");
        }
    }

    #[test]
    fn un_archivo_vacio_recibe_la_linea() {
        let r = asegurar("", zshrc()).expect("se agrega");
        assert!(r.contains("/usr/share/vasak/shell/zshrc"));
        assert!(r.ends_with('\n'));
    }

    #[test]
    fn la_marca_esta_dentro_del_texto_que_se_agrega() {
        // Si no coincidieran, la línea se agregaría en cada arranque para siempre.
        for i in INSERCIONES {
            assert!(i.texto.contains(i.marca), "{} no contiene su marca", i.archivo);
            assert!(i.texto.ends_with('\n'), "{}", i.archivo);
            assert!(!i.archivo.contains('/'), "{} tiene que estar en el hogar", i.archivo);
        }
    }

    #[test]
    fn la_linea_no_falla_si_el_archivo_del_paquete_no_esta() {
        // Un `.` sobre un archivo que no existe corta el arranque de la shell en
        // algunas configuraciones: por eso va con el `[ -r ]` adelante.
        for i in INSERCIONES {
            assert!(i.texto.contains("[ -r "), "{} sin guarda de lectura", i.archivo);
            assert!(i.texto.contains("] && ."), "{}", i.archivo);
        }
    }

    #[test]
    fn se_usa_punto_y_no_source() {
        // `.profile` lo lee cualquier shell POSIX, incluida dash, que no conoce
        // `source`. Con `source` ahí, el arranque tira un error en cada sesión.
        for i in INSERCIONES {
            assert!(!i.texto.contains("source "), "{} usa source", i.archivo);
        }
    }

    #[test]
    fn los_terminadores_de_windows_no_se_reescriben() {
        // El mismo cuidado que en `ini`: reconstruir con `\n` a secas convierte el
        // archivo entero, y este módulo también promete conservarlo.
        let usuario = "#!/bin/zsh\r\nexport EDITOR=vim\r\n";
        let r = asegurar(usuario, zshrc()).expect("se agrega");
        assert_eq!(r.matches('\n').count(), r.matches("\r\n").count(), "{r:?}");
        assert!(r.starts_with("#!/bin/zsh\r\n"), "{r:?}");
    }

    #[test]
    fn un_archivo_sin_salto_final_no_recibe_uno() {
        let r = asegurar("export EDITOR=vim", zshrc()).expect("se agrega");
        assert!(!r.ends_with('\n'), "{r:?}");
        assert!(r.contains("export EDITOR=vim"));
    }
}
