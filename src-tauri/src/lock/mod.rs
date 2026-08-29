//! The lock screen: the greeter's face, over an already open session.

mod auth;
mod contexto;
mod session_lock;
mod wallpaper;

use gtk::prelude::*;
use session_lock::SessionLock;
use std::cell::RefCell;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

// The held lock lives on the GTK main thread, which is the only thread allowed
// to touch it: GTK objects are not `Send`, so Tauri's managed state — which
// requires it — is not an option. Same reasoning as the shell surfaces in
// vasak-desktop.
thread_local! {
    static HELD: RefCell<Option<SessionLock>> = const { RefCell::new(None) };
}

#[tauri::command]
fn lock_user() -> String {
    auth::current_user()
}

/// The face of whoever this session belongs to, or `null` when there is none.
///
/// Same places the greeter looks — AccountsService first, then `~/.face` — but
/// here the home directory is reachable, because this runs as the person whose
/// session it is.
#[tauri::command]
fn lock_avatar() -> Option<String> {
    let user = auth::current_user();
    let home = std::env::var("HOME").ok()?;

    crate::users::avatar_path(&user, &home)
        .as_deref()
        .and_then(crate::users::image_data_url)
}

/// Answers the page: does this password open this session?
///
/// On success the lock is released here rather than in the page — the release
/// has to happen whether or not the webview is still in a state to ask for it.
#[tauri::command]
fn unlock(app: AppHandle, password: String) -> bool {
    if !auth::verify(&auth::current_user(), &password) {
        return false;
    }

    // Releasing has to happen on the thread that holds it, and the answer to
    // the page does not wait for that: the session is coming back either way.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        HELD.with(|held| {
            if let Some(lock) = held.borrow_mut().take() {
                lock.release();
            }
        });

        // Nothing to stay up for: every surface belonged to a lock that no
        // longer exists.
        handle.exit(0);
    });

    true
}

/// Quién muestra el formulario. Lo escuchan todas las pantallas.
///
/// El mismo nombre que usa `useLockScreen.ts`: es el canal por el que las páginas
/// —una por monitor, sin estado compartido— se ponen de acuerdo.
const EVENTO_PANTALLA_ACTIVA: &str = "lock:pantalla-activa";

/// La etiqueta de la ventana de un monitor.
fn etiqueta_de(indice: i32) -> String {
    format!("lock-{indice}")
}

/// En qué pantalla se dibuja el formulario mientras el compositor no diga otra cosa.
///
/// El monitor primario, y el primero si ninguno se declara primario. Antes esto no
/// existía y la página arrancaba sin saber: mostraba el formulario en **todas** las
/// pantallas hasta que el mouse entrara en alguna, que con el bloqueo por
/// inactividad —donde nadie está tocando el mouse— es para siempre.
fn pantalla_inicial(display: &gdk::Display) -> String {
    (0..display.n_monitors())
        .find(|&i| display.monitor(i).is_some_and(|m| m.is_primary()))
        .map_or_else(|| etiqueta_de(0), etiqueta_de)
}

/// Cuál es la pantalla activa, para la página que todavía no lo sabe.
///
/// Arranca en la del monitor primario y la corrige el foco del teclado. Se guarda
/// —en vez de sólo emitir el evento— porque el foco puede llegar **antes** de que
/// la página monte y se ponga a escuchar: ese aviso no lo recibe nadie, y sin
/// dejarlo anotado la página se quedaría con la de arranque para siempre.
///
/// Va por estado de Tauri y no consultando GDK desde el comando: los comandos
/// corren en el hilo del IPC, y los objetos de GDK sólo se pueden tocar desde el
/// hilo principal.
struct PantallaActiva(std::sync::Mutex<String>);

/// Qué pantalla muestra el formulario mientras nadie reclame nada en la página.
#[tauri::command]
fn lock_active_screen(estado: tauri::State<'_, PantallaActiva>) -> String {
    estado
        .0
        .lock()
        .map(|cual| cual.clone())
        .unwrap_or_else(|envenenado| envenenado.into_inner().clone())
}

/// Puts one surface on every monitor.
///
/// Same reparenting as the panel: Tauri only knows how to build xdg-toplevels,
/// so the webview is created inside a throwaway one and moved into the window
/// that becomes the lock surface.
fn cover_every_monitor(app: &AppHandle, lock: &SessionLock) -> Result<(), String> {
    let display = gdk::Display::default().ok_or("no hay display")?;
    let monitors = display.n_monitors();

    if monitors == 0 {
        return Err("no hay monitores".into());
    }

    for index in 0..monitors {
        let monitor = display
            .monitor(index)
            .ok_or_else(|| format!("no se pudo leer el monitor {index}"))?;

        let label = etiqueta_de(index);
        let webview = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html#/lock".into()))
            .title("Vasak lock")
            .decorations(false)
            .visible(false)
            .build()
            .map_err(|e| format!("no se pudo crear la vista del monitor {index}: {e}"))?;

        let toplevel = webview
            .gtk_window()
            .map_err(|e| format!("sin ventana GTK para el monitor {index}: {e}"))?;

        let surface = gtk::Window::new(gtk::WindowType::Toplevel);
        surface.set_decorated(false);

        // Quién tiene el teclado manda sobre quién dibuja el formulario.
        //
        // Con ext-session-lock el compositor le da el foco a **una** superficie, y
        // no tiene por qué ser la del monitor primario. Sin esto, la persona
        // escribía en una pantalla que no mostraba nada mientras el formulario
        // estaba en otra. El evento es el mismo que se mandan las páginas entre
        // ellas, así que la que lo recibe muestra y las demás esconden.
        let avisar = app.clone();
        let cual = label.clone();
        surface.connect_is_active_notify(move |ventana| {
            if !ventana.is_active() {
                return;
            }
            if let Some(estado) = avisar.try_state::<PantallaActiva>() {
                if let Ok(mut actual) = estado.0.lock() {
                    *actual = cual.clone();
                }
            }
            let _ = avisar.emit(EVENTO_PANTALLA_ACTIVA, &cual);
        });

        reparent(&toplevel, &surface)?;
        lock.attach(&surface, &monitor);
        toplevel.hide();
    }

    Ok(())
}

fn reparent(from: &gtk::ApplicationWindow, to: &gtk::Window) -> Result<(), String> {
    let child = from.child().ok_or("la ventana de Tauri no tiene hijo")?;
    let container = child
        .dynamic_cast_ref::<gtk::Container>()
        .ok_or_else(|| format!("el hijo {} no es un contenedor", child.type_().name()))?;
    let widget = container
        .children()
        .first()
        .cloned()
        .ok_or("el contenedor no tiene la vista web")?;

    container.remove(&widget);
    to.add(&widget);
    Ok(())
}

/// Forks and reports back once the screen is actually covered.
///
/// `before-sleep` needs this. swayidle waits for the command it runs, so a lock
/// screen that stays in the foreground until somebody types their password
/// holds up the suspend for exactly that long — the machine does not sleep
/// until it is unlocked. Returning immediately instead would race the other
/// way: the machine could suspend before anything covers the screen, and wake
/// up showing the desktop.
///
/// So the parent waits for the child to say "locked" through a pipe, and only
/// then exits. Same contract as gtklock's `-d`, which the idle unit already
/// speaks.
fn daemonize() -> Option<std::fs::File> {
    use std::os::unix::io::FromRawFd;

    if !std::env::args().any(|arg| arg == "-d" || arg == "--daemonize") {
        return None;
    }

    let mut fds = [0; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        eprintln!("[lock] no se pudo crear el pipe para avisar; se sigue en primer plano");
        return None;
    }

    match unsafe { libc::fork() } {
        -1 => {
            eprintln!("[lock] no se pudo bifurcar; se sigue en primer plano");
            None
        }
        0 => {
            // Child: keeps the write end and carries on to lock the session.
            unsafe { libc::close(fds[0]) };
            Some(unsafe { std::fs::File::from_raw_fd(fds[1]) })
        }
        _ => {
            // Parent: exits as soon as the child reports the screen is covered,
            // or right away if the child died without covering it — either way
            // suspend is no longer waiting on a password.
            unsafe { libc::close(fds[1]) };
            let mut byte = [0u8; 1];
            let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
            use std::io::Read;
            let locked = reader.read_exact(&mut byte).is_ok();
            std::process::exit(if locked { 0 } else { 1 });
        }
    }
}

/// Draws the screen without locking anything.
///
/// The only way to look at this while developing it. Taking a real lock to
/// check a colour costs a session when anything goes wrong, and it did: a debug
/// build ended up over a live session with no way to authenticate, and the
/// machine had to be power-cycled.
fn dry_run() -> bool {
    std::env::args().any(|arg| arg == "--dry-run")
}

pub fn run() {
    // Before anything else: the fork has to happen before GTK, WebKit or Tauri
    // have started any thread, because only the calling thread survives a fork.
    let ready = daemonize();

    tauri::Builder::default()
        .plugin(tauri_plugin_config_manager::init())
        .plugin(tauri_plugin_vicons::init())
        .plugin(tauri_plugin_i18n_vsk::init_with_path(
            Some(crate::default_locale()),
            crate::locales_dir(),
        ))
        .invoke_handler(tauri::generate_handler![
            lock_user,
            lock_avatar,
            lock_active_screen,
            unlock,
            contexto::lock_notifications,
            contexto::lock_media,
            contexto::lock_media_action,
            wallpaper::lock_background
        ])
        .setup(|app| {
            // Antes de crear ninguna ventana: la página la pide apenas monta, y
            // resolverlo acá es lo que la deja mostrar el formulario en una sola
            // pantalla desde el primer cuadro.
            app.manage(PantallaActiva(std::sync::Mutex::new(
                gdk::Display::default().map_or_else(|| etiqueta_de(0), |d| pantalla_inicial(&d)),
            )));

            if dry_run() {
                eprintln!("[lock] --dry-run: se dibuja en una ventana normal, sin bloquear");
                WebviewWindowBuilder::new(
                    app,
                    "lock-0",
                    WebviewUrl::App("index.html#/lock".into()),
                )
                .title("Vasak lock (dry-run)")
                .inner_size(900.0, 700.0)
                .center()
                .always_on_top(true)
                .build()?;
                return Ok(());
            }

            // Before anything is covered: a lock that cannot check a password
            // is a machine that cannot be recovered without the power button.
            // Refusing to start is the only acceptable answer.
            auth::can_authenticate()?;

            // Taking the lock first is what makes this safe to fail: from here
            // the compositor is already covering every output, so an error
            // below leaves a blank screen and not a visible desktop.
            let lock = SessionLock::acquire()?;

            if let Err(error) = cover_every_monitor(app.handle(), &lock) {
                // The lock is dropped here, which releases the session. A lock
                // screen that cannot draw is worse than none: it would leave
                // the machine unusable with no way to type a password.
                eprintln!("[lock] no se pudo dibujar el bloqueo: {error}");
                return Err(error.into());
            }

            HELD.with(|held| *held.borrow_mut() = Some(lock));

            // Every output is covered now: whoever is waiting to suspend can
            // stop waiting.
            if let Some(mut pipe) = ready {
                use std::io::Write;
                let _ = pipe.write_all(b"1");
                let _ = pipe.flush();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al correr la pantalla de bloqueo");
}
