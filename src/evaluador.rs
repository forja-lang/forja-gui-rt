// Forja GUI Runtime — Evaluador Tree-Walking Completo
//
// Reemplaza las funciones parciales ejecutar_callback_forja / evaluar_bloque / evaluar_expresion
// con un intérprete tree-walking que soporta TODO el AST de Forja.
//
// Arquitectura:
//   - Ambito:     ámbito de variables local (HashMap<String, ValorGUI>)
//   - ejecutar_funcion: punto de entrada para ejecutar una función Forja
//   - evaluar_bloque:   evalúa una lista de declaraciones
//   - evaluar_expresion: evalúa una expresión y retorna ValorGUI
//
// Integración:
//   - Escribe variables en VariableStore para que los widgets las vean
//   - Lee variables de VariableStore cuando no están en ámbito local
//   - Soporta llamadas a funciones definidas en el AST

use crate::gui_nativa::ValorGUI;
use crate::signals::VariableStore;
use forja::ast::*;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Sub};

// ─── Operadores aritméticos para ValorGUI ────────────────────────

impl Add for ValorGUI {
    type Output = ValorGUI;
    fn add(self, other: ValorGUI) -> ValorGUI {
        match (self, other) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => ValorGUI::Entero(a + b),
            (ValorGUI::Decimal(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a + b),
            (ValorGUI::Entero(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a as f64 + b),
            (ValorGUI::Decimal(a), ValorGUI::Entero(b)) => ValorGUI::Decimal(a + b as f64),
            (ValorGUI::Texto(a), ValorGUI::Texto(b)) => ValorGUI::Texto(a + &b),
            (ValorGUI::Texto(a), b) => ValorGUI::Texto(a + &b.to_string()),
            (a, ValorGUI::Texto(b)) => ValorGUI::Texto(a.to_string() + &b),
            _ => ValorGUI::Nulo,
        }
    }
}

impl Sub for ValorGUI {
    type Output = ValorGUI;
    fn sub(self, other: ValorGUI) -> ValorGUI {
        match (self, other) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => ValorGUI::Entero(a - b),
            (ValorGUI::Decimal(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a - b),
            (ValorGUI::Entero(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a as f64 - b),
            (ValorGUI::Decimal(a), ValorGUI::Entero(b)) => ValorGUI::Decimal(a - b as f64),
            _ => ValorGUI::Nulo,
        }
    }
}

impl Mul for ValorGUI {
    type Output = ValorGUI;
    fn mul(self, other: ValorGUI) -> ValorGUI {
        match (self, other) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => ValorGUI::Entero(a * b),
            (ValorGUI::Decimal(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a * b),
            (ValorGUI::Entero(a), ValorGUI::Decimal(b)) => ValorGUI::Decimal(a as f64 * b),
            (ValorGUI::Decimal(a), ValorGUI::Entero(b)) => ValorGUI::Decimal(a * b as f64),
            _ => ValorGUI::Nulo,
        }
    }
}

impl Div for ValorGUI {
    type Output = ValorGUI;
    fn div(self, other: ValorGUI) -> ValorGUI {
        match (self, other) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => {
                if b == 0 {
                    ValorGUI::Nulo
                } else {
                    ValorGUI::Entero(a / b)
                }
            }
            (ValorGUI::Decimal(a), ValorGUI::Decimal(b)) => {
                if b == 0.0 {
                    ValorGUI::Nulo
                } else {
                    ValorGUI::Decimal(a / b)
                }
            }
            (ValorGUI::Entero(a), ValorGUI::Decimal(b)) => {
                if b == 0.0 {
                    ValorGUI::Nulo
                } else {
                    ValorGUI::Decimal(a as f64 / b)
                }
            }
            (ValorGUI::Decimal(a), ValorGUI::Entero(b)) => {
                if b == 0 {
                    ValorGUI::Nulo
                } else {
                    ValorGUI::Decimal(a / b as f64)
                }
            }
            _ => ValorGUI::Nulo,
        }
    }
}

impl PartialEq for ValorGUI {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => a == b,
            (ValorGUI::Decimal(a), ValorGUI::Decimal(b)) => (a - b).abs() < f64::EPSILON,
            (ValorGUI::Texto(a), ValorGUI::Texto(b)) => a == b,
            (ValorGUI::Booleano(a), ValorGUI::Booleano(b)) => a == b,
            (ValorGUI::Nulo, ValorGUI::Nulo) => true,
            // Cross-type: convertir a f64 para comparación numérica
            (ValorGUI::Entero(a), ValorGUI::Decimal(b)) => *a as f64 == *b,
            (ValorGUI::Decimal(a), ValorGUI::Entero(b)) => *a == *b as f64,
            _ => false,
        }
    }
}

// ─── Helpers de ValorGUI ────────────────────────────────────────

impl ValorGUI {
    /// Evalúa si el valor es "verdadero" en contexto booleano
    pub fn es_verdadero(&self) -> bool {
        match self {
            ValorGUI::Booleano(b) => *b,
            ValorGUI::Entero(n) => *n != 0,
            ValorGUI::Decimal(n) => *n != 0.0,
            ValorGUI::Texto(t) => !t.is_empty(),
            ValorGUI::Nulo => false,
        }
    }

    /// Convierte a serde_json::Value para almacenar en VariableStore
    pub fn to_json_value(&self) -> serde_json::Value {
        match self {
            ValorGUI::Texto(t) => serde_json::Value::String(t.clone()),
            ValorGUI::Entero(n) => serde_json::Value::Number((*n).into()),
            ValorGUI::Decimal(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            ValorGUI::Booleano(b) => serde_json::Value::Bool(*b),
            ValorGUI::Nulo => serde_json::Value::Null,
        }
    }

    /// Convierte desde serde_json::Value
    pub fn from_serde(val: &serde_json::Value) -> Self {
        match val {
            serde_json::Value::String(s) => ValorGUI::Texto(s.clone()),
            serde_json::Value::Number(n) => n
                .as_i64()
                .map(ValorGUI::Entero)
                .or_else(|| n.as_f64().map(ValorGUI::Decimal))
                .unwrap_or(ValorGUI::Nulo),
            serde_json::Value::Bool(b) => ValorGUI::Booleano(*b),
            serde_json::Value::Array(arr) => arr
                .first()
                .map(|v| ValorGUI::from_serde(v))
                .unwrap_or(ValorGUI::Nulo),
            _ => ValorGUI::Nulo,
        }
    }

    /// Comparación ordenada para <, <=, >, >=
    pub fn compare(&self, op: &Operador, other: &ValorGUI) -> bool {
        let a = self.to_f64();
        let b = other.to_f64();
        match op {
            Operador::Menor => a < b,
            Operador::MenorIgual => a <= b,
            Operador::Mayor => a > b,
            Operador::MayorIgual => a >= b,
            _ => false,
        }
    }

    /// Convierte el valor a String (display)
    pub fn to_display(&self) -> String {
        match self {
            ValorGUI::Texto(s) => s.clone(),
            ValorGUI::Entero(n) => n.to_string(),
            ValorGUI::Decimal(f) => f.to_string(),
            ValorGUI::Booleano(b) => if *b { "verdadero" } else { "falso" }.to_string(),
            ValorGUI::Nulo => "nulo".to_string(),
        }
    }

    /// Convierte a lista de valores para iteración (para bucles `para`)
    pub fn a_lista(&self) -> Vec<ValorGUI> {
        match self {
            ValorGUI::Entero(n) => {
                if *n > 0 {
                    (0..*n).map(|i| ValorGUI::Entero(i)).collect()
                } else {
                    vec![]
                }
            }
            ValorGUI::Texto(t) => {
                if let Ok(serde_json::Value::Array(arr)) =
                    serde_json::from_str::<serde_json::Value>(t)
                {
                    arr.iter().map(|v| ValorGUI::from_serde(v)).collect()
                } else {
                    vec![self.clone()]
                }
            }
            _ => vec![self.clone()],
        }
    }
}

// ─── Ámbito de variables ────────────────────────────────────────

/// Ámbito de variables locales para la evaluación de una función
pub struct Ambito {
    variables: HashMap<String, ValorGUI>,
}

impl Ambito {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn obtener(&self, nombre: &str) -> Option<&ValorGUI> {
        self.variables.get(nombre)
    }

    pub fn asignar(&mut self, nombre: String, valor: ValorGUI) {
        self.variables.insert(nombre, valor);
    }

    pub fn contiene(&self, nombre: &str) -> bool {
        self.variables.contains_key(nombre)
    }
}

// ─── Evaluación de funciones ─────────────────────────────────────

/// Evalúa una función Forja a partir del AST
pub fn ejecutar_funcion(
    nombre: &str,
    args: &[ValorGUI],
    declaraciones: &[Declaracion],
    store: &mut VariableStore,
) -> Result<ValorGUI, String> {
    // Buscar la función en las declaraciones
    buscar_funcion(nombre, declaraciones).and_then(|func| {
        // Extraer los campos de la función
        let (parametros, cuerpo) = match func {
            Declaracion::Funcion {
                parametros, cuerpo, ..
            } => (parametros, cuerpo),
            _ => return Err(format!("'{}' no es una función", nombre)),
        };
        let mut ambito = Ambito::new();

        // Asignar parámetros: args explícitos primero, luego cargar desde store
        for (i, param) in parametros.iter().enumerate() {
            if let Some(val) = args.get(i) {
                ambito.asignar(param.nombre.clone(), val.clone());
            } else if let Some(json_val) = store.get(&param.nombre) {
                ambito.asignar(param.nombre.clone(), ValorGUI::from_serde(&json_val));
            }
        }

        // Evaluar cuerpo
        evaluar_bloque(cuerpo, &mut ambito, store, declaraciones)
    })
}

/// Busca una declaración de función por nombre
fn buscar_funcion<'a>(
    nombre: &str,
    declaraciones: &'a [Declaracion],
) -> Result<&'a Declaracion, String> {
    for d in declaraciones {
        if let Declaracion::Funcion { nombre: ref n, .. } = d {
            if n == nombre {
                return Ok(d);
            }
        }
    }
    Err(format!("Función '{}' no encontrada", nombre))
}

// ─── Evaluación de bloques ──────────────────────────────────────

/// Evalúa un bloque de declaraciones.
/// Retorna Ok(ValorGUI) si encuentra un `retornar`, o Ok(ValorGUI::Nulo) al final.
fn evaluar_bloque(
    bloque: &[Declaracion],
    ambito: &mut Ambito,
    store: &mut VariableStore,
    declaraciones: &[Declaracion],
) -> Result<ValorGUI, String> {
    for declaracion in bloque {
        let result = evaluar_declaracion(declaracion, ambito, store, declaraciones)?;
        // Si la declaración fue un retornar, propagar el valor
        if es_retorno(declaracion) {
            return Ok(result);
        }
    }
    Ok(ValorGUI::Nulo)
}

/// Indica si una declaración es de tipo retornar
fn es_retorno(decl: &Declaracion) -> bool {
    matches!(decl, Declaracion::Retornar { .. })
}

/// Evalúa una declaración individual.
/// Retorna el valor si es una declaración de retorno, o ValorGUI::Nulo en otro caso.
fn evaluar_declaracion(
    decl: &Declaracion,
    ambito: &mut Ambito,
    store: &mut VariableStore,
    declaraciones: &[Declaracion],
) -> Result<ValorGUI, String> {
    match decl {
        Declaracion::Retornar { valor } => {
            if let Some(expr) = valor {
                evaluar_expresion(expr, ambito, store, declaraciones)
            } else {
                Ok(ValorGUI::Nulo)
            }
        }

        Declaracion::Si {
            condicion,
            bloque_verdadero,
            bloque_falso,
        } => {
            let cond_val = evaluar_expresion(condicion, ambito, store, declaraciones)?;
            if cond_val.es_verdadero() {
                evaluar_bloque(bloque_verdadero, ambito, store, declaraciones)
            } else if let Some(sino_bloque) = bloque_falso {
                evaluar_bloque(sino_bloque, ambito, store, declaraciones)
            } else {
                Ok(ValorGUI::Nulo)
            }
        }

        Declaracion::Mientras { condicion, bloque } => {
            loop {
                let cond_val = evaluar_expresion(condicion, ambito, store, declaraciones)?;
                if !cond_val.es_verdadero() {
                    break;
                }
                let result = evaluar_bloque(bloque, ambito, store, declaraciones)?;
                if !matches!(result, ValorGUI::Nulo) {
                    return Ok(result);
                }
            }
            Ok(ValorGUI::Nulo)
        }

        Declaracion::Para {
            inicializacion,
            condicion,
            incremento,
            bloque,
        } => {
            // Estilo C: for (init; cond; inc) { cuerpo }
            if let Some(init) = inicializacion {
                evaluar_declaracion(init, ambito, store, declaraciones)?;
            }
            loop {
                if let Some(cond) = condicion {
                    let cond_val = evaluar_expresion(cond, ambito, store, declaraciones)?;
                    if !cond_val.es_verdadero() {
                        break;
                    }
                }
                let result = evaluar_bloque(bloque, ambito, store, declaraciones)?;
                if !matches!(result, ValorGUI::Nulo) {
                    return Ok(result);
                }
                if let Some(inc) = incremento {
                    evaluar_declaracion(inc, ambito, store, declaraciones)?;
                }
            }
            Ok(ValorGUI::Nulo)
        }

        Declaracion::Repetir { cantidad, bloque } => {
            let veces = evaluar_expresion(cantidad, ambito, store, declaraciones)?;
            let n = match veces {
                ValorGUI::Entero(n) if n > 0 => n as usize,
                _ => 0,
            };
            for _ in 0..n {
                let result = evaluar_bloque(bloque, ambito, store, declaraciones)?;
                if !matches!(result, ValorGUI::Nulo) {
                    return Ok(result);
                }
            }
            Ok(ValorGUI::Nulo)
        }

        Declaracion::Variable { nombre, valor, .. } => {
            let val = if let Some(expr) = valor {
                evaluar_expresion(expr, ambito, store, declaraciones)?
            } else {
                ValorGUI::Nulo
            };
            ambito.asignar(nombre.clone(), val.clone());
            store.set(nombre, val.to_json_value());
            Ok(ValorGUI::Nulo)
        }

        Declaracion::Asignacion { nombre, valor, .. } => {
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            ambito.asignar(nombre.clone(), val.clone());
            store.set(nombre, val.to_json_value());
            Ok(ValorGUI::Nulo)
        }

        Declaracion::AsignacionMiembro {
            objeto,
            miembro,
            valor,
            ..
        } => {
            let obj_val = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            let key = format!("{}_{}", obj_val.to_display(), miembro);
            store.set(&key, val.to_json_value());
            Ok(ValorGUI::Nulo)
        }

        Declaracion::AsignacionIndex {
            nombre,
            indice,
            valor,
            ..
        } => {
            let idx_val = evaluar_expresion(indice, ambito, store, declaraciones)?;
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            let key = format!("{}[{}]", nombre, idx_val.to_display());
            store.set(&key, val.to_json_value());
            Ok(ValorGUI::Nulo)
        }

        Declaracion::LlamadaFuncion { nombre, argumentos } => {
            let mut args = Vec::new();
            for arg in argumentos {
                args.push(evaluar_expresion(arg, ambito, store, declaraciones)?);
            }
            ejecutar_funcion(nombre, &args, declaraciones, store)
        }

        Declaracion::Expresion(expr) => {
            evaluar_expresion(expr, ambito, store, declaraciones)?;
            Ok(ValorGUI::Nulo)
        }

        Declaracion::Cuando {
            condicion, cuerpo, ..
        } => {
            let cond_val = evaluar_expresion(condicion, ambito, store, declaraciones)?;
            if cond_val.es_verdadero() {
                evaluar_bloque(cuerpo, ambito, store, declaraciones)
            } else {
                Ok(ValorGUI::Nulo)
            }
        }

        Declaracion::AsignacionMultiple {
            variables, valor, ..
        } => {
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            for var in variables {
                ambito.asignar(var.clone(), val.clone());
                store.set(var, val.to_json_value());
            }
            Ok(ValorGUI::Nulo)
        }

        // Declaraciones que se ignoran en runtime
        Declaracion::Funcion { .. }
        | Declaracion::Clase { .. }
        | Declaracion::Importar(_)
        | Declaracion::Enum { .. }
        | Declaracion::Rasgo { .. }
        | Declaracion::Implementacion { .. }
        | Declaracion::AccesoMiembro { .. } => Ok(ValorGUI::Nulo),
    }
}

// ─── Evaluación de expresiones ──────────────────────────────────

/// Evalúa una expresión y retorna su valor como ValorGUI
fn evaluar_expresion(
    expr: &Expresion,
    ambito: &mut Ambito,
    store: &mut VariableStore,
    declaraciones: &[Declaracion],
) -> Result<ValorGUI, String> {
    match expr {
        // ── Literales ──────────────────────────────────────────
        Expresion::LiteralNumero(n) => Ok(ValorGUI::Entero(*n)),
        Expresion::LiteralDecimal(f) => Ok(ValorGUI::Decimal(*f)),
        Expresion::LiteralTexto(s) => Ok(ValorGUI::Texto(s.clone())),
        Expresion::LiteralBooleano(b) => Ok(ValorGUI::Booleano(*b)),
        Expresion::LiteralNulo => Ok(ValorGUI::Nulo),
        Expresion::LiteralExacto(coeff, scale) => {
            let val = *coeff as f64 / (10f64).powi(*scale as i32);
            Ok(ValorGUI::Decimal(val))
        }

        // ── Identificador ──────────────────────────────────────
        Expresion::Identificador { nombre, .. } => {
            if let Some(val) = ambito.obtener(nombre) {
                Ok(val.clone())
            } else if let Some(json_val) = store.get(nombre) {
                Ok(ValorGUI::from_serde(&json_val))
            } else {
                Err(format!("Variable '{}' no encontrada", nombre))
            }
        }

        // ── Operaciones binarias ────────────────────────────────
        Expresion::Binaria {
            izquierda,
            operador,
            derecha,
        } => {
            let izq = evaluar_expresion(izquierda, ambito, store, declaraciones)?;
            let der = evaluar_expresion(derecha, ambito, store, declaraciones)?;
            evaluar_binaria(izq, operador, der)
        }

        // ── Operaciones unarias ────────────────────────────────
        Expresion::Unaria {
            operador,
            expr: inner,
        } => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
            match operador {
                OperadorUnario::Negar => match val {
                    ValorGUI::Entero(n) => Ok(ValorGUI::Entero(-n)),
                    ValorGUI::Decimal(f) => Ok(ValorGUI::Decimal(-f)),
                    _ => Err("No se puede negar un valor no numérico".to_string()),
                },
                OperadorUnario::No => Ok(ValorGUI::Booleano(!val.es_verdadero())),
            }
        }

        // ── Llamada a función ─────────────────────────────────
        Expresion::LlamadaFuncion { nombre, argumentos } => {
            let mut args = Vec::new();
            for arg in argumentos {
                args.push(evaluar_expresion(arg, ambito, store, declaraciones)?);
            }
            ejecutar_funcion(nombre, &args, declaraciones, store)
        }

        // ── Acceso a miembro ──────────────────────────────────
        Expresion::AccesoMiembro { objeto, miembro } => {
            let obj = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            let key = format!("{}_{}", obj.to_display(), miembro);
            if let Some(json_val) = store.get(&key) {
                Ok(ValorGUI::from_serde(&json_val))
            } else {
                Ok(ValorGUI::Nulo)
            }
        }

        // ── Instanciación ─────────────────────────────────────
        Expresion::Instanciacion { .. } => {
            Err("Instanciación de clases no soportada en runtime GUI".to_string())
        }

        // ── Referencia (préstamo) ─────────────────────────────
        Expresion::Referencia { expr: inner, .. } => {
            evaluar_expresion(inner, ambito, store, declaraciones)
        }

        // ── Arreglo literal ───────────────────────────────────
        Expresion::Arreglo(elementos) => {
            let mut values = Vec::new();
            for elem in elementos {
                values.push(evaluar_expresion(elem, ambito, store, declaraciones)?);
            }
            let json_arr: Vec<serde_json::Value> =
                values.iter().map(|v| v.to_json_value()).collect();
            let json_str = serde_json::to_string(&json_arr)
                .map_err(|e| format!("Error serializando array: {}", e))?;
            Ok(ValorGUI::Texto(json_str))
        }

        // ── Mapa literal ──────────────────────────────────────
        Expresion::Mapa(pares) => {
            let mut map = serde_json::Map::new();
            for (k, v) in pares {
                let key_val = evaluar_expresion(k, ambito, store, declaraciones)?;
                let val = evaluar_expresion(v, ambito, store, declaraciones)?;
                map.insert(key_val.to_display(), val.to_json_value());
            }
            let json_str = serde_json::to_string(&serde_json::Value::Object(map))
                .map_err(|e| format!("Error serializando mapa: {}", e))?;
            Ok(ValorGUI::Texto(json_str))
        }

        // ── Match/Coincidir ───────────────────────────────────
        Expresion::Coincidir {
            expr: inner,
            brazos,
        } => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
            for brazo in brazos {
                if coincidir_patron(&val, &brazo.patron, ambito) {
                    return evaluar_bloque(&brazo.cuerpo, ambito, store, declaraciones);
                }
            }
            Err("Ningún brazo de match coincidió".to_string())
        }

        // ── Index (arr[0]) ────────────────────────────────────
        Expresion::Index { objeto, indice } => {
            let obj = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            let idx = evaluar_expresion(indice, ambito, store, declaraciones)?;
            let idx_num = match idx {
                ValorGUI::Entero(n) => n as usize,
                _ => {
                    return Err("Índice debe ser un entero".to_string());
                }
            };
            if let Ok(serde_json::Value::Array(arr)) =
                serde_json::from_str::<serde_json::Value>(&obj.to_display())
            {
                if idx_num < arr.len() {
                    Ok(ValorGUI::from_serde(&arr[idx_num]))
                } else {
                    Err(format!(
                        "Índice {} fuera de rango (len={})",
                        idx_num,
                        arr.len()
                    ))
                }
            } else {
                Err("No se puede indexar un valor que no es un array".to_string())
            }
        }

        // ── Closure ───────────────────────────────────────────
        Expresion::Closure { .. } => Err("Closures no soportados en runtime GUI".to_string()),

        // ── Grupo (expresión agrupada) ────────────────────────
        Expresion::Grupo(inner) => evaluar_expresion(inner, ambito, store, declaraciones),

        // ── Hilo ligero ───────────────────────────────────────
        Expresion::Hilo { .. } => Err("Hilos ligeros no soportados en runtime GUI".to_string()),

        // ── Canal ─────────────────────────────────────────────
        Expresion::CanalNuevo => Err("Canales no soportados en runtime GUI".to_string()),

        // ── Try (propagación de error) ────────────────────────
        Expresion::Try(inner) => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
            if matches!(val, ValorGUI::Nulo) {
                Err("Error propagado desde expresión?".to_string())
            } else {
                Ok(val)
            }
        }

        // ── Seleccionar ───────────────────────────────────────
        Expresion::Seleccionar { .. } => Err("Seleccionar no soportado en runtime GUI".to_string()),

        // ── Asignación como expresión ─────────────────────────
        Expresion::Asignacion { variable, valor } => {
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            ambito.asignar(variable.clone(), val.clone());
            store.set(variable, val.to_json_value());
            Ok(val)
        }

        // ── Asignación a campo como expresión ────────────────
        Expresion::AsignacionCampo {
            objeto,
            campo,
            valor,
        } => {
            let obj = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            let key = format!("{}_{}", obj.to_display(), campo);
            store.set(&key, val.to_json_value());
            Ok(val)
        }

        // ── ArraySet (arr[i] = valor como expresión) ─────────
        Expresion::ArraySet { array, valor } => {
            let val = evaluar_expresion(valor, ambito, store, declaraciones)?;
            let _arr_val = evaluar_expresion(array, ambito, store, declaraciones)?;
            Ok(val)
        }

        // ── Resultado: Ok / Error ─────────────────────────────
        Expresion::Ok(inner) => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
            Ok(val)
        }
        Expresion::Error(inner) => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
            Err(format!("Error: {}", val.to_display()))
        }

        // ── Opción: Algo ──────────────────────────────────────
        Expresion::Algo(inner) => evaluar_expresion(inner, ambito, store, declaraciones),

        // ── Design by Contract ────────────────────────────────
        Expresion::Resultado => Ok(ValorGUI::Nulo),
        Expresion::Anterior(inner) => evaluar_expresion(inner, ambito, store, declaraciones),
    }
}

// ─── Evaluación binaria ─────────────────────────────────────────

fn evaluar_binaria(izq: ValorGUI, operador: &Operador, der: ValorGUI) -> Result<ValorGUI, String> {
    match operador {
        Operador::Suma => Ok(izq + der),
        Operador::Resta => Ok(izq - der),
        Operador::Multiplicacion => Ok(izq * der),
        Operador::Division => Ok(izq / der),
        Operador::Modulo => match (izq, der) {
            (ValorGUI::Entero(a), ValorGUI::Entero(b)) => {
                if b == 0 {
                    Err("División por cero en módulo".to_string())
                } else {
                    Ok(ValorGUI::Entero(a % b))
                }
            }
            _ => Err("Módulo sólo soportado para enteros".to_string()),
        },
        Operador::IgualIgual => Ok(ValorGUI::Booleano(izq == der)),
        Operador::Diferente => Ok(ValorGUI::Booleano(izq != der)),
        Operador::Menor | Operador::MenorIgual | Operador::Mayor | Operador::MayorIgual => {
            Ok(ValorGUI::Booleano(izq.compare(operador, &der)))
        }
        Operador::Y => Ok(ValorGUI::Booleano(izq.es_verdadero() && der.es_verdadero())),
        Operador::O => Ok(ValorGUI::Booleano(izq.es_verdadero() || der.es_verdadero())),
    }
}

// ─── Pattern matching ───────────────────────────────────────────

/// Verifica si un valor coincide con un patrón
fn coincidir_patron(valor: &ValorGUI, patron: &Patron, ambito: &mut Ambito) -> bool {
    match patron {
        Patron::Ignorar => true,
        Patron::Variable(nombre) => {
            ambito.asignar(nombre.clone(), valor.clone());
            true
        }
        Patron::Literal(expr) => match expr {
            Expresion::LiteralNumero(n) => {
                matches!(valor, ValorGUI::Entero(v) if v == n)
            }
            Expresion::LiteralDecimal(f) => {
                matches!(valor, ValorGUI::Decimal(v) if (v - f).abs() < f64::EPSILON)
            }
            Expresion::LiteralTexto(s) => {
                matches!(valor, ValorGUI::Texto(v) if v == s)
            }
            Expresion::LiteralBooleano(b) => {
                matches!(valor, ValorGUI::Booleano(v) if v == b)
            }
            Expresion::LiteralNulo => matches!(valor, ValorGUI::Nulo),
            _ => false,
        },
        Patron::Constructor(nombre, subpatrones) => match (nombre.as_str(), valor) {
            ("Ok", _) => {
                subpatrones.is_empty() || {
                    subpatrones.len() == 1 && coincidir_patron(valor, &subpatrones[0], ambito)
                }
            }
            ("Error", _) => subpatrones.is_empty(),
            ("Algo", _) => {
                subpatrones.is_empty() || {
                    subpatrones.len() == 1 && coincidir_patron(valor, &subpatrones[0], ambito)
                }
            }
            ("Ninguno", _) => matches!(valor, ValorGUI::Nulo),
            _ => false,
        },
    }
}

// ─── Funciones públicas para integración ───────────────────────

/// Inicializa el estado evaluando la función `main`
pub fn inicializar_estado(declaraciones: &[Declaracion], store: &mut VariableStore) {
    for decl in declaraciones {
        if let Declaracion::Funcion { nombre, cuerpo, .. } = decl {
            if nombre == "main" {
                let mut ambito = Ambito::new();
                for d in cuerpo {
                    if let Declaracion::Variable { nombre, valor, .. } = d {
                        let val = if let Some(expr) = valor {
                            match expr {
                                Expresion::LiteralNumero(n) => ValorGUI::Entero(*n),
                                Expresion::LiteralDecimal(f) => ValorGUI::Decimal(*f),
                                Expresion::LiteralTexto(s) => ValorGUI::Texto(s.clone()),
                                Expresion::LiteralBooleano(b) => ValorGUI::Booleano(*b),
                                Expresion::LiteralNulo => ValorGUI::Nulo,
                                Expresion::LiteralExacto(coeff, scale) => {
                                    ValorGUI::Decimal(*coeff as f64 / (10f64).powi(*scale as i32))
                                }
                                _ => {
                                    match evaluar_expresion(expr, &mut ambito, store, declaraciones)
                                    {
                                        Ok(v) => v,
                                        Err(_) => ValorGUI::Nulo,
                                    }
                                }
                            }
                        } else {
                            ValorGUI::Nulo
                        };
                        ambito.asignar(nombre.clone(), val.clone());
                        store.set(nombre, val.to_json_value());
                    }
                }
                return;
            }
        }
    }
}
