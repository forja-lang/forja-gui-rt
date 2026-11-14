//! Fase 0 de B16: paridad entre la VM y el evaluador de la GUI nativa.
//!
//! Cada programa define `funcion calcular() -> Texto` (sin imprimir nada) y se
//! ejecuta por dos caminos:
//!
//! - **VM** (`forja::ejecutar`): se agrega `imprimir(calcular())` al fuente.
//! - **Evaluador de la GUI** (`forja_gui_rt::evaluador::ejecutar_funcion`).
//!
//! Si los dos textos no coinciden, hay divergencia de dialecto. Las que se
//! conocen y todavía no se arreglaron van en [`DIVERGENCIAS_CONOCIDAS`] con el
//! motivo: el test falla si aparece una **nueva**.
//!
//! Esto es la red de seguridad para poder borrar el evaluador propio de
//! `wasm-gui` (Fase 1) sin cambiar el comportamiento del lenguaje.

use forja_gui_rt::evaluador::ejecutar_funcion;
use forja_gui_rt::gui_nativa::ValorGUI;
use forja_gui_rt::signals::VariableAlmacen;
use forja_syntax::ast::Declaracion;

/// (nombre, fuente, motivo) de los casos que hoy divergen a propósito.
const DIVERGENCIAS_CONOCIDAS: &[(&str, &str)] = &[
    (
        "clases con métodos",
        "el evaluador de la GUI no resuelve métodos declarados dentro de la clase          (falla con «Función 'suma' no encontrada»); lo arregla la Fase 3",
    ),
];

fn declaraciones(fuente: &str) -> Vec<Declaracion> {
    let mut lexer = forja_syntax::lexer::Lexer::nuevo(fuente);
    let tokens = lexer.tokenize().expect("lexer");
    let mut parser = forja_syntax::parser::Parser::nuevo(tokens);
    parser.parsear().expect("parser").declaraciones
}

fn por_la_vm(fuente: &str) -> String {
    let programa = format!("{fuente}\nimprimir(calcular())");
    let salida = forja::ejecutar(&programa).expect("la VM falló");
    salida.join("\n")
}

fn por_la_gui(fuente: &str) -> Result<String, String> {
    let decls = declaraciones(fuente);
    let mut almacen = VariableAlmacen::nuevo();
    let valor = ejecutar_funcion("calcular", &[], &decls, &mut almacen)?;
    Ok(match valor {
        ValorGUI::Texto(texto) => texto,
        otro => format!("{:?}", otro),
    })
}

fn comprobar_paridad(nombre: &str, fuente: &str) {
    let de_vm = por_la_vm(fuente);
    let de_gui = por_la_gui(fuente);
    let conocido = DIVERGENCIAS_CONOCIDAS.iter().any(|(n, _)| *n == nombre);

    if conocido {
        // Se documenta la divergencia; si un día se arregla, el test avisa para
        // sacar la entrada de la lista.
        let coincide = matches!(&de_gui, Ok(texto) if texto == &de_vm);
        assert!(
            !coincide,
            "{nombre}: figura como divergencia conocida pero ya coincide;              quitá la entrada de DIVERGENCIAS_CONOCIDAS"
        );
        return;
    }
    match de_gui {
        Ok(texto) => assert_eq!(
            de_vm, texto,
            "{nombre}: la VM y la GUI dan resultados distintos (divergencia de dialecto)"
        ),
        Err(error) => panic!("{nombre}: el evaluador de la GUI falló: {error}"),
    }
}

#[test]
fn aritmetica_entera() {
    comprobar_paridad(
        "aritmetica entera",
        r#"
funcion calcular() -> Texto {
    variable a = 2 + 3 * 4
    variable b = (2 + 3) * 4
    retornar a_texto(a) + "|" + a_texto(b) + "|" + a_texto(7 / 2) + "|" + a_texto(7 % 2)
}
"#,
    );
}

#[test]
fn decimales() {
    comprobar_paridad(
        "decimales",
        r#"
funcion calcular() -> Texto {
    variable x = 1.5
    variable y = 2.25
    retornar a_texto(x + y) + "|" + a_texto(y - x) + "|" + a_texto(x * 2.0)
}
"#,
    );
}

#[test]
fn texto_y_longitud() {
    comprobar_paridad(
        "texto",
        r#"
funcion calcular() -> Texto {
    variable saludo = "hola" + " " + "mundo"
    retornar saludo + "|" + a_texto(longitud(saludo))
}
"#,
    );
}

#[test]
fn condicionales_y_booleans() {
    comprobar_paridad(
        "si y booleanos",
        r#"
funcion calcular() -> Texto {
    variable n = 7
    variable salida = ""
    si (n > 5) {
        salida = "grande"
    } sino {
        salida = "chico"
    }
    si (n == 7 && verdadero) {
        salida = salida + "|siete"
    }
    retornar salida
}
"#,
    );
}

#[test]
fn bucle_mientras_con_corte() {
    comprobar_paridad(
        "mientras y romper",
        r#"
funcion calcular() -> Texto {
    variable i = 0
    variable suma = 0
    mientras (verdadero) {
        si (i >= 5) {
            romper
        }
        suma = suma + i
        i = i + 1
    }
    retornar a_texto(suma)
}
"#,
    );
}

#[test]
fn bucle_para_y_arreglos() {
    comprobar_paridad(
        "empujar y arreglos",
        r#"
funcion calcular() -> Texto {
    variable xs = [10, 20, 30]
    variable total = 0
    variable i = 0
    mientras (i < longitud(xs)) {
        total = total + xs[i]
        i = i + 1
    }
    empujar(xs, 40)
    retornar a_texto(total) + "|" + a_texto(longitud(xs))
}
"#,
    );
}

#[test]
fn mapas() {
    comprobar_paridad(
        "mapas",
        r#"
funcion calcular() -> Texto {
    variable m = {"a": 1, "b": 2}
    m["c"] = 3
    retornar a_texto(m["a"] + m["c"])
}
"#,
    );
}

#[test]
fn funciones_y_recursion() {
    comprobar_paridad(
        "recursión",
        r#"
funcion factorial(n) {
    si (n <= 1) {
        retornar 1
    }
    retornar n * factorial(n - 1)
}

funcion calcular() -> Texto {
    retornar a_texto(factorial(6))
}
"#,
    );
}

#[test]
fn coincidir() {
    comprobar_paridad(
        "coincidir",
        r#"
funcion calcular() -> Texto {
    variable n = 2
    variable salida = ""
    coincidir (n) {
        caso 1 -> { salida = "uno" }
        caso 2 -> { salida = "dos" }
        caso _ -> { salida = "otro" }
    }
    retornar salida
}
"#,
    );
}

#[test]
fn clases_y_campos() {
    comprobar_paridad(
        "clases con métodos",
        r#"
clase Punto {
    x
    y

    constructor(x, y) {
        este.x = x
        este.y = y
    }

    funcion suma() {
        retornar este.x + este.y
    }
}

funcion calcular() -> Texto {
    variable p = nuevo Punto(2, 3)
    retornar a_texto(p.suma())
}
"#,
    );
}

#[test]
fn asignacion_por_indice() {
    comprobar_paridad(
        "asignación por índice",
        r#"
funcion calcular() -> Texto {
    variable xs = [1, 2, 3]
    xs[1] = 9
    retornar a_texto(xs[1])
}
"#,
    );
}

#[test]
fn remover_elemento() {
    comprobar_paridad(
        "remover",
        r#"
funcion calcular() -> Texto {
    variable xs = [1, 2, 3]
    remover(xs, 0)
    retornar a_texto(longitud(xs))
}
"#,
    );
}
