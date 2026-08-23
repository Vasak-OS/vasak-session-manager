//! El fondo de la pantalla de inicio de sesión: una imagen, o un video.
//!
//! El greeter corre antes de que nadie haya iniciado sesión, así que no hay
//! configuración de usuario que leer: lo que muestra es el fondo del sistema que
//! viene con VasakOS. Un administrador lo puede apuntar a otro lado con un
//! archivo de una línea, que es también donde va a escribir la aplicación de
//! configuración cuando elegir el fondo del inicio se exponga en la interfaz.
//!
//! Siempre hay una imagen, incluso cuando lo configurado es un video: es el
//! respaldo de todo lo que puede salir mal del otro lado —sin decodificador, un
//! archivo demasiado grande, un contenedor roto— y la alternativa a esa imagen
//! no es un video, es una pantalla negra.
//!
//! La imagen se le entrega a la página como `data:` URL. La vista web no puede
//! leer `file://` desde su propio origen, y abrirle el sistema de archivos por
//! un fondo es un mal negocio en una pantalla que maneja contraseñas.

use std::io::Read;
use std::path::{Path, PathBuf};

/// Un reemplazo, una ruta absoluta por archivo, gana la primera línea no vacía.
const OVERRIDE_FILE: &str = "/etc/vasak-session-manager/background";

/// El fondo de `vasakos-wallpapers`, y los nombres simples que un administrador
/// esperaría que funcionen si deja un archivo él mismo.
const IMAGE_DEFAULTS: &[&str] = &[
    "/usr/share/backgrounds/vasakos/default.jpg",
    "/usr/share/backgrounds/vasakos/default.png",
    "/usr/share/backgrounds/vasakos/default.webp",
];

/// Lo mismo para un fondo en movimiento: dejar el archivo alcanza, sin tener que
/// además escribir el archivo de configuración.
const VIDEO_DEFAULTS: &[&str] = &[
    "/usr/share/backgrounds/vasakos/default.mp4",
    "/usr/share/backgrounds/vasakos/default.webm",
];

/// Grande para una foto, chico para que un archivo perdido no demore el primer
/// dibujado del inicio de sesión: la imagen entera cruza el puente IPC
/// codificada en base64 antes de que se muestre algo.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// El video se reproduce desde memoria (ver `read_background_video`), así que su
/// tamaño es RAM ocupada mientras la pantalla está a la vista. Un fondo en bucle
/// son unas decenas de megas; más que esto es una película apuntada por error, y
/// esta pantalla es lo primero que arranca en la máquina.
const MAX_VIDEO_BYTES: u64 = 64 * 1024 * 1024;

/// Lo que el elemento `<video>` de WebKit puede llegar a reproducir. Qué puede
/// de verdad lo decide la página con `canPlayType`, que es la única que sabe qué
/// decodificadores hay instalados.
const VIDEO_EXTENSIONS: &[(&str, &str)] = &[
    ("mp4", "video/mp4"),
    ("webm", "video/webm"),
    ("ogv", "video/ogg"),
];

/// Lo que la página necesita para dibujar el fondo: la imagen siempre, el video
/// cuando hay uno que valga la pena intentar.
#[derive(serde::Serialize)]
pub struct Background {
    /// `data:` URL, o `null` cuando no se encontró ninguna imagen usable; ahí la
    /// página se queda con su propio color de fondo y no con una pantalla vacía.
    image: Option<String>,
    video: Option<BackgroundVideo>,
}

/// Un video de fondo, descrito pero todavía no leído: los bytes se piden aparte
/// para que el cuadro de inicio de sesión se dibuje sin esperarlos.
#[derive(serde::Serialize)]
pub struct BackgroundVideo {
    /// Para los mensajes de la página cuando algo falla; nunca vuelve a Rust.
    path: String,
    /// Con qué preguntarle a WebKit si tiene el decodificador.
    extension: String,
    /// El tipo del `Blob`, sin que la página tenga que adivinarlo.
    mime: String,
    bytes: u64,
}

fn configured_path() -> Option<PathBuf> {
    let content = std::fs::read_to_string(OVERRIDE_FILE).ok()?;

    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(PathBuf::from)
}

fn video_mime(path: &Path) -> Option<(String, String)> {
    let extension = path.extension()?.to_string_lossy().to_lowercase();

    VIDEO_EXTENSIONS
        .iter()
        .find(|(name, _)| *name == extension)
        .map(|(_, mime)| (extension, (*mime).to_string()))
}

fn is_video(path: &Path) -> bool {
    video_mime(path).is_some()
}

/// El tamaño del archivo cuando se puede usar, `None` cuando no: no existe, no
/// es un archivo, está vacío o pasa el límite.
fn usable_size(path: &Path, max: u64) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;

    (meta.is_file() && meta.len() > 0 && meta.len() <= max).then(|| meta.len())
}

/// La imagen de fondo, que es también el respaldo del video.
///
/// Recorre los candidatos hasta encontrar uno que la vista web pueda dibujar: un
/// video configurado, o una ruta que no lleva a ninguna imagen, no dejan la
/// pantalla sin fondo, siguen hasta el que viene con el sistema.
fn pick_image() -> Option<String> {
    configured_path()
        .into_iter()
        .chain(IMAGE_DEFAULTS.iter().map(PathBuf::from))
        .filter(|path| !is_video(path))
        .filter(|path| usable_size(path, MAX_BYTES).is_some())
        .find_map(|path| crate::users::image_data_url(&path))
}

/// Qué video hay que intentar, si hay alguno.
///
/// Un fondo configurado es una decisión explícita: si lo elegido es una imagen,
/// no se pone un video encima porque exista el archivo. Y si lo elegido es un
/// video que no se puede usar, se cae a la imagen y no a otro video, que sería
/// mostrar algo que nadie pidió.
fn configured_video() -> Option<PathBuf> {
    match configured_path() {
        Some(path) if is_video(&path) => Some(path),
        Some(_) => None,
        None => VIDEO_DEFAULTS
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_file()),
    }
}

fn pick_video() -> Option<BackgroundVideo> {
    let path = configured_video()?;
    let (extension, mime) = video_mime(&path)?;

    let Some(bytes) = usable_size(&path, MAX_VIDEO_BYTES) else {
        // Al journal de greetd: desde la pantalla de inicio de sesión no hay
        // dónde contarlo, y sin esto el síntoma es «configuré un video y sigo
        // viendo la foto» sin ninguna pista de por qué.
        eprintln!(
            "[greeter] el fondo {} no se puede usar como video: no es un archivo legible o pasa \
             el límite de {} MB, que existe porque se reproduce desde memoria",
            path.display(),
            MAX_VIDEO_BYTES / 1024 / 1024
        );
        return None;
    };

    Some(BackgroundVideo {
        path: path.to_string_lossy().into_owned(),
        extension,
        mime,
        bytes,
    })
}

#[tauri::command]
pub fn get_background() -> Background {
    Background {
        image: pick_image(),
        video: pick_video(),
    }
}

/// Los bytes del video, para que la página lo reproduzca desde un `blob:`.
///
/// Un `<video src>` apuntando al protocolo interno de Tauri no funciona, y no es
/// por los codecs: el elemento multimedia de WebKit no se sirve del cargador de
/// recursos de la página sino de GStreamer, que no sabe leer de un esquema
/// propio. Termina en error 4 y encima reintenta, y cada reintento entrega el
/// archivo entero hasta agotar la memoria. Lo que sí funciona es que la página
/// se traiga los bytes y los reproduzca desde memoria.
///
/// Sin parámetros a propósito: el archivo lo elige este lado, no la página. Un
/// comando que leyera la ruta que le pasan sería un lector de archivos
/// arbitrarios a un paso de la pantalla que pide la contraseña.
#[tauri::command]
pub fn read_background_video() -> Result<tauri::ipc::Response, String> {
    let video = pick_video().ok_or_else(|| "no hay video de fondo que leer".to_string())?;

    // Se lee acotado, no completo y después se revisa. `std::fs::read` reserva
    // el archivo entero antes de que cualquier control pueda opinar: si creció
    // entre que `pick_video` lo midió y esto lo abre, el proceso ya pagó la
    // memoria y el control sólo llega a contarlo. Y esta pantalla es el primer
    // proceso de la máquina, así que esa memoria se nota.
    //
    // Se pide un byte más que el límite: si llega ese byte extra, el archivo se
    // pasa y se rechaza sin haber reservado más que eso.
    let archivo = std::fs::File::open(&video.path)
        .map_err(|error| format!("no se pudo abrir el fondo {}: {error}", video.path))?;

    let mut bytes = Vec::new();
    std::io::Read::take(archivo, MAX_VIDEO_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("no se pudo leer el fondo {}: {error}", video.path))?;

    if bytes.len() as u64 > MAX_VIDEO_BYTES {
        return Err(format!(
            "el fondo {} creció por encima del límite mientras se leía",
            video.path
        ));
    }

    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Lo que pidió la revisión: el límite tiene que acotar **la lectura**, no
    /// sólo revisar lo que ya se cargó en memoria. Se comprueba sobre el
    /// mecanismo —`take`— con un archivo más grande que el tope.
    #[test]
    fn la_lectura_se_corta_en_el_limite_en_vez_de_cargar_todo() {
        let dir = std::env::temp_dir().join("vasak-fondo-limite");
        let _ = std::fs::create_dir_all(&dir);
        let grande = dir.join("grande.bin");

        let tope: u64 = 1024;
        std::fs::write(&grande, vec![0u8; (tope * 4) as usize]).unwrap();

        let archivo = std::fs::File::open(&grande).unwrap();
        let mut leido = Vec::new();
        std::io::Read::take(archivo, tope + 1)
            .read_to_end(&mut leido)
            .unwrap();

        assert_eq!(
            leido.len() as u64,
            tope + 1,
            "sólo se reserva un byte más que el tope, no el archivo entero"
        );
        assert!(leido.len() as u64 > tope, "y ese byte extra es el que delata que se pasó");
    }

    #[test]
    fn los_videos_se_reconocen_por_extension_sin_importar_la_capitalizacion() {
        assert!(is_video(Path::new("/tmp/fondo.mp4")));
        assert!(is_video(Path::new("/tmp/fondo.WebM")));
        assert!(is_video(Path::new("/tmp/fondo.ogv")));

        assert!(!is_video(Path::new("/tmp/fondo.jpg")));
        assert!(!is_video(Path::new("/tmp/fondo")));
        assert!(!is_video(Path::new("/tmp/fondo.mp4.jpg")));
    }

    #[test]
    fn el_mime_es_el_del_contenedor_y_no_el_de_la_extension() {
        assert_eq!(
            video_mime(Path::new("/tmp/fondo.ogv")),
            Some(("ogv".to_string(), "video/ogg".to_string()))
        );
    }

    #[test]
    fn un_archivo_vacio_o_pasado_de_tamano_no_es_usable() {
        let dir = tempfile::tempdir().unwrap();

        let vacio = dir.path().join("vacio.mp4");
        std::fs::File::create(&vacio).unwrap();
        assert_eq!(usable_size(&vacio, 10), None);

        let lleno = dir.path().join("lleno.mp4");
        std::fs::File::create(&lleno)
            .unwrap()
            .write_all(&[0u8; 32])
            .unwrap();
        assert_eq!(usable_size(&lleno, 32), Some(32));
        assert_eq!(usable_size(&lleno, 31), None);

        assert_eq!(usable_size(dir.path(), 1024), None);
        assert_eq!(usable_size(&dir.path().join("no-existe.mp4"), 1024), None);
    }
}
