//! Fusionar un archivo INI sin reescribirlo.
//!
//! El problema: el paquete agrega opciones nuevas en cada versión —atajos de
//! teclado, plugins, valores por omisión— y `/etc/skel` sólo alcanza a las cuentas
//! **nuevas**. En esta máquina el `wayfire.ini` del usuario tiene 115 asignaciones
//! y el del paquete 97, con 42 líneas distintas: ni copiar el del paquete encima
//! —se pierde todo lo que la persona configuró— ni dejarlo como está —los atajos
//! nuevos no llegan nunca.
//!
//! Y no se puede parsear y volver a serializar: eso borra los comentarios, cambia
//! el orden y reformatea. El `wayfire.ini` del escritorio está lleno de
//! comentarios que explican por qué cada opción está donde está.
//!
//! Así que se trabaja **por líneas**: el archivo del usuario se conserva byte por
//! byte y sólo se **insertan** las claves que le faltan, en su sección.

use std::collections::HashSet;

/// Una asignación de un INI, con la sección a la que pertenece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asignacion {
    pub seccion: String,
    pub clave: String,
    pub valor: String,
}

/// Lo que se le agregó a un archivo.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Agregado {
    pub claves: Vec<(String, String)>,
}

/// Si una línea abre una sección, devuelve su nombre.
fn seccion_de(linea: &str) -> Option<&str> {
    let t = linea.trim();
    let dentro = t.strip_prefix('[')?.strip_suffix(']')?;
    Some(dentro.trim())
}

/// Si una línea es una asignación, devuelve la clave.
///
/// Los comentarios se saltan **antes** de buscar el `=`: un `# duration = 150`
/// tomado por asignación haría creer que la clave ya está y la opción nueva no se
/// agregaría nunca.
fn clave_de(linea: &str) -> Option<&str> {
    let t = linea.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') {
        return None;
    }
    let clave = t.split('=').next()?.trim();
    (!clave.is_empty() && t.contains('=')).then_some(clave)
}

/// Las asignaciones de un INI, en orden.
pub fn asignaciones_de(texto: &str) -> Vec<Asignacion> {
    let mut seccion = String::new();
    let mut salida = Vec::new();
    for linea in texto.lines() {
        if let Some(s) = seccion_de(linea) {
            seccion = s.to_string();
            continue;
        }
        if let Some(clave) = clave_de(linea) {
            let valor = linea
                .trim()
                .split_once('=')
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default();
            salida.push(Asignacion {
                seccion: seccion.clone(),
                clave: clave.to_string(),
                valor,
            });
        }
    }
    salida
}

/// Si el archivo ya tiene esa clave en esa sección.
pub fn tiene(texto: &str, seccion: &str, clave: &str) -> bool {
    asignaciones_de(texto)
        .iter()
        .any(|a| a.seccion == seccion && a.clave == clave)
}

/// El terminador de línea que usa el archivo.
///
/// `str::lines()` se come tanto `\n` como `\r\n`, así que reconstruir con `\n` a
/// secas convierte un archivo con terminadores de Windows entero. Y el prometido de
/// este módulo es que el archivo del usuario se conserva: cambiarle los mil
/// terminadores para agregar una clave no es conservarlo.
pub fn terminador_de(texto: &str) -> &'static str {
    if texto.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Dónde termina una sección: el índice de línea **después** de su última línea
/// con contenido.
///
/// Después de la última con contenido y no antes de la siguiente sección, para no
/// insertar en medio de las líneas en blanco que separan los bloques.
fn fin_de_seccion(lineas: &[&str], seccion: &str) -> Option<usize> {
    let mut actual = String::new();
    let mut ultima_con_contenido = None;
    for (i, linea) in lineas.iter().enumerate() {
        if let Some(s) = seccion_de(linea) {
            if actual == seccion {
                // Una clave sin sección va **antes** del primer encabezado. Puesta
                // al final del archivo, el analizador la asocia con la última
                // sección: una clave global del paquete terminaba dentro de
                // `[core]` en un archivo que sólo tiene `[core]`.
                return Some(ultima_con_contenido.map_or(i, |u| u + 1));
            }
            actual = s.to_string();
            if actual == seccion {
                ultima_con_contenido = Some(i);
            }
            continue;
        }
        if actual == seccion && !linea.trim().is_empty() {
            ultima_con_contenido = Some(i);
        }
    }
    ultima_con_contenido.map(|i| i + 1)
}

/// Los atajos que expresa un valor, normalizados. Vacío si el valor no es un atajo.
///
/// Wayfire escribe un atajo como una lista de modificadores y una tecla, y acepta
/// las dos formas: `<super> KEY_T` y `KEY_T <super>` son el mismo atajo escrito
/// distinto, y las dos conviven en los archivos que el paquete fue trayendo. Por eso
/// se normaliza —modificadores en minúscula y ordenados, la tecla después— antes de
/// comparar. Los modificadores se pueden pegar (`<alt><ctrl> KEY_T`).
///
/// Varios atajos para la misma acción van separados por `|`.
///
/// Un valor que no es un atajo —`plugins = expo cube`, un comando, un número—
/// devuelve la lista vacía: sólo se comparan atajos con atajos.
pub fn combos_de(valor: &str) -> Vec<String> {
    valor.split('|').filter_map(combo_de).collect()
}

fn combo_de(uno: &str) -> Option<String> {
    let mut modificadores = Vec::new();
    let mut teclas = Vec::new();

    for token in tokens_de(uno)? {
        if token.starts_with('<') {
            modificadores.push(token);
        } else if token.starts_with("KEY_") || token.starts_with("BTN_") {
            teclas.push(token);
        } else {
            // Cualquier otra cosa y el valor no es un atajo: un comando, un
            // nombre de plugin, un número.
            return None;
        }
    }

    if modificadores.is_empty() && teclas.is_empty() {
        return None;
    }
    modificadores.sort();
    modificadores.dedup();
    Some(format!("{}{}", modificadores.concat(), teclas.concat()))
}

/// Parte un atajo en sus piezas. `None` si hay un `<` sin cerrar.
fn tokens_de(texto: &str) -> Option<Vec<String>> {
    let mut salida = Vec::new();
    let mut resto = texto.trim();

    while !resto.is_empty() {
        if let Some(desde) = resto.strip_prefix('<') {
            let (modificador, sigue) = desde.split_once('>')?;
            salida.push(format!("<{}>", modificador.trim().to_lowercase()));
            resto = sigue.trim_start();
        } else {
            let fin = resto
                .find(|c: char| c.is_whitespace() || c == '<')
                .unwrap_or(resto.len());
            let (pieza, sigue) = resto.split_at(fin);
            salida.push(pieza.to_string());
            resto = sigue.trim_start();
        }
    }

    Some(salida)
}

/// El sufijo de una clave de atajo del `[command]` de wayfire, si lo es.
///
/// `binding_terminal` es la mitad de un par: la otra es `command_terminal`, y una
/// sin la otra no hace nada.
fn sufijo_de_atajo(clave: &str) -> Option<&str> {
    ["repeatable_binding_", "always_binding_", "binding_"]
        .into_iter()
        .find_map(|prefijo| clave.strip_prefix(prefijo))
}

/// Agrega a `usuario` las claves de `paquete` que le falten y que no estén en
/// `ya_ofrecidas`.
///
/// Devuelve el texto nuevo y qué se agregó. Nunca cambia un valor que ya está: si
/// la clave existe, se deja como la dejó la persona, aunque el paquete traiga otra
/// cosa. Y nunca borra nada, ni siquiera lo que el paquete dejó de traer.
///
/// `ya_ofrecidas` es lo que hace que una clave **borrada a propósito** no vuelva en
/// el próximo inicio de sesión. Sin eso, «actualizar sin pisar» sería «insistir
/// para siempre».
///
/// Y tampoco se agrega un **atajo que ya está ligado**, aunque la clave se llame
/// distinto. En el `[command]` de wayfire el nombre de la clave es una etiqueta
/// arbitraria y el atajo vive en el valor, así que comparar por nombre no alcanza:
/// el paquete renombró `binding_custom_0` a `binding_terminal` sin cambiarle el
/// `<super> KEY_T`, y en las cuentas que tenían el nombre viejo la fusión agregaba
/// el nuevo al lado. La tecla no ganaba una opción: abría tres terminales.
pub fn fusionar(
    usuario: &str,
    paquete: &str,
    ya_ofrecidas: &dyn Fn(&str, &str) -> bool,
) -> (String, Agregado) {
    let terminador = terminador_de(usuario);
    let mut lineas: Vec<String> = usuario.lines().map(|l| l.to_string()).collect();
    let mut agregado = Agregado::default();

    // Los atajos que el archivo del usuario ya tiene ligados, normalizados.
    let mut ligados: HashSet<String> = asignaciones_de(usuario)
        .iter()
        .flat_map(|a| combos_de(&a.valor))
        .collect();

    // Los `binding_*` que se van a saltear por chocar. Se calcula antes del bucle
    // para no depender de que el `binding_` venga escrito arriba de su `command_`:
    // agregar la mitad que ejecuta sin la mitad que la dispara deja una línea que
    // no hace nada.
    let del_paquete = asignaciones_de(paquete);
    let pares_saltados: HashSet<(String, String)> = del_paquete
        .iter()
        .filter_map(|a| {
            let sufijo = sufijo_de_atajo(&a.clave)?;
            let choca = combos_de(&a.valor).iter().any(|c| ligados.contains(c));
            (choca && !tiene(usuario, &a.seccion, &a.clave)).then(|| {
                (a.seccion.clone(), sufijo.to_string())
            })
        })
        .collect();

    for a in del_paquete {
        let actual = lineas.join("\n");
        if tiene(&actual, &a.seccion, &a.clave) || ya_ofrecidas(&a.seccion, &a.clave) {
            continue;
        }

        let combos = combos_de(&a.valor);
        if combos.iter().any(|c| ligados.contains(c)) {
            continue;
        }
        if let Some(sufijo) = a.clave.strip_prefix("command_") {
            if pares_saltados.contains(&(a.seccion.clone(), sufijo.to_string())) {
                continue;
            }
        }

        let nueva = format!("{} = {}", a.clave, a.valor);
        let refs: Vec<&str> = lineas.iter().map(String::as_str).collect();
        match fin_de_seccion(&refs, &a.seccion) {
            Some(donde) => lineas.insert(donde, nueva),
            None => {
                // La sección no existe: se crea al final, con una línea en blanco
                // antes para que el archivo siga legible.
                if lineas.last().is_some_and(|l| !l.trim().is_empty()) {
                    lineas.push(String::new());
                }
                if !a.seccion.is_empty() {
                    lineas.push(format!("[{}]", a.seccion));
                }
                lineas.push(nueva);
            }
        }
        ligados.extend(combos);
        agregado.claves.push((a.seccion.clone(), a.clave.clone()));
    }

    let mut texto = lineas.join(terminador);
    // El salto final se conserva **como estaba**: si el original lo tenía, va; si
    // no lo tenía, no se le agrega. Agregarlo era cambiar el archivo por algo que
    // nadie pidió, y `git diff` lo marca igual que quitarlo.
    let termina_con_salto = usuario.ends_with('\n');
    if termina_con_salto || usuario.is_empty() {
        texto.push_str(terminador);
    }
    (texto, agregado)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nada_ofrecido(_: &str, _: &str) -> bool {
        false
    }

    #[test]
    fn una_clave_nueva_entra_en_su_seccion() {
        let usuario = "[animate]\nduration = 300\n\n[core]\nplugins = expo\n";
        let paquete = "[animate]\nduration = 150\nopen_animation = fade\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);

        assert!(r.contains("open_animation = fade"));
        assert_eq!(a.claves, vec![("animate".to_string(), "open_animation".to_string())]);
        // Y entra en `[animate]`, no al final del archivo.
        let pos_nueva = r.find("open_animation").unwrap();
        let pos_core = r.find("[core]").unwrap();
        assert!(pos_nueva < pos_core, "{r}");
    }

    #[test]
    fn el_valor_de_la_persona_no_se_toca() {
        // Es lo que hace que esto sea «sin pisar»: el paquete dice 150 y el usuario
        // puso 300, así que queda 300.
        let usuario = "[animate]\nduration = 300\n";
        let paquete = "[animate]\nduration = 150\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("duration = 300"));
        assert!(!r.contains("150"));
        assert!(a.claves.is_empty());
    }

    #[test]
    fn los_comentarios_y_el_orden_sobreviven() {
        // No se puede parsear y volver a serializar: el wayfire.ini del escritorio
        // está lleno de comentarios que explican por qué cada opción está ahí.
        let usuario = "# por qué esto está acá\n[core]\n# no tocar\nplugins = expo\n";
        let paquete = "[core]\nvsync = true\n";
        let (r, _) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.starts_with("# por qué esto está acá\n"));
        assert!(r.contains("# no tocar\n"));
        assert!(r.contains("plugins = expo"));
        assert!(r.contains("vsync = true"));
    }

    #[test]
    fn una_clave_que_se_borro_a_proposito_no_vuelve() {
        // Sin esto, «actualizar sin pisar» sería «insistir para siempre»: la opción
        // que alguien sacó volvería en cada inicio de sesión.
        let usuario = "[animate]\nduration = 300\n";
        let paquete = "[animate]\nduration = 150\nopen_animation = fade\n";
        let ya = |s: &str, c: &str| s == "animate" && c == "open_animation";
        let (r, a) = fusionar(usuario, paquete, &ya);
        assert!(!r.contains("open_animation"), "{r}");
        assert!(a.claves.is_empty());
    }

    #[test]
    fn una_seccion_nueva_se_crea_al_final() {
        let usuario = "[core]\nplugins = expo\n";
        let paquete = "[nueva]\nopcion = 1\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("[nueva]"));
        assert!(r.contains("opcion = 1"));
        assert!(r.find("[core]").unwrap() < r.find("[nueva]").unwrap());
        assert_eq!(a.claves.len(), 1);
    }

    #[test]
    fn nunca_se_borra_lo_que_el_paquete_dejo_de_traer() {
        // Puede ser algo que la persona quiere conservar, y no es nuestro para
        // decidirlo.
        let usuario = "[core]\nplugins = expo\nvieja = 1\n";
        let paquete = "[core]\nplugins = expo\n";
        let (r, _) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("vieja = 1"));
    }

    #[test]
    fn una_clave_comentada_no_cuenta_como_puesta() {
        // Con `# duration = 150` tomado por asignación, la opción nueva no se
        // agregaría nunca y nadie sabría por qué.
        let usuario = "[animate]\n# duration = 150\n";
        let paquete = "[animate]\nduration = 150\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert_eq!(a.claves.len(), 1, "{r}");
        assert!(r.contains("# duration = 150"), "el comentario se conserva");
        assert!(r.lines().any(|l| l.trim() == "duration = 150"));
    }

    #[test]
    fn la_misma_clave_en_dos_secciones_son_dos_claves() {
        // `duration` existe en varios plugins de wayfire con valores distintos.
        let usuario = "[animate]\nduration = 300\n\n[expo]\nplugins = x\n";
        let paquete = "[animate]\nduration = 150\n\n[expo]\nduration = 200\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert_eq!(a.claves, vec![("expo".to_string(), "duration".to_string())]);
        assert!(r.contains("duration = 300"));
        assert!(r.contains("duration = 200"));
    }

    #[test]
    fn las_claves_sin_seccion_se_manejan() {
        // `mimeapps.list` y algunos `.conf` empiezan con asignaciones sueltas.
        let usuario = "clave = 1\n";
        let paquete = "clave = 1\notra = 2\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("otra = 2"));
        assert_eq!(a.claves, vec![(String::new(), "otra".to_string())]);
    }

    #[test]
    fn un_archivo_de_usuario_vacio_recibe_todo() {
        let (r, a) = fusionar("", "[core]\nplugins = expo\n", &nada_ofrecido);
        assert!(r.contains("[core]"));
        assert!(r.contains("plugins = expo"));
        assert_eq!(a.claves.len(), 1);
    }

    #[test]
    fn el_salto_final_se_conserva() {
        // Sin él, git y varios editores marcan la última línea como cambiada en
        // cada inicio de sesión.
        let (r, _) = fusionar("[a]\nx = 1\n", "[a]\ny = 2\n", &nada_ofrecido);
        assert!(r.ends_with('\n'));
        assert!(!r.ends_with("\n\n"), "y sólo uno");
    }

    #[test]
    fn fusionar_dos_veces_no_cambia_nada_la_segunda() {
        // Esto corre en cada inicio de sesión: si no fuera idempotente, el archivo
        // crecería una copia por arranque.
        let usuario = "[animate]\nduration = 300\n";
        let paquete = "[animate]\nduration = 150\nopen_animation = fade\n";
        let (una, _) = fusionar(usuario, paquete, &nada_ofrecido);
        let (dos, a) = fusionar(&una, paquete, &nada_ofrecido);
        assert_eq!(una, dos);
        assert!(a.claves.is_empty());
    }

    #[test]
    fn un_valor_con_iguales_adentro_no_se_corta() {
        // Los atajos de wayfire son así: `binding = <super> KEY_A`, y algunos
        // comandos llevan `=`.
        let a = asignaciones_de("[comando]\nbinding = sh -c 'x=1'\n");
        assert_eq!(a[0].clave, "binding");
        assert_eq!(a[0].valor, "sh -c 'x=1'");
    }

    #[test]
    fn los_terminadores_de_windows_no_se_reescriben() {
        // `str::lines()` se come tanto `\n` como `\r\n`, así que reconstruir con
        // `\n` a secas convierte el archivo entero. Cambiarle los mil terminadores
        // para agregar una clave no es «conservar el archivo del usuario».
        let usuario = "[animate]\r\nduration = 300\r\n";
        let paquete = "[animate]\nopen_animation = fade\n";
        let (r, _) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("\r\n"), "{r:?}");
        assert!(!r.contains("\n\n"), "no se mezclan estilos: {r:?}");
        assert_eq!(r.matches('\n').count(), r.matches("\r\n").count());
    }

    #[test]
    fn un_archivo_sin_salto_final_no_recibe_uno() {
        // Agregarlo era cambiar el archivo por algo que nadie pidió, y `git diff`
        // lo marca igual que quitarlo.
        let (r, _) = fusionar("[a]\nx = 1", "[a]\ny = 2", &nada_ofrecido);
        assert!(!r.ends_with('\n'), "{r:?}");
        assert!(r.contains("y = 2"));
    }

    #[test]
    fn una_clave_sin_seccion_va_antes_del_primer_encabezado() {
        // Puesta al final, el analizador la asocia con la última sección: una clave
        // global del paquete terminaba dentro de `[core]`.
        let usuario = "[core]\nplugins = expo\n";
        let paquete = "global = 1\n[core]\nplugins = expo\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert_eq!(a.claves, vec![(String::new(), "global".to_string())]);
        let pos_global = r.find("global = 1").expect("está");
        let pos_core = r.find("[core]").expect("está");
        assert!(pos_global < pos_core, "quedó dentro de [core]: {r}");

        // Y volviendo a leerlo, la clave no pertenece a ninguna sección.
        let leidas = asignaciones_de(&r);
        let global = leidas.iter().find(|x| x.clave == "global").expect("está");
        assert_eq!(global.seccion, "");
    }

    /// El caso que abría tres terminales con una sola tecla.
    ///
    /// El paquete renombró las etiquetas del `[command]`: `binding_custom_0` y
    /// `binding_custom_4` —las dos con el mismo `KEY_T <super>`— pasaron a ser
    /// `binding_terminal = <super> KEY_T`. En una cuenta con los nombres viejos,
    /// fusionar por nombre agregaba el nombre nuevo al lado de los dos que ya
    /// estaban, y wayfire dispara todos los que coincidan.
    #[test]
    fn un_atajo_que_ya_esta_ligado_no_se_agrega_con_otro_nombre() {
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       command_custom_0 = vasak-terminal\n\
                       binding_custom_4 = KEY_T <super>\n\
                       command_custom_4 = vasak-terminal\n";
        let paquete = "[command]\n\
                       binding_terminal = <super> KEY_T\n\
                       command_terminal = vasak-terminal\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);

        assert!(!r.contains("binding_terminal"), "{r}");
        // Y su mitad tampoco: un `command_` sin `binding_` es una línea muerta.
        assert!(!r.contains("command_terminal"), "{r}");
        assert!(a.claves.is_empty());
        // Lo que la persona tenía sigue intacto, incluso lo que ya estaba repetido:
        // acá no se borra nada, sólo se deja de agregar.
        assert!(r.contains("binding_custom_0"));
        assert!(r.contains("binding_custom_4"));
    }

    #[test]
    fn el_mismo_atajo_escrito_al_reves_es_el_mismo_atajo() {
        // `KEY_T <super>` y `<super> KEY_T` conviven en los archivos que el paquete
        // fue trayendo. Comparando el texto tal cual, el segundo entraba encima del
        // primero.
        assert_eq!(combos_de("KEY_T <super>"), combos_de("<super> KEY_T"));
        assert_eq!(combos_de("<alt> <ctrl> KEY_T"), combos_de("<ctrl> <alt> KEY_T"));
        // Pegados también es válido en wayfire.
        assert_eq!(combos_de("<alt><ctrl> KEY_T"), combos_de("<ctrl> <alt> KEY_T"));
        // Y las mayúsculas del modificador no hacen a otro atajo.
        assert_eq!(combos_de("<Super> KEY_L"), combos_de("<super> KEY_L"));
    }

    #[test]
    fn lo_que_no_es_un_atajo_no_se_compara_como_atajo() {
        // Si un comando o una lista de plugins contara como atajo, dos claves sin
        // nada que ver dejarían de agregarse por «chocar».
        assert!(combos_de("vasak-terminal").is_empty());
        assert!(combos_de("dbus-send --session --dest=org.vasak.os.Desktop").is_empty());
        assert!(combos_de("expo cube animate").is_empty());
        assert!(combos_de("1920x1080@75").is_empty());
        assert!(combos_de("").is_empty());
        assert!(combos_de("true").is_empty());
        assert!(combos_de("sh -c 'x=1'").is_empty());
        // Un `<` sin cerrar no es un atajo a medio escribir: no es un atajo.
        assert!(combos_de("<super KEY_T").is_empty());

        // Y lo que sí lo es, lo es.
        assert_eq!(combos_de("KEY_MUTE"), vec!["KEY_MUTE".to_string()]);
        assert_eq!(combos_de("<super>"), vec!["<super>".to_string()]);
        assert_eq!(combos_de("BTN_EXTRA"), vec!["BTN_EXTRA".to_string()]);
    }

    #[test]
    fn un_atajo_libre_si_se_agrega() {
        // La otra mitad de la regla: no agregar lo que choca no puede convertirse
        // en no agregar nada. `KEY_F12` no estaba ligado, así que entra.
        let usuario = "[command]\nbinding_terminal = <super> KEY_T\ncommand_terminal = vasak-terminal\n";
        let paquete = "[command]\n\
                       binding_terminal = <super> KEY_T\n\
                       binding_terminal_overlay = KEY_F12\n\
                       command_terminal_overlay = vasak-terminal --overlay\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("binding_terminal_overlay = KEY_F12"), "{r}");
        assert!(r.contains("command_terminal_overlay = vasak-terminal --overlay"), "{r}");
        assert_eq!(a.claves.len(), 2);
    }

    #[test]
    fn el_command_entra_cuando_el_binding_ya_estaba_por_nombre() {
        // Distinto de chocar: acá la persona tiene el atajo con el mismo nombre y
        // le falta la mitad que ejecuta. Sin el `command_`, la tecla no hace nada.
        let usuario = "[command]\nbinding_files = <super> KEY_E\n";
        let paquete = "[command]\nbinding_files = <super> KEY_E\ncommand_files = vasak-file-manager\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("command_files = vasak-file-manager"), "{r}");
        assert_eq!(a.claves, vec![("command".to_string(), "command_files".to_string())]);
    }

    #[test]
    fn el_choque_tambien_cuenta_entre_secciones() {
        // Un atajo es global: que `<super> KEY_E` esté en `[expo]` y el paquete lo
        // traiga en `[command]` no lo hace otro atajo, lo hace un conflicto.
        let usuario = "[expo]\ntoggle = <super> KEY_E\n";
        let paquete = "[command]\nbinding_files = <super> KEY_E\ncommand_files = vasak-file-manager\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(!r.contains("binding_files"), "{r}");
        assert!(a.claves.is_empty());
    }

    #[test]
    fn dos_atajos_distintos_del_paquete_entran_los_dos() {
        // La regla mira lo que ya está ligado y lo que se va agregando, así que no
        // puede dejar entrar dos veces el mismo ni frenar dos distintos.
        let usuario = "[command]\n";
        let paquete = "[command]\n\
                       binding_terminal = <super> KEY_T\n\
                       binding_terminal_alt = <alt> <ctrl> KEY_T\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("binding_terminal = <super> KEY_T"), "{r}");
        assert!(r.contains("binding_terminal_alt = <alt> <ctrl> KEY_T"), "{r}");
        assert_eq!(a.claves.len(), 2);
    }

    #[test]
    fn una_clave_sin_seccion_va_despues_del_preambulo_que_ya_habia() {
        // Si ya hay claves globales, la nueva va con ellas y no arriba de todo, que
        // podría meterse antes de un comentario de cabecera.
        let usuario = "# cabecera\nuna = 1\n\n[core]\nplugins = expo\n";
        let paquete = "otra = 2\n";
        let (r, _) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.starts_with("# cabecera\n"), "{r}");
        let pos = |t: &str| r.find(t).unwrap();
        assert!(pos("una = 1") < pos("otra = 2"));
        assert!(pos("otra = 2") < pos("[core]"), "{r}");
    }
}
