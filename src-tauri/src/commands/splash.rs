use std::process::Command;

/// Avisarle al splash de arranque (vasak-splash) que el greeter ya pintó su
/// primer frame: el daemon se retira y le entrega la pantalla sin esperar su
/// temporizador de seguridad de 20 s. Va por el cliente `vasak-splashd quit`, que
/// escribe el socket de control `/run/vasak-splash.sock`; el greeter corre como
/// `greeter`, que el paquete pone en el grupo `vasak-splash` para poder hacerlo.
///
/// Best-effort y silencioso: si no hay splash (arranque sin él, o ya se retiró),
/// el cliente falla al contactar el socket y no hay nada que hacer —no es un error
/// del greeter, y la red de seguridad del propio splash cubre el caso—. Se usa la
/// ruta absoluta porque greetd arranca el greeter con un entorno mínimo donde no
/// se puede contar con el `PATH`.
#[tauri::command]
pub fn dismiss_boot_splash() {
    let _ = Command::new("/usr/bin/vasak-splashd").arg("quit").status();
}
