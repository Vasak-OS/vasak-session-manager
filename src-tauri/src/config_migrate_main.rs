//! `vasak-config-migrate`: lleva a una cuenta que ya existe las opciones nuevas
//! del paquete, sin pisar lo que la persona configuró.
//!
//! Corre al arrancar la sesión, desde `vasak-session`, antes del compositor. Es
//! idempotente y no falla hacia afuera: un problema acá no puede impedir que la
//! sesión abra.
//!
//! ```text
//! vasak-config-migrate            aplica
//! vasak-config-migrate --prueba   dice qué haría, sin tocar nada
//! ```

use std::path::PathBuf;
use vasak_session_manager_lib::migracion;

fn main() {
    let argumentos: Vec<String> = std::env::args().collect();
    let prueba = argumentos.iter().any(|a| a == "--prueba" || a == "--dry-run");

    let Some(hogar) = std::env::var_os("HOME").map(PathBuf::from).filter(|p| p.is_absolute())
    else {
        eprintln!("[config-migrate] sin HOME: no hay nada que migrar");
        return;
    };

    let skel = std::env::var_os("VASAK_SKEL")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(migracion::SKEL));
    if !skel.is_dir() {
        eprintln!("[config-migrate] no está {}: nada que hacer", skel.display());
        return;
    }

    let registro = migracion::estado::ruta(&hogar);
    let resultados = migracion::aplicar(&hogar, &skel, &registro, !prueba);

    let mut cambios = 0;
    for r in &resultados {
        if let Some(motivo) = &r.motivo {
            // A depuración: que un archivo no esté es lo normal, no un problema.
            eprintln!("[config-migrate] {} : {motivo}", r.archivo);
            continue;
        }
        // Lo que no se puede resolver solo. Va aunque no haya nada que cambiar: es
        // información sobre el archivo, no un cambio.
        for c in &r.conflictos {
            eprintln!(
                "[config-migrate] {}: «{}» está ligado a comandos distintos por {} \
                 — no se toca, decidilo vos",
                r.archivo,
                c.combo,
                c.claves.join(", ")
            );
        }

        if r.agregadas.is_empty() && r.quitadas.is_empty() {
            continue;
        }
        cambios += r.agregadas.len() + r.quitadas.len();

        let donde = |seccion: &String, clave: &String| {
            if seccion == migracion::SECCION_DE_LINEA {
                // Una línea que carga el archivo del paquete, no una clave.
                format!("la línea que carga {clave}")
            } else if seccion.is_empty() {
                clave.clone()
            } else {
                format!("{seccion}.{clave}")
            }
        };

        // Primero lo que se retira, que es lo que más conviene ver: es la única
        // parte de esto que saca una línea del archivo de alguien.
        let saco = if prueba { "quitaría" } else { "quitado" };
        for q in &r.quitadas {
            let porque = match q.motivo {
                migracion::ini::Motivo::LaPusimosYChoca => {
                    "lo había puesto la actualización y choca con un atajo anterior"
                }
                migracion::ini::Motivo::RepiteAOtra => {
                    "repite un atajo anterior que hace exactamente lo mismo"
                }
            };
            eprintln!(
                "[config-migrate] {saco} de {}: {} ({porque})",
                r.archivo,
                donde(&q.seccion, &q.clave)
            );
        }

        let que = if prueba { "agregaría" } else { "agregado" };
        for (seccion, clave) in &r.agregadas {
            eprintln!("[config-migrate] {que} en {}: {}", r.archivo, donde(seccion, clave));
        }
    }

    if cambios == 0 {
        eprintln!("[config-migrate] no hay nada que cambiar en esta cuenta");
    }
}
