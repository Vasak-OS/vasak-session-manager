//! Checking the password of the person whose session this is.
//!
//! The greeter never does this: it hands the password to greetd, which owns
//! PAM, the seat and the session. A lock screen has no such helper — the
//! session already exists and is ours — so it authenticates directly against
//! PAM, the same way swaylock does.

use zeroize::Zeroizing;

/// The PAM service this authenticates against, and the only one.
///
/// There is deliberately no fallback to `login`. That stack decides whether
/// somebody may *enter* the system: it pulls in pam_nologin and pam_securetty,
/// which have every reason to refuse a person who is already logged in and only
/// wants their own session back. Falling back to it would trade a lock that
/// refuses to start for a lock that refuses the right password, and the second
/// one costs a reboot.
const SERVICE: &str = "vasak-lock-screen";

fn service_path() -> String {
    format!("/etc/pam.d/{SERVICE}")
}

/// Whether there is any chance of authenticating at all.
///
/// Checked before the session is locked, and the lock is refused if it fails.
/// A lock screen that cannot verify a password is not a lock screen: it is a
/// machine nobody can get back into, and the only way out is the power button.
/// That is exactly what happened while this was being written — PAM answered
/// `Auth_Err` to the right password because the service file was missing — and
/// no amount of care while testing is a substitute for the program refusing to
/// put somebody in that position.
pub fn can_authenticate() -> Result<(), String> {
    if std::path::Path::new(&service_path()).is_file() {
        return Ok(());
    }

    Err(format!(
        "falta {}: sin ese archivo PAM rechaza cualquier contraseña y la sesión \
         quedaría inaccesible. Se instala con el paquete vasak-session-manager.",
        service_path()
    ))
}

/// Whether this password opens this session.
///
/// Only ever answers yes or no: telling apart "wrong password" from "PAM is
/// misconfigured" on a lock screen helps whoever is standing in front of a
/// machine that is not theirs more than it helps its owner. The difference goes
/// to the log, which only the owner can read afterwards.
pub fn verify(user: &str, password: &str) -> bool {
    let password = Zeroizing::new(password.to_string());

    let mut client = match pam::Client::with_password(SERVICE) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("[lock] no se pudo abrir el servicio PAM '{SERVICE}': {error}");
            return false;
        }
    };

    client
        .conversation_mut()
        .set_credentials(user, password.as_str());

    match client.authenticate() {
        Ok(()) => true,
        Err(error) => {
            eprintln!("[lock] autenticación rechazada por '{SERVICE}': {error}");
            false
        }
    }
}

/// The account this session belongs to.
pub fn current_user() -> String {
    if let Ok(user) = std::env::var("USER") {
        if !user.is_empty() {
            return user;
        }
    }

    // A session with no USER in the environment is odd but not fatal: the uid
    // is what PAM is going to be asked about anyway.
    unsafe {
        let entry = libc::getpwuid(libc::getuid());
        if entry.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr((*entry).pw_name)
                .to_string_lossy()
                .into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a formality: this is the check that decides whether the screen is
    /// allowed to cover the session, and getting it wrong costs a reboot.
    #[test]
    fn without_the_service_file_it_refuses() {
        let installed = std::path::Path::new(&service_path()).is_file();
        match (can_authenticate(), installed) {
            (Ok(()), true) => {}
            (Err(message), false) => assert!(message.contains("/etc/pam.d/vasak-lock-screen")),
            (Ok(()), false) => panic!("aceptó bloquear sin servicio PAM"),
            (Err(e), true) => panic!("rechazó con el servicio instalado: {e}"),
        }
    }
}
