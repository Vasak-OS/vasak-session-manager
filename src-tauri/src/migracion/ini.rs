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

use std::collections::{HashMap, HashSet};

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

/// Los atajos que liga una asignación, **si es de las que llevan etiqueta libre**.
///
/// Sólo participan las claves `binding_*` del `[command]` de wayfire, que son las
/// que tienen el problema: su nombre es una etiqueta arbitraria y el atajo vive en
/// el valor, así que dos nombres distintos pueden ligar la misma tecla.
///
/// Las opciones de plugin quedan afuera aunque su valor parezca un atajo, y eso es
/// justo lo que hay que hacer: el propio paquete trae `[zoom] modifier = <super>`
/// —que no es un atajo sino con qué modificador se hace zoom— y
/// `[wayfire-shell] toggle_menu = <super>`. Tomándolas por atajos, `binding_menu`
/// chocaba con ellas y no se agregaba nunca en ninguna cuenta. Sus nombres son
/// fijos, así que comparar por nombre —lo que ya se hacía— alcanza y sobra.
///
/// La sección va en el par porque las etiquetas son del `[command]` que las define.
fn atajos_de(a: &Asignacion) -> Vec<(String, String)> {
    if sufijo_de_atajo(&a.clave).is_none() {
        return Vec::new();
    }
    combos_de(&a.valor)
        .into_iter()
        .map(|c| (a.seccion.clone(), c))
        .collect()
}

/// Un atajo ligado más de una vez **a cosas distintas**. No se toca: se informa.
#[derive(Debug, PartialEq, Eq)]
pub struct Conflicto {
    /// El atajo, normalizado.
    pub combo: String,
    /// Las claves que se lo disputan, en orden de archivo.
    pub claves: Vec<String>,
}

/// Por qué se retiró una línea.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motivo {
    /// La puso la migración y choca con un atajo que ya estaba.
    LaPusimosYChoca,
    /// Otra línea anterior liga el mismo atajo al mismo comando.
    RepiteAOtra,
}

/// Una línea retirada: dónde estaba y por qué se fue.
#[derive(Debug, PartialEq, Eq)]
pub struct Quitada {
    pub seccion: String,
    pub clave: String,
    pub motivo: Motivo,
}

/// El resultado de limpiar un archivo.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Limpieza {
    pub texto: String,
    pub quitadas: Vec<Quitada>,
    /// Lo que quedó sin resolver, para que se pueda ver. Elegir por la persona
    /// cuál de dos teclas distintas gana sería cambiarle lo que hace el teclado.
    pub conflictos: Vec<Conflicto>,
}

/// El comando que dispara un atajo: el `command_x` de su `binding_x`.
fn comando_de<'a>(todas: &'a [Asignacion], a: &Asignacion) -> Option<&'a str> {
    let sufijo = sufijo_de_atajo(&a.clave)?;
    let pareja = format!("command_{sufijo}");
    todas
        .iter()
        .find(|x| x.seccion == a.seccion && x.clave == pareja)
        .map(|x| x.valor.as_str())
}

/// Quita del archivo del usuario las líneas que ligan un atajo que ya está ligado.
///
/// Son dos casos distintos y se resuelven distinto, porque la pregunta difícil no es
/// cuál sobra sino **cuál se queda**.
///
/// # 1. Lo que puso la migración y choca con lo que ya estaba
///
/// Acá el que se queda está claro: la línea anterior. La nuestra es la de más.
/// Tres condiciones para tocarla, porque borrar configuración ajena es justo lo que
/// este módulo promete no hacer:
///
/// 1. **La pusimos nosotros.** Sale del registro de lo ya ofrecido —`fue_agregada`—,
///    que es la única forma de distinguir una línea nuestra de una que la persona
///    escribió. Lo que no está anotado no entra por este camino.
/// 2. **Sigue diciendo lo que decía cuando la pusimos**, comparado contra el archivo
///    del paquete. Si alguien le cambió el valor, la línea dejó de ser nuestra.
/// 3. **Choca de verdad** con otra línea.
///
/// # 2. Dos líneas que hacen exactamente lo mismo
///
/// El paquete viejo traía el mismo atajo con varias etiquetas —`binding_custom_0` y
/// `binding_custom_4`, las dos con `KEY_T <super>` y las dos ejecutando
/// `vasak-terminal`; cuatro `KEY_MUTE` ejecutando todas `amixer set Master toggle`—,
/// y esas líneas están en las cuentas desde que se crearon: no las agregó la
/// migración y no hay registro que las señale.
///
/// Se quitan igual, pero **sólo cuando disparan el mismo comando**. Ahí no hay nada
/// que elegir: la tecla queda haciendo exactamente lo que hacía, una vez en vez de
/// tres. Se conserva la primera del archivo.
///
/// Si el mismo atajo dispara **comandos distintos**, no se toca ninguna y se informa:
/// decidir cuál gana es cambiarle a alguien lo que hace una tecla, y eso no es
/// nuestro. Es el único caso en el que esto deja el archivo como estaba.
///
/// # En los dos casos
///
/// Cuando se quita un `binding_x` se quita también su `command_x`: solo no dispara
/// nada. Y el registro no se toca, así que una clave nuestra que se retira sigue
/// anotada como ofrecida y no vuelve en el próximo arranque.
pub fn quitar_choques(
    usuario: &str,
    paquete: &str,
    fue_agregada: &dyn Fn(&str, &str) -> bool,
) -> Limpieza {
    let del_usuario = asignaciones_de(usuario);
    let del_paquete = asignaciones_de(paquete);

    let nuestra = |a: &Asignacion| {
        fue_agregada(&a.seccion, &a.clave)
            && del_paquete
                .iter()
                .any(|p| p.seccion == a.seccion && p.clave == a.clave && p.valor == a.valor)
    };

    // Qué se retira y por qué. Marca la línea y, si es un atajo, la mitad que
    // ejecuta: sola no dispara nada.
    let mut sobran: HashMap<(String, String), Motivo> = HashMap::new();
    let descartar = |a: &Asignacion, motivo: Motivo, sobran: &mut HashMap<_, _>| {
        sobran.insert((a.seccion.clone(), a.clave.clone()), motivo);
        if let Some(sufijo) = sufijo_de_atajo(&a.clave) {
            let pareja = format!("command_{sufijo}");
            if del_usuario
                .iter()
                .any(|x| x.seccion == a.seccion && x.clave == pareja)
            {
                sobran.insert((a.seccion.clone(), pareja), motivo);
            }
        }
    };

    // ── 1. Lo nuestro pierde contra lo que ya estaba ────────────────────────────
    //
    // El orden del archivo no decide acá: aunque nuestra línea esté escrita más
    // arriba, la que se queda es la de la persona.
    let mut ligados: HashSet<(String, String)> = del_usuario
        .iter()
        .filter(|a| !nuestra(a))
        .flat_map(atajos_de)
        .collect();

    for a in del_usuario.iter().filter(|a| nuestra(a)) {
        let atajos = atajos_de(a);
        if atajos.is_empty() {
            continue;
        }
        if atajos.iter().any(|c| ligados.contains(c)) {
            descartar(a, Motivo::LaPusimosYChoca, &mut sobran);
        } else {
            ligados.extend(atajos);
        }
    }

    // ── 2. Dos líneas que hacen exactamente lo mismo ────────────────────────────
    //
    // Acá sí manda el orden: se conserva la primera. Sólo se miran los atajos de un
    // solo combo; uno que lista varios con `|` puede compartir uno y no los otros, y
    // quitar la línea entera perdería los que no se repetían.
    let mut dueno: Vec<((String, String), &Asignacion)> = Vec::new();
    let mut conflictos: Vec<Conflicto> = Vec::new();

    for a in del_usuario.iter() {
        if sobran.contains_key(&(a.seccion.clone(), a.clave.clone())) {
            continue;
        }
        let atajos = atajos_de(a);
        let [atajo] = atajos.as_slice() else { continue };

        let Some((_, primero)) = dueno.iter().find(|(c, _)| c == atajo) else {
            dueno.push((atajo.clone(), a));
            continue;
        };

        let comando = comando_de(&del_usuario, a);
        if comando.is_some() && comando == comando_de(&del_usuario, primero) {
            descartar(a, Motivo::RepiteAOtra, &mut sobran);
            continue;
        }

        // Distinto comando: no se toca, se informa.
        match conflictos.iter_mut().find(|x| x.combo == atajo.1) {
            Some(ya) => ya.claves.push(a.clave.clone()),
            None => conflictos.push(Conflicto {
                combo: atajo.1.clone(),
                claves: vec![primero.clave.clone(), a.clave.clone()],
            }),
        }
    }

    if sobran.is_empty() {
        return Limpieza {
            texto: usuario.to_string(),
            quitadas: Vec::new(),
            conflictos,
        };
    }

    // ── Reescribir sin esas líneas, conservando todo lo demás byte por byte ──────
    let terminador = terminador_de(usuario);
    let mut seccion = String::new();
    let mut salida: Vec<&str> = Vec::new();
    let mut quitadas = Vec::new();

    for linea in usuario.lines() {
        if let Some(s) = seccion_de(linea) {
            seccion = s.to_string();
            salida.push(linea);
            continue;
        }
        if let Some(clave) = clave_de(linea) {
            if let Some(&motivo) = sobran.get(&(seccion.clone(), clave.to_string())) {
                quitadas.push(Quitada {
                    seccion: seccion.clone(),
                    clave: clave.to_string(),
                    motivo,
                });
                continue;
            }
        }
        salida.push(linea);
    }

    let mut texto = salida.join(terminador);
    if usuario.ends_with('\n') || usuario.is_empty() {
        texto.push_str(terminador);
    }

    Limpieza {
        texto,
        quitadas,
        conflictos,
    }
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
    let mut ligados: HashSet<(String, String)> = asignaciones_de(usuario)
        .iter()
        .flat_map(atajos_de)
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
            let choca = atajos_de(a).iter().any(|c| ligados.contains(c));
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

        let atajos = atajos_de(&a);
        if atajos.iter().any(|c| ligados.contains(c)) {
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
        ligados.extend(atajos);
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

    /// La regla mira sólo las claves de etiqueta libre, y tiene que ser así.
    ///
    /// El propio paquete trae `[zoom] modifier = <super>` —que no es un atajo sino
    /// con qué modificador se hace zoom— y `[wayfire-shell] toggle_menu = <super>`.
    /// Si esos valores contaran como atajos ligados, `binding_menu = <super>`
    /// chocaría con ellos y no se agregaría en **ninguna** cuenta. Sus nombres son
    /// fijos, así que la comparación por nombre que ya existía alcanza.
    #[test]
    fn una_opcion_de_plugin_no_bloquea_un_atajo_del_paquete() {
        let usuario = "[zoom]\nmodifier = <super>\n\n[wayfire-shell]\ntoggle_menu = <super>\n\n[command]\n";
        let paquete = "[command]\nbinding_menu = <super>\ncommand_menu = dbus-send --session\n";
        let (r, a) = fusionar(usuario, paquete, &nada_ofrecido);
        assert!(r.contains("binding_menu = <super>"), "{r}");
        assert!(r.contains("command_menu = dbus-send --session"), "{r}");
        assert_eq!(a.claves.len(), 2);
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

    /// Lo que le quedó en el archivo a quien ya pasó por la actualización rota.
    #[test]
    fn se_retira_el_atajo_que_puso_la_migracion_y_choca() {
        // `binding_custom_0` es de la persona; `binding_terminal` lo puso la
        // migración con el mismo `<super> KEY_T`, y por eso la tecla abría dos.
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       command_custom_0 = vasak-terminal\n\
                       binding_terminal = <super> KEY_T\n\
                       command_terminal = vasak-terminal\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\ncommand_terminal = vasak-terminal\n";
        let nuestras = |s: &str, c: &str| s == "command" && c.ends_with("_terminal");

        let Limpieza { texto: r, quitadas, .. } = quitar_choques(usuario, paquete, &nuestras);

        assert!(!r.contains("binding_terminal"), "{r}");
        // Y su pareja: un `command_` sin `binding_` no dispara nada.
        assert!(!r.contains("command_terminal"), "{r}");
        // Lo de la persona queda intacto.
        assert!(r.contains("binding_custom_0 = KEY_T <super>"), "{r}");
        assert!(r.contains("command_custom_0 = vasak-terminal"), "{r}");
        assert_eq!(quitadas.len(), 2);
    }

    #[test]
    fn no_se_toca_una_linea_que_la_persona_no_puso_la_migracion() {
        // La condición que hace que esto no sea «borrar configuración ajena»: lo
        // que no está anotado en el registro no se toca ni aunque choque.
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       binding_mio = <super> KEY_T\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\n";
        let nada_nuestro = |_: &str, _: &str| false;

        let Limpieza { texto: r, quitadas, .. } = quitar_choques(usuario, paquete, &nada_nuestro);
        assert_eq!(r, usuario);
        assert!(quitadas.is_empty());
    }

    #[test]
    fn no_se_toca_la_linea_que_la_persona_editó_despues() {
        // Se la pusimos nosotros, pero le cambió la tecla: dejó de ser nuestra.
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       binding_terminal = <super> <shift> KEY_T\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\n";
        let nuestras = |s: &str, c: &str| s == "command" && c == "binding_terminal";

        let Limpieza { texto: r, quitadas, .. } = quitar_choques(usuario, paquete, &nuestras);
        assert_eq!(r, usuario, "el valor no es el del paquete: es de la persona");
        assert!(quitadas.is_empty());
    }

    #[test]
    fn el_atajo_que_pusimos_y_no_choca_se_queda() {
        // La limpieza no puede convertirse en deshacer la actualización.
        let usuario = "[command]\n\
                       binding_terminal_overlay = KEY_F12\n\
                       command_terminal_overlay = vasak-terminal --overlay\n";
        let paquete = usuario;
        let nuestras = |_: &str, _: &str| true;

        let Limpieza { texto: r, quitadas, .. } = quitar_choques(usuario, paquete, &nuestras);
        assert_eq!(r, usuario);
        assert!(quitadas.is_empty());
    }

    #[test]
    fn de_dos_que_pusimos_con_el_mismo_atajo_queda_una() {
        // El paquete viejo traía `KEY_T <super>` con dos etiquetas, así que a una
        // cuenta que no tenía ninguna se le agregaban las dos.
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       binding_custom_4 = KEY_T <super>\n";
        let paquete = usuario;
        let nuestras = |_: &str, _: &str| true;

        let Limpieza { texto: r, quitadas, .. } = quitar_choques(usuario, paquete, &nuestras);
        assert!(r.contains("binding_custom_0"), "queda la primera: {r}");
        assert!(!r.contains("binding_custom_4"), "{r}");
        assert_eq!(
            quitadas,
            vec![Quitada {
                seccion: "command".to_string(),
                clave: "binding_custom_4".to_string(),
                motivo: Motivo::LaPusimosYChoca,
            }]
        );
    }

    /// Lo que traía el skel viejo: el mismo atajo con dos etiquetas.
    ///
    /// Estas líneas están en la cuenta desde que se creó, no las puso la migración y
    /// no hay registro que las señale. Se quitan igual porque disparan **el mismo
    /// comando**: la tecla queda haciendo exactamente lo que hacía, una vez.
    #[test]
    fn dos_lineas_que_hacen_lo_mismo_quedan_en_una() {
        let usuario = "[command]\n\
                       binding_custom_0 = KEY_T <super>\n\
                       command_custom_0 = vasak-terminal\n\
                       binding_custom_4 = KEY_T <super>\n\
                       command_custom_4 = vasak-terminal\n";
        let nada_nuestro = |_: &str, _: &str| false;

        let limpieza = quitar_choques(usuario, "", &nada_nuestro);

        assert!(limpieza.texto.contains("binding_custom_0"), "queda la primera");
        assert!(limpieza.texto.contains("command_custom_0"), "y su comando");
        assert!(!limpieza.texto.contains("binding_custom_4"), "{}", limpieza.texto);
        assert!(!limpieza.texto.contains("command_custom_4"), "{}", limpieza.texto);
        assert_eq!(limpieza.quitadas.len(), 2);
        // Y con el motivo que corresponde: ésta no la puso la migración.
        assert!(limpieza.quitadas.iter().all(|q| q.motivo == Motivo::RepiteAOtra));
        assert!(limpieza.conflictos.is_empty());
    }

    #[test]
    fn los_cuatro_key_mute_del_skel_viejo_quedan_en_uno() {
        // El caso real, con los nombres y el comando que traía el paquete.
        let usuario = "[command]\n\
                       binding_mute = KEY_MUTE\n\
                       command_mute = amixer set Master toggle\n\
                       binding_custom_2 = KEY_MUTE\n\
                       command_custom_2 = amixer set Master toggle\n\
                       binding_custom_3 = KEY_MUTE\n\
                       command_custom_3 = amixer set Master toggle\n\
                       binding_custom_6 = KEY_MUTE\n\
                       command_custom_6 = amixer set Master toggle\n";
        let limpieza = quitar_choques(usuario, "", &|_: &str, _: &str| false);

        let ligados = asignaciones_de(&limpieza.texto)
            .iter()
            .filter(|a| sufijo_de_atajo(&a.clave).is_some())
            .count();
        assert_eq!(ligados, 1, "{}", limpieza.texto);
        assert!(limpieza.texto.contains("binding_mute"), "queda el primero");
        assert_eq!(limpieza.quitadas.len(), 6, "tres atajos y sus tres comandos");
    }

    /// El único caso en el que la limpieza deja el archivo como estaba.
    #[test]
    fn el_mismo_atajo_con_comandos_distintos_no_se_toca_y_se_informa() {
        // Elegir cuál gana es cambiarle a alguien lo que hace una tecla.
        let usuario = "[command]\n\
                       binding_mute = KEY_MUTE\n\
                       command_mute = wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle\n\
                       binding_custom_2 = KEY_MUTE\n\
                       command_custom_2 = amixer set Master toggle\n";
        let limpieza = quitar_choques(usuario, "", &|_: &str, _: &str| false);

        assert_eq!(limpieza.texto, usuario);
        assert!(limpieza.quitadas.is_empty());
        assert_eq!(
            limpieza.conflictos,
            vec![Conflicto {
                combo: "KEY_MUTE".to_string(),
                claves: vec!["binding_mute".to_string(), "binding_custom_2".to_string()],
            }]
        );
    }

    #[test]
    fn la_limpieza_tampoco_mira_las_opciones_de_plugin() {
        // Misma razón que al agregar: `[zoom] modifier` no es un atajo, y reportarlo
        // como conflicto contra `binding_menu` sería un aviso falso en cada arranque
        // de cada cuenta.
        let usuario = "[zoom]\nmodifier = <super>\n\n\
                       [command]\nbinding_menu = <super>\ncommand_menu = dbus-send\n";
        let limpieza = quitar_choques(usuario, "", &|_: &str, _: &str| false);

        assert_eq!(limpieza.texto, usuario);
        assert!(limpieza.quitadas.is_empty());
        assert!(limpieza.conflictos.is_empty(), "{:?}", limpieza.conflictos);
    }

    #[test]
    fn la_linea_de_la_persona_gana_aunque_este_escrita_mas_abajo() {
        // El orden del archivo decide entre dos líneas iguales, pero no entre una
        // nuestra y una suya: ahí gana la suya, esté donde esté.
        let usuario = "[command]\n\
                       binding_terminal = <super> KEY_T\n\
                       command_terminal = vasak-terminal\n\
                       binding_mio = <super> KEY_T\n\
                       command_mio = alacritty\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\ncommand_terminal = vasak-terminal\n";
        let nuestras = |s: &str, c: &str| s == "command" && c.ends_with("_terminal");

        let limpieza = quitar_choques(usuario, paquete, &nuestras);

        assert!(!limpieza.texto.contains("binding_terminal"), "{}", limpieza.texto);
        assert!(limpieza.texto.contains("binding_mio"), "{}", limpieza.texto);
        // Y no queda reportado como conflicto: se resolvió quitando la nuestra.
        assert!(limpieza.conflictos.is_empty(), "{:?}", limpieza.conflictos);
    }

    #[test]
    fn la_limpieza_conserva_los_comentarios_y_el_resto_del_archivo() {
        // Se trabaja por líneas, igual que al agregar: el archivo no se reescribe.
        let usuario = "# cabecera\n\
                       [command]\n\
                       # por qué esto está acá\n\
                       binding_custom_0 = KEY_T <super>\n\
                       binding_terminal = <super> KEY_T\n\
                       \n\
                       [animate]\n\
                       duration = 300\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\n";
        let nuestras = |s: &str, c: &str| s == "command" && c == "binding_terminal";

        let Limpieza { texto: r, .. } = quitar_choques(usuario, paquete, &nuestras);
        assert!(r.starts_with("# cabecera\n"), "{r}");
        assert!(r.contains("# por qué esto está acá\n"), "{r}");
        assert!(r.contains("[animate]\nduration = 300"), "{r}");
        assert!(r.ends_with('\n'));
    }

    #[test]
    fn limpiar_dos_veces_no_cambia_nada_la_segunda() {
        // Corre en cada inicio de sesión.
        let usuario = "[command]\nbinding_custom_0 = KEY_T <super>\nbinding_terminal = <super> KEY_T\n";
        let paquete = "[command]\nbinding_terminal = <super> KEY_T\n";
        let nuestras = |s: &str, c: &str| s == "command" && c == "binding_terminal";

        let Limpieza { texto: una, quitadas: q1, .. } = quitar_choques(usuario, paquete, &nuestras);
        let Limpieza { texto: dos, quitadas: q2, .. } = quitar_choques(&una, paquete, &nuestras);
        assert_eq!(una, dos);
        assert_eq!(q1.len(), 1);
        assert!(q2.is_empty());
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
