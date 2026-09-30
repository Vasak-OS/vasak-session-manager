//! Que la lista de dependencias del `.deb` sea la de esta aplicación.
//!
//! El campo no existía: el `.deb` salía sin declarar ninguna dependencia, así
//! que se instalaba en cualquier máquina y la pantalla de inicio no arrancaba
//! en ninguna que no tuviera ya WebKitGTK. La lista es la de lo que enlaza
//! `vasak-session-manager`, que es el único binario que el `.deb` lleva, más lo
//! que necesita para ser una pantalla de inicio: greetd, que la lanza y le
//! abre el socket por donde autentica; Cage, el compositor en el que se
//! muestra; y `systemctl`, para apagar y reiniciar.
//!
//! `vasak-lock-screen` (que enlaza `libgtk-session-lock` y `libpam`) y
//! `vasak-config-migrate` los instala la receta de Arch, no el `.deb`: sus
//! bibliotecas no van acá mientras el `.deb` no los lleve.
//!
//! Lo que se declara se audita con `readelf -d … | grep NEEDED`, nunca con
//! `ldd`. Estas pruebas no reemplazan esa auditoría: cuidan que no vuelva lo
//! que se sacó y que no falte lo que se sabe que se enlaza.

use std::path::PathBuf;

fn deb_depends() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", path.display()));
    let config: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{} no es JSON válido: {e}", path.display()));
    config["bundle"]["linux"]["deb"]["depends"]
        .as_array()
        .expect("bundle.linux.deb.depends tiene que existir")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("cada dependencia es un texto")
                .to_string()
        })
        .collect()
}

#[test]
fn las_dependencias_del_deb_tienen_nombre_de_debian() {
    for name in deb_depends() {
        // Los nombres de paquete de Debian: minúsculas, dígitos y `+-.`.
        let valid = name.len() >= 2
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "+-.".contains(c));
        assert!(valid, "«{name}» no es un nombre de paquete de Debian");
        assert!(
            !name.ends_with("-devel") && !name.starts_with("gstreamer1-"),
            "«{name}» es un nombre de Fedora, no de Debian"
        );
        assert!(
            !name.ends_with("-dev"),
            "«{name}» es de compilación: el paquete instalado no lo usa"
        );
    }
}

#[test]
fn las_dependencias_del_deb_no_se_repiten() {
    let mut seen = std::collections::BTreeSet::new();
    for name in deb_depends() {
        assert!(seen.insert(name.clone()), "«{name}» está dos veces");
    }
}

/// El binario enlaza libsoup 3 y nada más: la 2.4, que la plantilla `vapp`
/// les puso a las demás aplicaciones, no tiene que entrar acá tampoco.
#[test]
fn no_viajan_las_dos_generaciones_de_libsoup() {
    assert!(
        !deb_depends().iter().any(|n| n == "libsoup2.4-1"),
        "libsoup2.4-1 no la enlaza nadie: el binario usa libsoup-3.0"
    );
}

/// Lo que `readelf -d` muestra enlazado, con su paquete de Debian. Si una de
/// éstas deja de enlazarse, se saca de la lista y de acá a la vez.
#[test]
fn estan_las_bibliotecas_que_el_binario_enlaza() {
    let depends = deb_depends();
    for (soname, package) in [
        ("libcairo.so.2", "libcairo2"),
        ("libdbus-1.so.3", "libdbus-1-3"),
        ("libgdk_pixbuf-2.0.so.0", "libgdk-pixbuf-2.0-0"),
        ("libglib-2.0.so.0", "libglib2.0-0t64"),
        ("libgtk-3.so.0", "libgtk-3-0t64"),
        (
            "libjavascriptcoregtk-4.1.so.0",
            "libjavascriptcoregtk-4.1-0",
        ),
        ("libsoup-3.0.so.0", "libsoup-3.0-0"),
        ("libwebkit2gtk-4.1.so.0", "libwebkit2gtk-4.1-0"),
    ] {
        assert!(
            depends.iter().any(|n| n == package),
            "el binario enlaza {soname} y el .deb no declara {package}"
        );
    }
}

/// Lo que no se enlaza pero hace falta para que haya pantalla de inicio: greetd
/// la lanza y le da el socket de autenticación, Cage es el compositor en el que
/// se muestra, y `systemctl` apaga y reinicia.
#[test]
fn estan_los_programas_que_se_usan_sin_enlazarlos() {
    let depends = deb_depends();
    for package in ["greetd", "cage", "systemd"] {
        assert!(
            depends.iter().any(|n| n == package),
            "falta {package} en el .deb"
        );
    }
}
