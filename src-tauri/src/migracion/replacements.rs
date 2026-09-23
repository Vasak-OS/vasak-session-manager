//! Cambiar un valor que puso el paquete, y sólo ése.
//!
//! La regla de toda la migración es que **un valor que ya está no se cambia
//! nunca**, y sigue valiendo. Pero tiene un agujero: cuando el paquete cambia un
//! valor por omisión, la cuenta que ya existe se queda con el viejo para siempre.
//!
//! Con un atajo de teclado eso es peor que quedarse atrás. La desplegable pasó de
//! `KEY_F12` a `<super> <alt> KEY_T`, y el paquete además agregó una segunda
//! combinación: sin esto, la máquina termina con **las dos** —el F12 que ya tenía
//! y la combinación nueva que entró como clave nueva—, que no es ninguno de los
//! dos estados que alguien quiso.
//!
//! # Qué lo hace seguro
//!
//! El reemplazo se aplica **sólo si el valor actual es exactamente el que la
//! versión anterior del paquete traía**. Eso es lo que distingue «esto lo puso el
//! paquete» de «esto lo eligió la persona»: si lo cambió a cualquier otra cosa, es
//! suyo y no se toca. No hay forma de saberlo mirando el archivo —nadie anota
//! quién escribió cada línea— y ésta es la aproximación que no se equivoca en la
//! dirección cara.
//!
//! Tres cosas más, y las tres son la diferencia entre migrar y atropellar:
//!
//! 1. **No se repite.** Se anota en el mismo registro que las claves ofrecidas. Si
//!    después alguien vuelve a poner `KEY_F12` a propósito, se queda en F12.
//! 2. **No se pisa un atajo que ya está en uso.** Si el valor nuevo ya está ligado
//!    a otra cosa en ese archivo, no se reemplaza: quedarían dos claves con el
//!    mismo combo, y wayfire dispara una sola —la primera— sin avisar de nada.
//! 3. **Sólo la clave nombrada, en su sección.** No es una búsqueda y reemplazo
//!    sobre el archivo: `KEY_F12` puede estar en otra clave por un motivo que no
//!    tiene nada que ver.

use std::collections::HashSet;

use super::estado;
use super::ini;

/// La sección con la que se anotan los reemplazos en el registro.
///
/// Igual que `SECCION_DE_LINEA`, con un nombre que no puede ser una sección de un
/// INI de verdad para que no choque con una clave que se llame igual.
pub const REPLACEMENT_SECTION: &str = "@reemplazo";

/// Un valor del paquete que cambió de una versión a otra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// El archivo, relativo al hogar.
    pub archivo: &'static str,
    pub seccion: &'static str,
    pub clave: &'static str,
    /// El valor que traía el paquete antes. **Sólo se reemplaza si es éste.**
    pub anterior: &'static str,
    /// El que trae ahora.
    pub nuevo: &'static str,
}

/// Lo que se reemplazó, para poder informarlo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aplicado {
    pub seccion: String,
    pub clave: String,
    pub anterior: String,
    pub nuevo: String,
}

/// Los reemplazos que el escritorio necesita.
///
/// La lista es corta a propósito: cada entrada es un permiso explícito para tocar
/// algo de otra persona, y se agrega cuando el valor viejo **estorba**, no cada
/// vez que un valor por omisión cambia. Un `duration` que quedó en el número
/// anterior no molesta a nadie; un atajo que quedó en una tecla que ahora usa otra
/// cosa, sí.
pub const REPLACEMENTS: [Replacement; 1] = [
    // La terminal desplegable sale de F12, que es la consola de media docena de
    // juegos, el menú de arranque de muchas máquinas y la captura de Steam.
    Replacement {
        archivo: ".config/wayfire.ini",
        seccion: "command",
        clave: "binding_terminal_overlay",
        anterior: "KEY_F12",
        nuevo: "<super> <alt> KEY_T",
    },
];

/// Cómo se anota en el registro que este reemplazo ya se hizo.
///
/// Lleva el valor viejo adentro y no sólo la clave: el día que el mismo atajo
/// cambie por segunda vez, el reemplazo nuevo es otra marca y se aplica, en vez de
/// verse como uno que ya se hizo.
pub fn marca(r: &Replacement) -> estado::Clave {
    (
        r.archivo.to_string(),
        REPLACEMENT_SECTION.to_string(),
        format!("{}@{}", r.clave, r.anterior),
    )
}

/// Si dos valores son el mismo.
///
/// Para un atajo se comparan los combos normalizados —`<super> KEY_T` y
/// `KEY_T <super>` son el mismo, y los modificadores van en cualquier orden y
/// pueden ir pegados—. Para lo que no es un atajo, el texto sin los espacios de
/// los bordes: ahí no hay ninguna normalización que aplicar y adivinar una sería
/// peor.
fn es_el_mismo(uno: &str, otro: &str) -> bool {
    let combos = ini::combos_de(uno);
    if !combos.is_empty() {
        return combos == ini::combos_de(otro);
    }
    uno.trim() == otro.trim()
}

/// Si el valor nuevo ya está ligado a otra clave del archivo.
///
/// Se salta la clave que estamos por cambiar: que su valor viejo esté ahí es
/// justamente el caso normal.
fn ya_esta_en_uso(texto: &str, r: &Replacement) -> bool {
    ini::asignaciones_de(texto)
        .iter()
        .filter(|a| !(a.seccion == r.seccion && a.clave == r.clave))
        .any(|a| es_el_mismo(&a.valor, r.nuevo))
}

/// Aplica los reemplazos que le tocan a un archivo.
///
/// Devuelve el texto nuevo y qué se reemplazó. Sin nada que hacer, devuelve el
/// texto tal cual y una lista vacía — el archivo no se reescribe.
///
/// Por líneas y no parseando: el `wayfire.ini` del escritorio está lleno de
/// comentarios que explican por qué cada opción está donde está, y de la línea que
/// se toca se cambia **sólo lo que hay después del `=`**, así el espaciado y lo
/// que venga en la misma línea quedan como estaban.
pub fn apply(relativo: &str, texto: &str, ya: &HashSet<estado::Clave>) -> (String, Vec<Aplicado>) {
    let mios: Vec<&Replacement> = REPLACEMENTS
        .iter()
        .filter(|r| r.archivo == relativo)
        .filter(|r| !ya.contains(&marca(r)))
        .filter(|r| ini::tiene(texto, r.seccion, r.clave))
        .filter(|r| !ya_esta_en_uso(texto, r))
        .collect();

    if mios.is_empty() {
        return (texto.to_string(), Vec::new());
    }

    let terminador = ini::terminador_de(texto);
    let mut seccion = String::new();
    let mut aplicados = Vec::new();
    let mut salida: Vec<String> = Vec::new();

    for linea in texto.lines() {
        let t = linea.trim();
        if t.starts_with('[') && t.ends_with(']') {
            seccion = t[1..t.len() - 1].trim().to_string();
            salida.push(linea.to_string());
            continue;
        }

        let cambio = mios.iter().find(|r| {
            r.seccion == seccion
                && linea.split_once('=').is_some_and(|(clave, valor)| {
                    clave.trim() == r.clave && es_el_mismo(valor, r.anterior)
                })
        });

        match cambio {
            Some(r) => {
                let (clave, valor) = linea.split_once('=').expect("lo acaba de encontrar");
                salida.push(format!("{clave}= {}", r.nuevo));
                aplicados.push(Aplicado {
                    seccion: seccion.clone(),
                    clave: r.clave.to_string(),
                    anterior: valor.trim().to_string(),
                    nuevo: r.nuevo.to_string(),
                });
            }
            None => salida.push(linea.to_string()),
        }
    }

    if aplicados.is_empty() {
        return (texto.to_string(), Vec::new());
    }

    // El salto final se conserva como estaba: agregarle uno, o comérselo, es
    // cambiar el archivo por algo que nadie pidió.
    let mut nuevo = salida.join(terminador);
    if texto.ends_with('\n') {
        nuevo.push_str(terminador);
    }
    (nuevo, aplicados)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WAYFIRE: &str = ".config/wayfire.ini";

    fn nada() -> HashSet<estado::Clave> {
        HashSet::new()
    }

    /// Un archivo con la desplegable donde la dejó el paquete viejo.
    fn como_lo_dejo_el_paquete() -> String {
        "[command]\n\
         binding_terminal = <super> KEY_T\n\
         command_terminal = vasak-terminal\n\
         # La desplegable.\n\
         binding_terminal_overlay = KEY_F12\n\
         command_terminal_overlay = vasak-terminal --overlay\n"
            .to_string()
    }

    #[test]
    fn el_valor_que_puso_el_paquete_se_cambia() {
        let (texto, hechos) = apply(WAYFIRE, &como_lo_dejo_el_paquete(), &nada());

        assert!(
            texto.contains("binding_terminal_overlay = <super> <alt> KEY_T"),
            "{texto}"
        );
        assert!(!texto.contains("KEY_F12"), "{texto}");
        assert_eq!(hechos.len(), 1);
        assert_eq!(hechos[0].anterior, "KEY_F12");
    }

    #[test]
    fn lo_que_la_persona_eligio_no_se_toca() {
        // Es la regla de la que depende todo lo demás. Quien movió la desplegable a
        // otra tecla la quiere ahí.
        let suyo = como_lo_dejo_el_paquete().replace("KEY_F12", "KEY_F11");
        let (texto, hechos) = apply(WAYFIRE, &suyo, &nada());

        assert!(
            texto.contains("binding_terminal_overlay = KEY_F11"),
            "{texto}"
        );
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn lo_demas_del_archivo_queda_intacto() {
        // Byte por byte salvo la línea que cambia: los comentarios explican por qué
        // cada opción está donde está, y perderlos es perder la mitad del archivo.
        let antes = como_lo_dejo_el_paquete();
        let (texto, _) = apply(WAYFIRE, &antes, &nada());

        let cambiadas: Vec<(&str, &str)> = antes
            .lines()
            .zip(texto.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(cambiadas.len(), 1, "{cambiadas:?}");
        assert!(texto.contains("# La desplegable."), "{texto}");
        assert_eq!(antes.lines().count(), texto.lines().count());
    }

    #[test]
    fn una_vez_hecho_no_se_repite() {
        // Quien después vuelva a poner F12 a propósito se queda en F12: es lo mismo
        // que hace el registro con las claves que se ofrecieron una vez.
        let mut ya = nada();
        ya.insert(marca(&REPLACEMENTS[0]));

        let (texto, hechos) = apply(WAYFIRE, &como_lo_dejo_el_paquete(), &ya);

        assert!(texto.contains("KEY_F12"), "{texto}");
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn no_se_pisa_un_atajo_que_ya_esta_en_uso() {
        // Con el combo nuevo ligado a otra cosa, reemplazar dejaría dos claves con
        // el mismo atajo: wayfire dispara la primera y la otra no anda nunca, sin
        // decir nada.
        let suyo = como_lo_dejo_el_paquete().replace(
            "binding_terminal = <super> KEY_T",
            "binding_terminal = <super> <alt> KEY_T",
        );
        let (texto, hechos) = apply(WAYFIRE, &suyo, &nada());

        assert!(
            texto.contains("binding_terminal_overlay = KEY_F12"),
            "{texto}"
        );
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn el_mismo_atajo_escrito_distinto_igual_se_reconoce() {
        // `KEY_T <super>` y `<super> KEY_T` son el mismo atajo, y los archivos que
        // el paquete fue trayendo tienen las dos formas. Comparando el texto tal
        // cual, el reemplazo no se aplicaría —o peor: se aplicaría encima de un
        // combo que ya estaba en uso escrito al revés—.
        let suyo = como_lo_dejo_el_paquete().replace(
            "binding_terminal = <super> KEY_T",
            "binding_terminal = KEY_T <alt><super>",
        );
        let (_, hechos) = apply(WAYFIRE, &suyo, &nada());

        assert!(
            hechos.is_empty(),
            "el combo ya estaba en uso escrito al revés: {hechos:?}"
        );
    }

    #[test]
    fn una_clave_con_el_mismo_nombre_en_otra_seccion_no_cuenta() {
        // No es una búsqueda y reemplazo sobre el archivo: la clave nombrada, en su
        // sección.
        let ajeno = "[otra]\nbinding_terminal_overlay = KEY_F12\n";
        let (texto, hechos) = apply(WAYFIRE, ajeno, &nada());

        assert_eq!(texto, ajeno);
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn un_archivo_sin_la_clave_no_se_toca() {
        // Quien borró la desplegable a propósito no la recupera por acá: esto
        // cambia valores, no agrega claves.
        let sin = "[command]\nbinding_terminal = <super> KEY_T\n";
        let (texto, hechos) = apply(WAYFIRE, sin, &nada());

        assert_eq!(texto, sin);
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn otro_archivo_no_recibe_los_reemplazos_de_este() {
        let (texto, hechos) = apply(
            ".config/gtk-3.0/settings.ini",
            &como_lo_dejo_el_paquete(),
            &nada(),
        );

        assert!(texto.contains("KEY_F12"), "{texto}");
        assert!(hechos.is_empty(), "{hechos:?}");
    }

    #[test]
    fn los_terminadores_de_windows_se_conservan() {
        // `str::lines()` se come tanto `\n` como `\r\n`: reconstruir con `\n` a
        // secas convierte el archivo entero, que es lo contrario del prometido.
        let crlf = como_lo_dejo_el_paquete().replace('\n', "\r\n");
        let (texto, hechos) = apply(WAYFIRE, &crlf, &nada());

        assert_eq!(hechos.len(), 1);
        assert!(texto.contains("\r\n"), "{texto:?}");
        assert!(
            !texto.contains("KEY_T\n"),
            "quedó una línea con terminador de Unix: {texto:?}"
        );
    }

    #[test]
    fn la_marca_lleva_el_valor_viejo() {
        // Para que el día que el mismo atajo cambie por segunda vez, el reemplazo
        // nuevo sea otra marca y se aplique en lugar de parecer uno ya hecho.
        let (_, seccion, clave) = marca(&REPLACEMENTS[0]);

        assert_eq!(seccion, REPLACEMENT_SECTION);
        assert_eq!(clave, "binding_terminal_overlay@KEY_F12");
    }

    #[test]
    fn la_lista_nombra_archivos_que_la_migracion_mira() {
        // Un reemplazo sobre un archivo que no está en `ARCHIVOS` no se aplicaría
        // nunca, y no fallaría nada: quedaría ahí, pareciendo que hace algo.
        for r in &REPLACEMENTS {
            assert!(
                super::super::ARCHIVOS.contains(&r.archivo),
                "«{}» no está entre los archivos que se fusionan",
                r.archivo
            );
        }
    }

    #[test]
    fn ningun_reemplazo_deja_el_valor_como_estaba() {
        // Una entrada con `anterior == nuevo` reescribiría el archivo en cada
        // arranque para dejarlo igual.
        for r in &REPLACEMENTS {
            assert!(
                !es_el_mismo(r.anterior, r.nuevo),
                "«{}» no cambia nada",
                r.clave
            );
        }
    }
}
