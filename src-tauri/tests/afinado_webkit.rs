//! Que el afinado del WebKit siga cableado de punta a punta.
//!
//! El glue vive en `commands/webkit.rs` —excluido de la cobertura porque no se
//! puede ejercitar sin un display ni un WebKit reales—, así que esta prueba lee
//! el fuente y comprueba que las palancas y el cableado no se caigan en un
//! refactor: las palancas en `webkit.rs`, el registro del comando en `lib.rs` y
//! la llamada desde el frontend en `App.vue`. No arranca GTK.

use std::path::PathBuf;

fn read(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", path.display()))
}

#[test]
fn el_glue_conserva_las_palancas_del_afinado() {
    let webkit = read("src/commands/webkit.rs");
    assert!(
        webkit.contains("pub fn tune_webview"),
        "falta el comando `tune_webview` en commands/webkit.rs"
    );
    assert!(
        webkit.contains("set_enable_developer_extras(false)"),
        "falta apagar los extras de desarrollo"
    );
    assert!(
        webkit.contains("set_enable_back_forward_navigation_gestures(false)"),
        "falta apagar los gestos de navegación atrás/adelante"
    );
    assert!(
        webkit.contains("CacheModel::DocumentViewer"),
        "falta el caché de visor de documento"
    );
}

#[test]
fn el_comando_esta_registrado_en_lib() {
    let lib = read("src/lib.rs");
    assert!(
        lib.contains("webkit::tune_webview"),
        "el comando `tune_webview` no está en el generate_handler de lib.rs"
    );
}

#[test]
fn el_frontend_dispara_el_afinado_al_montar() {
    let app = read("../src/App.vue");
    assert!(
        app.contains("invoke(\"tune_webview\")"),
        "App.vue no llama a `invoke(\"tune_webview\")` al montar"
    );
}
