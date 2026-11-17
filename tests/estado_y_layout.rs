//! Layout reactivo y estado de callbacks (sin abrir ventana).
//!
//! Regresión: `resultado` es keyword del lenguaje (valor de retorno en
//! postcondiciones), así que `escribir(resultado)` se parseaba como
//! `Expresion::Resultado` y la etiqueta quedaba vacía. El estado sí cambiaba,
//! por eso el bug se veía como "los botones no hacen nada".

use forja_gui_rt::evaluador::{ejecutar_funcion, AlmacenGui};
use forja_gui_rt::gui_nativa::{extraer_layout, inicializar_estado, AppStateNativo, Layout};
use forja_syntax::ast::Declaracion;

const PROGRAMA: &str = r#"
importar "gui"

funcion main() {
    variable cuenta = 0
    variable resultado = "0"
    columna(escribir(resultado), boton("+1", &incrementar))
}

funcion incrementar() -> Texto {
    cuenta = cuenta + 1
    retornar a_texto(cuenta)
}
"#;

fn declaraciones(fuente: &str) -> Vec<Declaracion> {
    let mut lexer = forja_syntax::lexer::Lexer::nuevo(fuente);
    let tokens = lexer.tokenize().expect("lexer");
    let mut parser = forja_syntax::parser::Parser::nuevo(tokens);
    parser.parsear().expect("parser").declaraciones
}

fn etiquetas(layout: &Layout, salida: &mut Vec<(String, bool)>) {
    match layout {
        Layout::Label { texto, es_variable } => salida.push((texto.clone(), *es_variable)),
        Layout::Column { children, .. } | Layout::Row { children, .. } => {
            for hijo in children {
                etiquetas(hijo, salida);
            }
        }
        _ => {}
    }
}

#[test]
fn escribir_resultado_es_etiqueta_de_la_variable() {
    let decls = declaraciones(PROGRAMA);
    let mut state = AppStateNativo::nuevo();
    inicializar_estado(&decls, &mut state);
    assert_eq!(
        state.almacen.obtener("resultado"),
        Some(serde_json::json!("0")),
        "main() tiene que dejar las variables en el store"
    );

    let layout = extraer_layout(&decls, &mut state.almacen);
    let mut labels = Vec::new();
    etiquetas(&layout, &mut labels);
    assert!(
        labels
            .iter()
            .any(|(texto, es_variable)| texto == "resultado" && *es_variable),
        "`escribir(resultado)` debe ser una etiqueta de variable, no un DynamicLabel: {labels:?}"
    );
}

#[test]
fn callback_muta_el_estado_y_devuelve_el_texto() {
    let decls = declaraciones(PROGRAMA);
    let mut state = AppStateNativo::nuevo();
    inicializar_estado(&decls, &mut state);

    let valor = ejecutar_funcion(
        "incrementar",
        &[],
        &decls,
        &AlmacenGui(&state.almacen),
    )
    .expect("el callback no debería fallar");

    assert_eq!(valor.to_string(), "1");
    assert_eq!(state.almacen.obtener("cuenta"), Some(serde_json::json!(1)));
}

#[test]
fn el_estado_guardado_pisa_las_variables_de_main() {
    // `forja ejecutar` pasa `--load-state=<json>` (clave `estado:` de forja.yaml):
    // el snapshot tiene que pisar lo que dejó `main()`.
    let decls = declaraciones(PROGRAMA);
    let mut state = AppStateNativo::nuevo();
    inicializar_estado(&decls, &mut state);
    assert_eq!(state.almacen.obtener("cuenta"), Some(serde_json::json!(0)));

    state.cargar_estado(r#"{"cuenta": 7, "resultado": "7"}"#);
    assert_eq!(state.almacen.obtener("cuenta"), Some(serde_json::json!(7)));
    assert_eq!(
        state.almacen.obtener("resultado"),
        Some(serde_json::json!("7"))
    );

    // Y el layout se re-extrae con ese estado: el label sigue siendo la variable.
    let layout = extraer_layout(&decls, &mut state.almacen);
    let mut labels = Vec::new();
    etiquetas(&layout, &mut labels);
    assert!(labels.iter().any(|(texto, _)| texto == "resultado"));
}
