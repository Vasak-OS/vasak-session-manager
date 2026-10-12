//! The ext-session-lock client side, through gtk-session-lock.
//!
//! A lock screen cannot be an ordinary window: the compositor only shows the
//! surfaces created through this protocol while the session is locked, and it
//! is the protocol — not the application — that guarantees nothing else can be
//! seen or clicked until the lock is released. gtk-session-lock is the same
//! library gtklock uses; there is no Rust crate for it, and it is six
//! functions.

use gtk::prelude::*;
use std::ffi::c_void;

// La biblioteca de C va detrás de la feature `session-lock`, que está activada
// por omisión: el paquete de Arch la trae y el binario es el mismo de siempre.
// Ubuntu no la empaqueta, y el CI corre en Ubuntu: ahí las pruebas se compilan
// con `--no-default-features` y estas seis funciones se reemplazan por el
// `ffi` de abajo, que dice que el compositor no soporta el bloqueo. Así se
// compila todo el resto del módulo y sólo queda afuera el enlace
// (Vasak-OS/.github#25).
#[cfg(feature = "session-lock")]
mod ffi {
    use std::ffi::c_void;

    #[link(name = "gtk-session-lock")]
    extern "C" {
        pub fn gtk_session_lock_is_supported() -> glib::ffi::gboolean;
        pub fn gtk_session_lock_prepare_lock() -> *mut c_void;
        pub fn gtk_session_lock_lock_lock(lock: *mut c_void);
        pub fn gtk_session_lock_lock_destroy(lock: *mut c_void);
        pub fn gtk_session_lock_lock_unlock_and_destroy(lock: *mut c_void);
        pub fn gtk_session_lock_lock_new_surface(
            lock: *mut c_void,
            window: *mut gtk::ffi::GtkWindow,
            monitor: *mut gdk::ffi::GdkMonitor,
        );
    }
}

/// Sin la biblioteca: el bloqueo no está disponible y `SessionLock::acquire`
/// devuelve error antes de llamar a ninguna otra. Las demás no se alcanzan, y si
/// algo las llamara igual, falla fuerte en vez de bloquear la sesión a medias.
#[cfg(not(feature = "session-lock"))]
mod ffi {
    use std::ffi::c_void;

    const SIN_BIBLIOTECA: &str = "compilado sin la feature `session-lock`";

    pub unsafe fn gtk_session_lock_is_supported() -> glib::ffi::gboolean {
        glib::ffi::GFALSE
    }
    pub unsafe fn gtk_session_lock_prepare_lock() -> *mut c_void {
        unreachable!("{SIN_BIBLIOTECA}")
    }
    pub unsafe fn gtk_session_lock_lock_lock(_lock: *mut c_void) {
        unreachable!("{SIN_BIBLIOTECA}")
    }
    pub unsafe fn gtk_session_lock_lock_destroy(_lock: *mut c_void) {
        unreachable!("{SIN_BIBLIOTECA}")
    }
    pub unsafe fn gtk_session_lock_lock_unlock_and_destroy(_lock: *mut c_void) {
        unreachable!("{SIN_BIBLIOTECA}")
    }
    pub unsafe fn gtk_session_lock_lock_new_surface(
        _lock: *mut c_void,
        _window: *mut gtk::ffi::GtkWindow,
        _monitor: *mut gdk::ffi::GdkMonitor,
    ) {
        unreachable!("{SIN_BIBLIOTECA}")
    }
}

/// A held session lock.
pub struct SessionLock {
    lock: *mut c_void,
}

impl SessionLock {
    /// Takes the lock. From this point the compositor blanks every output until
    /// surfaces are attached, so there is no instant where the desktop shows.
    pub fn acquire() -> Result<Self, String> {
        unsafe {
            if ffi::gtk_session_lock_is_supported() == 0 {
                return Err("el compositor no implementa ext-session-lock".into());
            }

            let lock = ffi::gtk_session_lock_prepare_lock();
            if lock.is_null() {
                return Err("no se pudo preparar el bloqueo".into());
            }

            // Locking before creating any surface is required, not a
            // preference: gtk-session-lock refuses to build a surface for a lock
            // that has not been taken ("lock_surface_new: assertion
            // 'session_lock' failed"), and the screen ends up locked with
            // nothing drawn on it.
            ffi::gtk_session_lock_lock_lock(lock);
            Ok(Self { lock })
        }
    }

    /// Turns a plain GTK window into the lock surface of one monitor.
    pub fn attach(&self, window: &gtk::Window, monitor: &gdk::Monitor) {
        unsafe {
            ffi::gtk_session_lock_lock_new_surface(self.lock, window.as_ptr(), monitor.as_ptr());
        }
        window.show_all();
    }

    /// Releases the session. Everything the person could not reach comes back.
    pub fn release(self) {
        unsafe { ffi::gtk_session_lock_lock_unlock_and_destroy(self.lock) };
        std::mem::forget(self);
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // Only reached when the lock is dropped without unlocking, which means
        // something failed partway through setting up. Destroying it releases
        // the session rather than leaving a screen nobody can get past with no
        // way to type a password into it.
        unsafe { ffi::gtk_session_lock_lock_destroy(self.lock) };
    }
}

#[cfg(all(test, not(feature = "session-lock")))]
mod tests_without_library {
    use super::SessionLock;

    #[test]
    fn sin_la_feature_tomar_el_bloqueo_devuelve_error_sin_entrar_en_panico() {
        // Sin GTK inicializado y sin compositor: este camino no los necesita,
        // y si el stub dejara pasar a `prepare_lock` el `unreachable!` lo
        // convierte en un pánico, que acá se ataja para decir por qué falla.
        let result = std::panic::catch_unwind(SessionLock::acquire);
        let result = match result {
            Ok(result) => result,
            Err(panic) => {
                let message = panic
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                panic!("acquire entró en pánico en vez de devolver error: {message}");
            }
        };
        match result {
            Err(error) => assert_eq!(error, "el compositor no implementa ext-session-lock"),
            Ok(lock) => {
                std::mem::forget(lock);
                panic!("acquire tomó un bloqueo sin la biblioteca");
            }
        }
    }
}

#[cfg(test)]
mod tests_link_declaration {
    const SOURCE: &str = include_str!("session_lock.rs");
    const MANIFEST: &str = include_str!("../../Cargo.toml");

    #[test]
    fn con_la_feature_el_enlace_con_gtk_session_lock_sigue_declarado() {
        let gate = "#[cfg(feature = \"session-lock\")]\nmod ffi {";
        let start = SOURCE
            .find(gate)
            .expect("el módulo ffi real tiene que ir detrás de la feature `session-lock`");
        let end = SOURCE[start..]
            .find("\n}\n")
            .map(|i| start + i)
            .expect("el módulo ffi real tiene que cerrar");
        let real_ffi = &SOURCE[start..end];
        assert!(
            real_ffi.contains("#[link(name = \"gtk-session-lock\")]"),
            "el módulo ffi con la feature dejó de enlazar gtk-session-lock"
        );
        assert!(real_ffi.contains("extern \"C\""));
    }

    #[test]
    fn la_feature_session_lock_viene_activada_por_omision() {
        let features = MANIFEST
            .split("\n[features]\n")
            .nth(1)
            .expect("Cargo.toml tiene que declarar [features]");
        let features = features.split("\n[").next().unwrap_or(features);
        let default = features
            .lines()
            .find(|line| line.trim_start().starts_with("default"))
            .expect("[features] tiene que declarar `default`");
        assert!(
            default.contains("\"session-lock\""),
            "el paquete de Arch dejaría de enlazar el bloqueo: {default}"
        );
        assert!(features
            .lines()
            .any(|l| l.trim_start().starts_with("session-lock")));
    }
}
