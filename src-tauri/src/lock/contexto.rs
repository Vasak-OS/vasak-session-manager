//! Lo que la pantalla de bloqueo puede contar sin desbloquear nada.
//!
//! Dos cosas, y las dos con el mismo criterio: **decir que hay algo, no decir
//! qué**. La pantalla está bloqueada justamente porque quien está adelante
//! puede no ser el dueño de la sesión.
//!
//! - De las notificaciones sale sólo el icono de la aplicación y cuántas tiene
//!   sin leer. Ni el resumen ni el cuerpo cruzan hasta acá, así que un mensaje
//!   no se lee de reojo con la sesión trabada.
//! - Del reproductor sale lo que ya se escucha en la habitación, que no es un
//!   secreto, y los dos botones que hacen falta.

use serde::Serialize;
use std::time::Duration;
use zbus::blocking::{Connection, Proxy};

/// Ninguna consulta puede dejar colgada a la pantalla de bloqueo.
///
/// Un reproductor que no contesta —pasa: una pestaña del navegador que se
/// quedó— bloquearía el arranque de la única pantalla desde la que se puede
/// volver a entrar. Antes que esperar, se muestra sin esa parte.
const ESPERA: Duration = Duration::from_millis(400);

/// Una aplicación con notificaciones sin leer.
#[derive(Debug, Serialize, PartialEq)]
pub struct AplicacionConAvisos {
    /// El nombre del icono en el tema, tal como lo mandó la aplicación.
    pub icono: String,
    /// Para el texto alternativo. No es el contenido de la notificación.
    pub aplicacion: String,
    pub cuantas: u32,
}

/// Lo que suena, si algo suena.
#[derive(Debug, Serialize, PartialEq)]
pub struct Reproduccion {
    /// El bus del reproductor, para mandarle las órdenes al mismo.
    pub reproductor: String,
    pub titulo: String,
    pub artista: String,
    pub sonando: bool,
}

/// Agrupa por aplicación las notificaciones sin leer que devuelve el demonio.
///
/// Separado de la consulta para poder probarlo: lo que llega es el JSON de
/// `org.vasak.Notifications.get_unread`.
pub fn agrupar_avisos(json: &str) -> Vec<AplicacionConAvisos> {
    let items: Vec<serde_json::Value> = serde_json::from_str(json).unwrap_or_default();
    let mut grupos: Vec<AplicacionConAvisos> = Vec::new();

    for item in items {
        let aplicacion = item
            .get("app_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if aplicacion.is_empty() {
            continue;
        }

        let icono = item
            .get("app_icon")
            .and_then(|v| v.as_str())
            .filter(|icono| !icono.is_empty())
            .unwrap_or("dialog-information")
            .to_string();

        match grupos.iter_mut().find(|g| g.aplicacion == aplicacion) {
            Some(grupo) => grupo.cuantas += 1,
            None => grupos.push(AplicacionConAvisos {
                icono,
                aplicacion,
                cuantas: 1,
            }),
        }
    }

    grupos
}

fn conexion() -> Option<Connection> {
    Connection::session().ok()
}

/// Las aplicaciones con avisos sin leer, o una lista vacía.
#[tauri::command]
pub fn lock_notifications() -> Vec<AplicacionConAvisos> {
    con_espera(avisos_sin_leer).unwrap_or_default()
}

fn avisos_sin_leer() -> Vec<AplicacionConAvisos> {
    let Some(conexion) = conexion() else {
        return Vec::new();
    };

    let proxy = match Proxy::new(
        &conexion,
        "org.vasak.Notifications",
        "/org/vasak/Notifications",
        "org.vasak.Notifications",
    ) {
        Ok(proxy) => proxy,
        Err(_) => return Vec::new(),
    };

    match proxy.call_method("GetUnread", &()) {
        Ok(respuesta) => respuesta
            .body()
            .deserialize::<String>()
            .map(|json| agrupar_avisos(&json))
            .unwrap_or_default(),
        // Sin demonio de notificaciones no hay nada que mostrar, y no es un
        // error de esta pantalla.
        Err(_) => Vec::new(),
    }
}

/// El primer reproductor que esté sonando, o `None`.
///
/// «Sonando» y no «abierto»: un reproductor en pausa desde hace horas no es
/// contexto de esta sesión, es ruido en una pantalla que tiene que decir poco.
#[tauri::command]
pub fn lock_media() -> Option<Reproduccion> {
    con_espera(reproduccion_actual).flatten()
}

fn reproduccion_actual() -> Option<Reproduccion> {
    let conexion = conexion()?;
    for bus in reproductores(&conexion) {
        if let Some(reproduccion) = leer_reproductor(&conexion, &bus) {
            if reproduccion.sonando {
                return Some(reproduccion);
            }
        }
    }
    None
}

/// Pausa o pasa a la siguiente. Devuelve si la orden salió.
#[tauri::command]
pub fn lock_media_action(player: String, action: String) -> bool {
    // El nombre del bus viene de `lock_media`, pero igual se comprueba: lo que
    // llega es lo que mande la página, y desde acá se llama a un servicio
    // cualquiera del bus de sesión.
    if !player.starts_with("org.mpris.MediaPlayer2.") {
        return false;
    }

    let metodo = match action.as_str() {
        "playpause" => "PlayPause",
        "next" => "Next",
        _ => return false,
    };

    let Some(conexion) = conexion() else {
        return false;
    };

    reproductor_proxy(&conexion, &player)
        .and_then(|proxy| proxy.call_method(metodo, &()).map(|_| ()).ok())
        .is_some()
}

fn reproductor_proxy<'a>(conexion: &'a Connection, bus: &str) -> Option<Proxy<'a>> {
    Proxy::new(
        conexion,
        bus.to_string(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .ok()
}

fn reproductores(conexion: &Connection) -> Vec<String> {
    let proxy = match Proxy::new(
        conexion,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    ) {
        Ok(proxy) => proxy,
        Err(_) => return Vec::new(),
    };

    let nombres: Vec<String> = proxy
        .call_method("ListNames", &())
        .ok()
        .and_then(|r| r.body().deserialize().ok())
        .unwrap_or_default();

    nombres
        .into_iter()
        .filter(|n| n.starts_with("org.mpris.MediaPlayer2."))
        .collect()
}

fn leer_reproductor(conexion: &Connection, bus: &str) -> Option<Reproduccion> {
    let proxy = reproductor_proxy(conexion, bus)?;

    let estado: String = proxy.get_property("PlaybackStatus").ok()?;
    let metadatos: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
        proxy.get_property("Metadata").unwrap_or_default();

    let texto = |clave: &str| -> String {
        metadatos
            .get(clave)
            .and_then(|v| <&str>::try_from(v).ok())
            .unwrap_or_default()
            .to_string()
    };
    // `xesam:artist` es una lista: se toma el primero, que es lo que muestra
    // cualquier reproductor.
    let primero = |clave: &str| -> String {
        match metadatos.get(clave).map(std::ops::Deref::deref) {
            Some(zbus::zvariant::Value::Array(lista)) => lista
                .iter()
                .find_map(|valor| <&str>::try_from(valor).ok().map(str::to_string))
                .unwrap_or_default(),
            _ => String::new(),
        }
    };

    Some(Reproduccion {
        reproductor: bus.to_string(),
        titulo: texto("xesam:title"),
        artista: primero("xesam:artist"),
        sonando: estado == "Playing",
    })
}

/// Corre la consulta en un hilo aparte y se rinde a los 400 ms.
///
/// `zbus` bloqueante no tiene tiempo de espera propio, y un reproductor que no
/// contesta —pasa: una pestaña del navegador que se quedó— dejaría sin dibujar
/// la única pantalla desde la que se puede volver a entrar. El hilo que quedó
/// esperando termina solo cuando le contesten, o cuando se cierre la pantalla.
fn con_espera<T, F>(consulta: F) -> Option<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (emisor, receptor) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = emisor.send(consulta());
    });
    receptor.recv_timeout(ESPERA).ok()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn los_avisos_se_agrupan_por_aplicacion() {
        let json = r#"[
            {"app_name":"Telegram","app_icon":"telegram","summary":"Hola","body":"secreto"},
            {"app_name":"Telegram","app_icon":"telegram","summary":"Otra","body":"secreto"},
            {"app_name":"Discord","app_icon":"discord","summary":"Ping","body":"secreto"}
        ]"#;

        assert_eq!(
            agrupar_avisos(json),
            vec![
                AplicacionConAvisos {
                    icono: "telegram".into(),
                    aplicacion: "Telegram".into(),
                    cuantas: 2
                },
                AplicacionConAvisos {
                    icono: "discord".into(),
                    aplicacion: "Discord".into(),
                    cuantas: 1
                },
            ]
        );
    }

    #[test]
    fn el_contenido_no_sale_de_la_pantalla_bloqueada() {
        // Lo que importa de todo esto: que el resumen y el cuerpo no viajen a
        // una pantalla que puede estar mirando cualquiera.
        let json = r#"[{"app_name":"Telegram","app_icon":"telegram",
                        "summary":"Código 123456","body":"No se lo pases a nadie"}]"#;
        let serializado = serde_json::to_string(&agrupar_avisos(json)).unwrap();

        assert!(!serializado.contains("123456"), "{serializado}");
        assert!(!serializado.contains("No se lo pases"), "{serializado}");
        assert!(serializado.contains("telegram"));
    }

    #[test]
    fn la_forma_real_que_manda_el_demonio_se_entiende() {
        // Los campos son los que devuelve `GetUnread` de verdad —comprobado
        // contra el demonio andando—, con los que no se usan incluidos: si
        // alguno cambia de nombre, esto falla acá y no en la pantalla.
        let json = r#"[{"id":5113,"notif_id":290,"app_name":"Telegram Desktop",
                        "app_icon":"telegram-desktop","summary":"Un resumen",
                        "body":"Un cuerpo","urgency":1,
                        "actions":["default","Open"],"seen":false,
                        "timestamp":1787900000}]"#;

        assert_eq!(
            agrupar_avisos(json),
            vec![AplicacionConAvisos {
                icono: "telegram-desktop".into(),
                aplicacion: "Telegram Desktop".into(),
                cuantas: 1
            }]
        );
    }

    #[test]
    fn una_aplicacion_sin_icono_igual_se_muestra() {
        let json = r#"[{"app_name":"Cosa","app_icon":""}]"#;
        let grupos = agrupar_avisos(json);
        assert_eq!(grupos.len(), 1);
        assert_eq!(grupos[0].icono, "dialog-information");
    }

    #[test]
    fn lo_que_no_es_una_lista_no_rompe_nada() {
        // El demonio puede no estar, o contestar cualquier cosa: la pantalla de
        // bloqueo tiene que dibujarse igual.
        assert!(agrupar_avisos("").is_empty());
        assert!(agrupar_avisos("no soy json").is_empty());
        assert!(agrupar_avisos("{}").is_empty());
        assert!(agrupar_avisos(r#"[{"sin":"nombre"}]"#).is_empty());
    }

    #[test]
    fn solo_se_le_habla_a_un_reproductor_mpris() {
        // El nombre del bus llega desde la página. Sin este cerco, la pantalla
        // de bloqueo sería un puente para llamar a cualquier servicio de la
        // sesión.
        assert!(!lock_media_action("org.freedesktop.login1".into(), "next".into()));
        assert!(!lock_media_action("org.vasak.Notifications".into(), "playpause".into()));
        // Y sólo esos dos métodos.
        assert!(!lock_media_action(
            "org.mpris.MediaPlayer2.vlc".into(),
            "Quit".into()
        ));
    }
}
