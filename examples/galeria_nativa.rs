// Abre una ventana real con la galería de componentes, para verificación visual.
//
// Ejecutar con:
//     cargo run --example galeria_nativa -p forja-gui-rt
//
// El layout sale de `ejemplos/208_gui_galeria.fa`, el mismo archivo que ejecuta
// `forja ejecutar --native`. El diseño (Fluent / Liquid Glass, claro u oscuro)
// lo decide el `tema_*` con el que arranca ese archivo.

use forja_gui_rt::gui_nativa::{build_and_run, extraer_layout};

fn main() -> Result<(), String> {
    // FORJA_FUENTE=ruta permite abrir otro .fa (diagnóstico).
    let ruta = std::env::var("FORJA_FUENTE").ok();
    let fuente: &str = match ruta.as_deref() {
        Some(p) => Box::leak(
            std::fs::read_to_string(p)
                .map_err(|e| e.to_string())?
                .into_boxed_str(),
        ),
        None => include_str!("../../../ejemplos/208_gui_galeria.fa"),
    };
    // Ayuda de verificación visual: FORJA_DISENO=fluent|liquid_glass|... cambia el
    // `tema_fluent(...)` del ejemplo sin tocar el archivo.
    let diseno = std::env::var("FORJA_DISENO").unwrap_or_default();
    let fuente: String = if diseno.is_empty() {
        fuente.to_string()
    } else {
        fuente.replace("tema_fluent(", &format!("tema_{}(", diseno))
    };
    let mut lexer = forja_syntax::lexer::Lexer::new(&fuente);
    let tokens = lexer.tokenize().map_err(|e| format!("{:?}", e))?;
    let mut parser = forja_syntax::parser::Parser::new(tokens);
    let programa = parser.parse().map_err(|e| format!("{:?}", e))?;

    // Chequeo previo sin ventana: si el layout sale vacío, no tiene sentido abrir.
    let mut store = forja_gui_rt::VariableStore::new();
    let layout = extraer_layout(&programa.declaraciones, &mut store);
    eprintln!("[ejemplo] raíz del layout: {:?}", layout);

    build_and_run(&programa, None, None, true)
}
