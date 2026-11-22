//! El cableado del afinado del WebKit en las dos pantallas del paquete.
//!
//! Se mira sobre el texto de los fuentes y no sobre el programa corriendo porque
//! el afinado baja al `WebKitWebView` por el árbol de GTK y no se puede ejercitar
//! sin un display ni un WebKit reales. Lo que hay que sostener es que las
//! palancas sigan puestas, que el comando siga registrado en el greeter y en el
//! bloqueo, y que ambas vistas lo invoquen al montar: si alguno se cae, el
//! afinado deja de aplicarse sin que nada falle al compilar.

const WEBKIT: &str = include_str!("../src/commands/webkit.rs");
const LIB: &str = include_str!("../src/lib.rs");
const LOCK: &str = include_str!("../src/lock/mod.rs");
const APP_VUE: &str = include_str!("../../src/App.vue");
const LOCK_VUE: &str = include_str!("../../src/LockView.vue");

fn contiene(texto: &str, aguja: &str, donde: &str) {
    assert!(
        texto.contains(aguja),
        "no aparece «{aguja}» en {donde}. Si el afinado del WebKit se tocó, hay \
         que actualizar esta prueba; si se borró, leer el encabezado antes."
    );
}

/// El cuerpo de `generate_handler![ ... ]`, para no dar por registrado un
/// comando que sólo se nombra en un comentario.
fn handler(texto: &str) -> &str {
    let desde = texto
        .find("generate_handler![")
        .expect("sin generate_handler!");
    let resto = &texto[desde..];
    &resto[..resto.find("])").expect("generate_handler! sin cerrar")]
}

#[test]
fn el_comando_expone_el_afinado() {
    contiene(WEBKIT, "pub fn tune_webview", "commands/webkit.rs");
}

#[test]
fn las_tres_palancas_seguras_siguen_puestas() {
    contiene(
        WEBKIT,
        "set_enable_developer_extras(false)",
        "commands/webkit.rs",
    );
    contiene(
        WEBKIT,
        "set_enable_back_forward_navigation_gestures(false)",
        "commands/webkit.rs",
    );
    contiene(WEBKIT, "CacheModel::DocumentViewer", "commands/webkit.rs");
}

#[test]
fn no_apaga_la_aceleracion_ni_webgl_porque_las_pantallas_animan() {
    for prohibido in ["set_hardware_acceleration_policy", "set_enable_webgl"] {
        assert!(
            !WEBKIT.contains(prohibido),
            "commands/webkit.rs no debe usar «{prohibido}»: el greeter y el bloqueo animan su UI"
        );
    }
}

#[test]
fn el_comando_esta_registrado_en_el_greeter() {
    contiene(
        handler(LIB),
        "webkit::tune_webview",
        "el generate_handler! de lib.rs",
    );
}

#[test]
fn el_comando_esta_registrado_en_el_bloqueo() {
    contiene(
        handler(LOCK),
        "webkit::tune_webview",
        "el generate_handler! de lock/mod.rs",
    );
}

#[test]
fn el_greeter_lo_invoca_al_montar() {
    contiene(APP_VUE, "invoke(\"tune_webview\")", "App.vue");
}

#[test]
fn el_bloqueo_lo_invoca_al_montar() {
    contiene(LOCK_VUE, "invoke(\"tune_webview\")", "LockView.vue");
}
