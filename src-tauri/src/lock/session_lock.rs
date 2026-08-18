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
