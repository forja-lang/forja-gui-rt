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
//   - Soporta llamadas a funciones definidas en el AST y funciones nativas

use crate::gui_nativa::ValorGUI;
use crate::signals::VariableStore;
use forja::ast::*;
use rusqlite::Connection;
use std::collections::HashMap;
use std::ops::{Add, Div, Mul, Sub};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

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
            ValorGUI::Mapa(m) => !m.is_empty(),
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
            ValorGUI::Mapa(m) => {
                let mut map = serde_json::Map::new();
                for (k, v) in m {
                    map.insert(k.clone(), v.to_json_value());
                }
                serde_json::Value::Object(map)
            }
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
            serde_json::Value::Array(_arr) => {
                // Representar arrays como JSON textual para compatibilidad
                // con el resto del runtime (que maneja arrays como strings)
                let json_str = serde_json::to_string(val)
                    .unwrap_or_else(|_| "[]".to_string());
                ValorGUI::Texto(json_str)
            }
            serde_json::Value::Object(_map) => {
                // Representar mapas como JSON textual para compatibilidad
                // con acceso por clave (mapa["clave"])
                let json_str = serde_json::to_string(val)
                    .unwrap_or_else(|_| "{}".to_string());
                ValorGUI::Texto(json_str)
            }
            serde_json::Value::Null => ValorGUI::Nulo,
        }
    }

    /// 5.3: Conversión lazy desde serde_json::Value.
    /// Para objetos, preserva la estructura como Mapa en vez de serializar a JSON string.
    /// Para arrays pequeños (≤10 elems), también preserva como Mapa con claves numéricas.
    pub fn from_serde_lazy(val: &serde_json::Value) -> Self {
        match val {
            serde_json::Value::Object(map) => {
                let inner: HashMap<String, ValorGUI> = map.iter()
                    .map(|(k, v)| (k.clone(), ValorGUI::from_serde(v)))
                    .collect();
                ValorGUI::Mapa(inner)
            }
            serde_json::Value::Array(arr) => {
                if arr.len() <= 10 {
                    let inner: Vec<ValorGUI> = arr.iter()
                        .map(|v| ValorGUI::from_serde(v))
                        .collect();
                    // Usar Mapa con claves numéricas como workaround para arrays pequeños
                    let map: HashMap<String, ValorGUI> = inner.into_iter()
                        .enumerate()
                        .map(|(i, v)| (i.to_string(), v))
                        .collect();
                    ValorGUI::Mapa(map)
                } else {
                    // Arrays grandes: serializar a JSON string (comportamiento original)
                    let json_str = val.to_string();
                    ValorGUI::Texto(json_str)
                }
            }
            _ => ValorGUI::from_serde(val),
        }
    }

    /// Comparación ordenada para <, <=, >, >=
    pub fn compare(&self, op: &Operador, other: &ValorGUI) -> bool {
        let a = ValorGUI::to_f64(self);
        let b = ValorGUI::to_f64(other);
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
            ValorGUI::Mapa(m) => {
                let parts: Vec<String> = m.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_display()))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
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

    /// Obtiene todas las variables del ámbito para su copia global
    pub fn obtener_todas(&self) -> &HashMap<String, ValorGUI> {
        &self.variables
    }
}

// ═════════════════════════════════════════════════════════════════════════
// FUNCIONES NATIVAS (built-ins del compilador Forja)
// ═════════════════════════════════════════════════════════════════════════

/// Registro de funciones nativas: nombre → implementación
/// El segundo argumento `&mut VariableStore` permite a las funciones nativas
/// leer/escribir variables del store (útil para SQLite, callbacks, etc.).
type NativeFn = fn(&[ValorGUI], &mut VariableStore) -> Result<ValorGUI, String>;

fn obtener_nativas() -> HashMap<&'static str, NativeFn> {
    let mut m: HashMap<&'static str, NativeFn> = HashMap::new();
    m.insert("timestamp", nativa_timestamp);
    m.insert("fecha_desde_timestamp", nativa_fecha_desde_timestamp);
    m.insert("_fecha_desde_timestamp", nativa_fecha_desde_timestamp);
    m.insert("dividir", nativa_dividir);
    m.insert("a_numero", nativa_a_numero);
    m.insert("longitud", nativa_longitud);
    m.insert("tipo", nativa_tipo);
    m.insert("a_texto", nativa_a_texto);
    m.insert("empujar", nativa_empujar);
    // SQLite
    m.insert("_sqlite_abrir", sqlite_abrir);
    m.insert("_sqlite_cerrar", sqlite_cerrar);
    m.insert("_sqlite_ejecutar", sqlite_ejecutar);
    m.insert("_sqlite_consultar", sqlite_consultar);
    m.insert("_sqlite_ejecutar_params", sqlite_ejecutar_params);
    m.insert("_sqlite_consultar_params", sqlite_consultar_params);
    m.insert("_sqlite_ultimo_id", sqlite_ultimo_id);
    m.insert("_sqlite_tablas", sqlite_tablas);
    m.insert("_sqlite_columnas", sqlite_columnas);
    // Archivo
    m.insert("_archivo_leer", nativa_archivo_leer);
    m.insert("_archivo_escribir", nativa_archivo_escribir);
    // JSON
    m.insert("_json_stringificar", nativa_json_stringificar);
    m
}

/// `timestamp()` — tiempo actual en milisegundos desde Unix epoch
fn nativa_timestamp(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if !args.is_empty() {
        return Err("timestamp() no requiere argumentos".to_string());
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Error obteniendo timestamp: {}", e))?
        .as_millis() as i64;
    Ok(ValorGUI::Entero(ts))
}

/// `fecha_desde_timestamp(ts)` — formatea timestamp (ms) a "YYYY|MM|DD|HH|mm|ss|n_dia|n_mes"
fn nativa_fecha_desde_timestamp(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let ts = match args.get(0) {
        Some(ValorGUI::Entero(n)) => *n,
        _ => return Err("fecha_desde_timestamp(ts) requiere un entero".to_string()),
    };

    // Si el timestamp está en milisegundos (> 1e12), convertir a segundos
    let ts_secs = if ts > 100_000_000_0000 { ts / 1000 } else { ts };

    // Calcular componentes de fecha
    let dias = ts_secs.div_euclid(86400);
    let segundos_del_dia = ts_secs.rem_euclid(86400);
    let hora = (segundos_del_dia / 3600) as u32;
    let minuto = ((segundos_del_dia % 3600) / 60) as u32;
    let segundo = (segundos_del_dia % 60) as u32;

    let (anio, mes, dia) = civil_from_days(dias);
    let dia_semana = day_of_week(anio, mes, dia);
    let nombre_dia = NOMBRES_DIA[dia_semana as usize];
    let nombre_mes = NOMBRES_MES[(mes - 1) as usize];

    let salida = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        anio, mes, dia, hora, minuto, segundo, nombre_dia, nombre_mes
    );
    Ok(ValorGUI::Texto(salida))
}

/// `dividir(texto, separador)` — divide un string y retorna JSON array
fn nativa_dividir(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 2 {
        return Err("dividir(texto, separador) requiere 2 argumentos".to_string());
    }
    let texto = match &args[0] {
        ValorGUI::Texto(s) => s.clone(),
        ValorGUI::Entero(n) => n.to_string(),
        ValorGUI::Decimal(f) => f.to_string(),
        ValorGUI::Booleano(b) => b.to_string(),
        ValorGUI::Mapa(m) => format!("{:?}", m),
        ValorGUI::Nulo => String::new(),
    };
    let separador = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("dividir: separador debe ser texto".to_string()),
    };

    let partes: Vec<serde_json::Value> = if separador.is_empty() {
        texto.chars()
            .map(|c| serde_json::Value::String(c.to_string()))
            .collect()
    } else {
        texto.split(&separador)
            .map(|s| serde_json::Value::String(s.to_string()))
            .collect()
    };

    let json_arr = serde_json::Value::Array(partes);
    let json_str = serde_json::to_string(&json_arr)
        .map_err(|e| format!("Error serializando array: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

/// `a_numero(texto)` — convierte texto a entero (i64)
fn nativa_a_numero(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let val = args.get(0).ok_or("a_numero(valor) requiere un argumento".to_string())?;
    match val {
        ValorGUI::Entero(n) => Ok(ValorGUI::Entero(*n)),
        ValorGUI::Decimal(f) => Ok(ValorGUI::Entero(*f as i64)),
        ValorGUI::Booleano(b) => Ok(ValorGUI::Entero(if *b { 1 } else { 0 })),
        ValorGUI::Texto(s) => {
            let trimmed = s.trim();
            // Si el texto es JSON (objeto o array), no se puede convertir a número
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                return Ok(ValorGUI::Entero(0));
            }
            match trimmed.parse::<i64>() {
                Ok(n) => Ok(ValorGUI::Entero(n)),
                Err(_) => match trimmed.parse::<f64>() {
                    Ok(f) => Ok(ValorGUI::Entero(f as i64)),
                    Err(_) => {
                       Ok(ValorGUI::Entero(0))
                    }
                },
            }
        }
        ValorGUI::Mapa(_) => {
           Ok(ValorGUI::Entero(0))
        }
        ValorGUI::Nulo => {
           Ok(ValorGUI::Entero(0))
        }
    }
}

/// `longitud(valor)` — longitud de array (JSON) o texto
fn nativa_longitud(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let val = args.get(0).ok_or("longitud(valor) requiere 1 argumento".to_string())?;
    match val {
        ValorGUI::Texto(s) => {
            // Intentar parsear como JSON array
            if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(s) {
                Ok(ValorGUI::Entero(arr.len() as i64))
            } else {
                // Contar caracteres (incluyendo emojis/unicode)
                Ok(ValorGUI::Entero(s.chars().count() as i64))
            }
        }
        ValorGUI::Entero(n) => Ok(ValorGUI::Entero(*n)),
        ValorGUI::Decimal(_) => Ok(ValorGUI::Entero(1)),
        ValorGUI::Booleano(_) => Ok(ValorGUI::Entero(1)),
        ValorGUI::Mapa(m) => Ok(ValorGUI::Entero(m.len() as i64)),
        ValorGUI::Nulo => Ok(ValorGUI::Entero(0)),
    }
}

/// `tipo(valor)` — retorna el nombre del tipo como texto
fn nativa_tipo(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let val = args.get(0).ok_or("tipo(valor) requiere 1 argumento".to_string())?;
    let nombre = match val {
        ValorGUI::Entero(_) => "entero",
        ValorGUI::Decimal(_) => "decimal",
        ValorGUI::Texto(s) => {
            // El runtime Forja representa arrays como JSON textual.
            // Detectar JSON arrays (ej: "[1,2,3]") y retornar "arreglo".
            let trimmed = s.trim_start();
            if trimmed.starts_with('[') {
                "arreglo"
            } else if trimmed.starts_with('{') {
                "mapa"
            } else {
                "texto"
            }
        }
        ValorGUI::Booleano(_) => "booleano",
        ValorGUI::Mapa(_) => "mapa",
        ValorGUI::Nulo => "nulo",
    };
    Ok(ValorGUI::Texto(nombre.to_string()))
}

/// `_archivo_leer(ruta)` — lee texto de un archivo en disco
fn nativa_archivo_leer(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let ruta = match args.get(0) {
        Some(ValorGUI::Texto(t)) => t.clone(),
        _ => return Err("_archivo_leer(ruta): ruta debe ser texto".to_string()),
    };
    let contenido = std::fs::read_to_string(&ruta)
        .map_err(|e| format!("Error al leer archivo '{}': {}", ruta, e))?;
    Ok(ValorGUI::Texto(contenido))
}

/// `_archivo_escribir(ruta, contenido)` — escribe texto a un archivo en disco
fn nativa_archivo_escribir(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let ruta = match args.get(0) {
        Some(ValorGUI::Texto(t)) => t.clone(),
        _ => return Err("_archivo_escribir(ruta, contenido): ruta debe ser texto".to_string()),
    };
    let contenido = match args.get(1) {
        Some(ValorGUI::Texto(t)) => t.clone(),
        _ => return Err("_archivo_escribir(ruta, contenido): contenido debe ser texto".to_string()),
    };
    std::fs::write(&ruta, &contenido).map_err(|e| format!("Error al escribir archivo '{}': {}", ruta, e))?;
    Ok(ValorGUI::Entero(contenido.len() as i64))
}

/// `_json_stringificar(valor)` — convierte cualquier valor a JSON string
fn nativa_json_stringificar(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let val = args.get(0).ok_or("_json_stringificar(valor) requiere 1 argumento".to_string())?;
    let json_value = val.to_json_value();
    let json_str = serde_json::to_string(&json_value)
        .map_err(|e| format!("Error al stringificar a JSON: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

/// `a_texto(valor)` — convierte cualquier valor a texto
fn nativa_a_texto(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let val = args.get(0).ok_or("a_texto(valor) requiere 1 argumento".to_string())?;
    Ok(ValorGUI::Texto(val.to_display()))
}

/// `empujar(array, elemento)` — agrega un elemento al final de un array JSON
/// El array se representa como texto conteniendo un JSON array: "[1,2,3]"
/// Retorna el nuevo array serializado como texto.
fn nativa_empujar(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 2 {
        return Err("empujar(array, elemento) requiere 2 argumentos".to_string());
    }

    let json_str = match &args[0] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("empujar: el primer argumento debe ser un array (texto)".to_string()),
    };

    let mut arr: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("empujar: error parseando array: {}", e))?;

    let arr_ref = match &mut arr {
        serde_json::Value::Array(a) => a,
        _ => return Err("empujar: el primer argumento no es un array JSON".to_string()),
    };

    let valor_json = args[1].to_json_value();
    arr_ref.push(valor_json);

    let result = serde_json::to_string(&arr)
        .map_err(|e| format!("empujar: error serializando array: {}", e))?;
   Ok(ValorGUI::Texto(result))
}

// ═════════════════════════════════════════════════════════════════════════
// POOL DE CONEXIONES SQLITE
// ═════════════════════════════════════════════════════════════════════════

/// Heap global de conexiones SQLite (envueltas en Arc<Mutex<>>).
/// Cada conexión se almacena como `Option` para permitir cerrado lógico.
static SQLITE_HEAP: Mutex<Vec<Option<Arc<Mutex<Connection>>>>> = Mutex::new(Vec::new());

/// Extrae un índice entero de un ValorGUI, aceptando Entero y Decimal.
fn extraer_indice(valor: &ValorGUI, nombre_fn: &str) -> Result<usize, String> {
    match valor {
        ValorGUI::Entero(n) => Ok(*n as usize),
        ValorGUI::Decimal(f) => Ok(*f as usize),
        _ => Err(format!(
            "{}: primer argumento debe ser entero (índice), se recibió: {}",
            nombre_fn,
            valor.to_display()
        )),
    }
}

/// Obtiene una conexión del heap por índice (retorna Arc clonado).
fn sqlite_obtener_conn(idx: usize) -> Result<Arc<Mutex<Connection>>, String> {
    let heap = SQLITE_HEAP.lock().map_err(|e| format!("sqlite_error_interno: {}", e))?;
    if idx >= heap.len() {
        return Err("sqlite_error: índice de conexión inválido".to_string());
    }
    match &heap[idx] {
        Some(conn) => Ok(Arc::clone(conn)),
        None => Err("sqlite_error: conexión cerrada o inexistente".to_string()),
    }
}

/// Convierte un Value de SQLite a serde_json::Value para serialización.
fn sqlite_valor_a_json(val: rusqlite::types::Value) -> serde_json::Value {
    match val {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(n) => serde_json::Value::Number(n.into()),
        rusqlite::types::Value::Real(f) => {
            serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        rusqlite::types::Value::Text(s) => serde_json::Value::String(s),
        rusqlite::types::Value::Blob(b) => {
            serde_json::Value::String(String::from_utf8_lossy(&b).to_string())
        }
    }
}

/// Parsea un ValorGUI::Texto como JSON array para extraer parámetros SQL.
fn args_a_params(val: &ValorGUI) -> Result<Vec<Box<dyn rusqlite::types::ToSql>>, String> {
    let json_str = match val {
        ValorGUI::Texto(s) => s.clone(),
        ValorGUI::Entero(n) => return Ok(vec![Box::new(*n)]),
        ValorGUI::Decimal(f) => return Ok(vec![Box::new(*f)]),
        ValorGUI::Booleano(b) => return Ok(vec![Box::new(*b as i64)]),
        ValorGUI::Mapa(m) => {
           // Intentar extraer un arreglo de valores del mapa
            // Si el mapa tiene una clave "0", "1", etc., tratarlo como arreglo
            let mut arr = Vec::new();
            let mut i = 0;
            loop {
                let key = i.to_string();
                if let Some(v) = m.get(&key) {
                    arr.push(v.to_json_value());
                    i += 1;
                } else {
                    break;
                }
            }
            if !arr.is_empty() {
                serde_json::to_string(&serde_json::Value::Array(arr))
                    .map_err(|e| format!("sqlite_error: no se pudo serializar mapa como arreglo: {}", e))?
            } else {
                // Fallback: serializar como objeto
                let map: serde_json::Map<String, serde_json::Value> = m.iter()
                    .map(|(k, v)| (k.clone(), v.to_json_value()))
                    .collect();
                serde_json::to_string(&serde_json::Value::Object(map))
                    .map_err(|e| format!("sqlite_error: no se pudo serializar mapa: {}", e))?
            }
        }
        ValorGUI::Nulo => {
           return Ok(vec![Box::new(rusqlite::types::Null)]);
        }
    };
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("sqlite_error: no se pudieron parsear los parámetros: {}", e))?;
    let arr = match parsed {
        serde_json::Value::Array(a) => a,
        _ => return Err("sqlite_error: los parámetros deben ser un arreglo JSON".to_string()),
    };
    if arr.is_empty() {
    }
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(arr.len());
    for v in &arr {
        match v {
            serde_json::Value::String(s) => params.push(Box::new(s.clone())),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    params.push(Box::new(i));
                } else if let Some(f) = n.as_f64() {
                    params.push(Box::new(f));
                }
            }
            serde_json::Value::Bool(b) => params.push(Box::new(*b as i64)),
            serde_json::Value::Null => params.push(Box::new(rusqlite::types::Null)),
            _ => return Err("sqlite_error: tipo de parámetro no soportado".to_string()),
        }
    }
    Ok(params)
}

// ═════════════════════════════════════════════════════════════════════════
// FUNCIONES SQLITE NATIVAS
// ═════════════════════════════════════════════════════════════════════════

/// `_sqlite_abrir(ruta)` → índice de conexión (entero)
fn sqlite_abrir(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let ruta = match args.get(0) {
        Some(ValorGUI::Texto(s)) => s.clone(),
        _ => return Err("_sqlite_abrir(ruta) requiere 1 argumento: ruta (texto)".to_string()),
    };
    let conn = Connection::open(&ruta)
        .map_err(|e| format!("sqlite_error_apertura: {}", e))?;
    let mut heap = SQLITE_HEAP.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    let idx = heap.len() as i64;
    heap.push(Some(Arc::new(Mutex::new(conn))));
    Ok(ValorGUI::Entero(idx))
}

/// `_sqlite_cerrar(indice_conexion)` → 0 éxito, -1 error
fn sqlite_cerrar(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let idx = match args.get(0) {
        Some(v) => extraer_indice(v, "_sqlite_cerrar")?,
        None => return Err("_sqlite_cerrar(indice) requiere 1 argumento: entero".to_string()),
    };
    let mut heap = SQLITE_HEAP.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    if idx < heap.len() {
        heap[idx] = None;
        Ok(ValorGUI::Entero(0))
    } else {
        Err("sqlite_error: índice de conexión inválido".to_string())
    }
}

/// `_sqlite_ejecutar(indice_conexion, sql)` → filas_afectadas
fn sqlite_ejecutar(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 2 {
        return Err("_sqlite_ejecutar(indice, sql) requiere 2 argumentos".to_string());
    }
    let idx = extraer_indice(&args[0], "_sqlite_ejecutar")?;
    let sql = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("_sqlite_ejecutar: segundo argumento debe ser texto (SQL)".to_string()),
    };
    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    match conn.execute(&sql, []) {
        Ok(filas) => Ok(ValorGUI::Entero(filas as i64)),
        Err(e) => Err(format!("sqlite_error_ejecucion: {}", e)),
    }
}

/// `_sqlite_consultar(indice_conexion, sql)` → JSON array de mapas (como texto)
fn sqlite_consultar(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 2 {
        return Err("_sqlite_consultar(indice, sql) requiere 2 argumentos".to_string());
    }
    let idx = extraer_indice(&args[0], "_sqlite_consultar")?;
    let sql = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("_sqlite_consultar: segundo argumento debe ser texto (SQL)".to_string()),
    };
    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("sqlite_error_consulta: {}", e))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    // Extraer datos sin mantener borrow del closure
    let rows_data: Vec<Vec<(String, rusqlite::types::Value)>> = stmt.query_map([], |row| {
        let mut row_data = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let name = col_names[i].clone();
            let val = row.get::<_, rusqlite::types::Value>(i)
                .unwrap_or(rusqlite::types::Value::Null);
            row_data.push((name, val));
        }
        Ok(row_data)
    }).map_err(|e| format!("sqlite_error_consulta: {}", e))?
    .filter_map(|r| r.ok())
    .collect();

    // Convertir a JSON
    let json_rows: Vec<serde_json::Value> = rows_data.into_iter().map(|row| {
        let mut map = serde_json::Map::new();
        for (name, val) in row {
            map.insert(name, sqlite_valor_a_json(val));
        }
        serde_json::Value::Object(map)
    }).collect();

    let json_str = serde_json::to_string(&serde_json::Value::Array(json_rows))
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

/// `_sqlite_ultimo_id(indice_conexion)` → último rowid insertado
fn sqlite_ultimo_id(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let idx = match args.get(0) {
        Some(v) => extraer_indice(v, "_sqlite_ultimo_id")?,
        None => return Err("_sqlite_ultimo_id(indice) requiere 1 argumento: entero".to_string()),
    };
    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    Ok(ValorGUI::Entero(conn.last_insert_rowid()))
}

/// `_sqlite_ejecutar_params(indice, sql, arreglo_valores)` → filas_afectadas
fn sqlite_ejecutar_params(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 3 {
        return Err("_sqlite_ejecutar_params(indice, sql, valores) requiere 3 argumentos".to_string());
    }
    let idx = extraer_indice(&args[0], "_sqlite_ejecutar_params")?;
    let sql = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("_sqlite_ejecutar_params: segundo argumento debe ser texto (SQL)".to_string()),
    };
    let params = args_a_params(&args[2])?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    match conn.execute(&sql, params_refs.as_slice()) {
        Ok(filas) => Ok(ValorGUI::Entero(filas as i64)),
        Err(e) => Err(format!("sqlite_error_ejecucion: {}", e)),
    }
}

/// `_sqlite_consultar_params(indice, sql, arreglo_valores)` → JSON array de mapas
fn sqlite_consultar_params(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 3 {
        return Err("_sqlite_consultar_params(indice, sql, valores) requiere 3 argumentos".to_string());
    }
    let idx = extraer_indice(&args[0], "_sqlite_consultar_params")?;
    let sql = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("_sqlite_consultar_params: segundo argumento debe ser texto (SQL)".to_string()),
    };
    let params = args_a_params(&args[2])?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;

    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("sqlite_error_consulta: {}", e))?;
    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let rows_data: Vec<Vec<(String, rusqlite::types::Value)>> = stmt.query_map(params_refs.as_slice(), |row| {
        let mut row_data = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let name = col_names[i].clone();
            let val = row.get::<_, rusqlite::types::Value>(i)
                .unwrap_or(rusqlite::types::Value::Null);
            row_data.push((name, val));
        }
        Ok(row_data)
    }).map_err(|e| format!("sqlite_error_consulta: {}", e))?
    .filter_map(|r| r.ok())
    .collect();

    let json_rows: Vec<serde_json::Value> = rows_data.into_iter().map(|row| {
        let mut map = serde_json::Map::new();
        for (name, val) in row {
            map.insert(name, sqlite_valor_a_json(val));
        }
        serde_json::Value::Object(map)
    }).collect();

    let json_str = serde_json::to_string(&serde_json::Value::Array(json_rows))
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

/// `_sqlite_tablas(indice_conexion)` → JSON array de nombres de tablas (como texto)
fn sqlite_tablas(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    let idx = match args.get(0) {
        Some(v) => extraer_indice(v, "_sqlite_tablas")?,
        None => return Err("_sqlite_tablas(indice) requiere 1 argumento: entero".to_string()),
    };
    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;

    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).map_err(|e| format!("sqlite_error_consulta: {}", e))?;

    let nombres: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("sqlite_error_consulta: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let json_arr: Vec<serde_json::Value> = nombres.into_iter()
        .map(serde_json::Value::String)
        .collect();
    let json_str = serde_json::to_string(&serde_json::Value::Array(json_arr))
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

/// `_sqlite_columnas(indice_conexion, tabla)` → JSON array de mapas con info de columnas
fn sqlite_columnas(args: &[ValorGUI], _store: &mut VariableStore) -> Result<ValorGUI, String> {
    if args.len() < 2 {
        return Err("_sqlite_columnas(indice, tabla) requiere 2 argumentos".to_string());
    }
    let idx = extraer_indice(&args[0], "_sqlite_columnas")?;
    let tabla = match &args[1] {
        ValorGUI::Texto(s) => s.clone(),
        _ => return Err("_sqlite_columnas: segundo argumento debe ser texto (tabla)".to_string()),
    };
    let conn_arc = sqlite_obtener_conn(idx)?;
    let conn = conn_arc.lock()
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;

    let sql = format!("PRAGMA table_info('{}')", tabla.replace('\'', "''"));
    let mut stmt = conn.prepare(&sql)
        .map_err(|e| format!("sqlite_error_consulta: {}", e))?;

    // Extraer datos crudos
    let cols: Vec<(i64, String, String, bool, Option<String>, i64)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, bool>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
        ))
    }).map_err(|e| format!("sqlite_error_consulta: {}", e))?
    .filter_map(|r| r.ok())
    .collect();

    let json_rows: Vec<serde_json::Value> = cols.into_iter().map(|(cid, nombre, tipo, not_null, default, pk)| {
        let mut map = serde_json::Map::new();
        map.insert("cid".to_string(), serde_json::Value::Number(cid.into()));
        map.insert("nombre".to_string(), serde_json::Value::String(nombre));
        map.insert("tipo".to_string(), serde_json::Value::String(tipo));
        map.insert("not_null".to_string(), serde_json::Value::Bool(not_null));
        map.insert("default".to_string(), match default {
            Some(d) => serde_json::Value::String(d),
            None => serde_json::Value::Null,
        });
        map.insert("pk".to_string(), serde_json::Value::Number(pk.into()));
        serde_json::Value::Object(map)
    }).collect();

    let json_str = serde_json::to_string(&serde_json::Value::Array(json_rows))
        .map_err(|e| format!("sqlite_error_interno: {}", e))?;
    Ok(ValorGUI::Texto(json_str))
}

// ═════════════════════════════════════════════════════════════════════════
// Algoritmos de fecha (copiados de native_registry.rs)
// ═════════════════════════════════════════════════════════════════════════

/// Algoritmo: días desde epoch (1970-01-01) hasta una fecha civil (año, mes, día)
/// Basado en el algoritmo de Howard Hinnant (calendario Gregoriano)
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m_shifted = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m_shifted as i64 + 2) / 5 + day as i64 - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Algoritmo inverso: timestamp → componentes de fecha civil
/// Retorna (year, month, day)
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468; // days since 0000-03-01
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month progress [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Día de la semana (0=Domingo, 1=Lunes, ..., 6=Sábado)
fn day_of_week(y: i64, m: u32, d: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    ((y + y / 4 - y / 100 + y / 400 + t[(m - 1) as usize] + d as i64) % 7) as u32
}

const NOMBRES_DIA: [&str; 7] = [
    "domingo", "lunes", "martes", "miércoles", "jueves", "viernes", "sábado",
];

const NOMBRES_MES: [&str; 12] = [
    "enero", "febrero", "marzo", "abril", "mayo", "junio",
    "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre",
];

// ─── Evaluación de funciones ─────────────────────────────────────

/// Evalúa una función Forja a partir del AST
pub fn ejecutar_funcion(
    nombre: &str,
    args: &[ValorGUI],
    declaraciones: &[Declaracion],
    store: &mut VariableStore,
) -> Result<ValorGUI, String> {
    // 1. Verificar si es una función nativa
    let nativas = obtener_nativas();
    if let Some(nativa_fn) = nativas.get(nombre) {
        return nativa_fn(args, store);
    }

    // 2. Buscar la función en las declaraciones del AST
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
            } else {
           }
        }

        // Fallback: cuando la función se invoca como callback desde widgets GUI
        if args.is_empty() && !parametros.is_empty() {
            if let Some(json_val) = store.get(nombre) {
               let primer_param = &parametros[0];
                if !ambito.contiene(&primer_param.nombre) {
                    ambito.asignar(
                        primer_param.nombre.clone(),
                        ValorGUI::from_serde(&json_val),
                    );
                }
            } else {
           }
        }

        // Evaluar cuerpo
        evaluar_bloque(cuerpo, &mut ambito, store, declaraciones)
    })
}

/// Busca una declaración de función por nombre.
/// Si no encuentra por nombre exacto, intenta sin el prefijo de módulo
/// (ej: "estadisticas_gastos_por_categoria" → busca "gastos_por_categoria").
fn buscar_funcion<'a>(
    nombre: &str,
    declaraciones: &'a [Declaracion],
) -> Result<&'a Declaracion, String> {
    // Primera pasada: búsqueda exacta
    for d in declaraciones {
        if let Declaracion::Funcion { nombre: ref n, .. } = d {
            if n == nombre {
                return Ok(d);
            }
        }
    }

    // Segunda pasada: si el nombre tiene formato modulo_funcion,
    // buscar solo la parte después del primer _
    if let Some(pos) = nombre.find('_') {
        let sin_prefijo = &nombre[pos + 1..];
        if !sin_prefijo.is_empty() {
            for d in declaraciones {
                if let Declaracion::Funcion { nombre: ref n, .. } = d {
                    if n == sin_prefijo {
                        return Ok(d);
                    }
                }
            }
        }
    }

    Err(format!("Función '{}' no encontrada", nombre))
}

/// Verifica si un nombre corresponde a una función ejecutable real
/// (nativa o declarada en el AST). Los widgets del layout GUI
/// (andamio, barra_superior, navegador, pantalla, columna, boton, ...)
/// no son funciones ejecutables del programa.
fn es_funcion_ejecutable(nombre: &str, declaraciones: &[Declaracion]) -> bool {
    let nativas = obtener_nativas();
    if nativas.contains_key(nombre) {
        return true;
    }
    buscar_funcion(nombre, declaraciones).is_ok()
}

// ─── Evaluación de bloques ──────────────────────────────────────

/// Evalúa un bloque de declaraciones.
/// Retorna Ok(ValorGUI) si encuentra un `retornar`, o el último valor del bloque.
fn evaluar_bloque(
    bloque: &[Declaracion],
    ambito: &mut Ambito,
    store: &mut VariableStore,
    declaraciones: &[Declaracion],
) -> Result<ValorGUI, String> {
    let mut ultimo = ValorGUI::Nulo;
    for declaracion in bloque {
        let result = evaluar_declaracion(declaracion, ambito, store, declaraciones)?;
        // Si la declaración fue un retornar, propagar el valor
        if es_retorno(declaracion) {
            return Ok(result);
        }
        ultimo = result;
    }
    // Retornar el último valor (permite que retornos dentro de Coincidir
    // y otras expresiones anidadas se propaguen al bloque contenedor)
    Ok(ultimo)
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
            // Actualizar el mapa/objeto en el ámbito local (clave → valor).
            // Los mapas y arrays se representan como Texto(JSON), así que se
            // parsean, se inserta la clave y se vuelven a serializar.
            if let Some(obj) = ambito.obtener(nombre).cloned() {
                match obj {
                    ValorGUI::Mapa(mut m) => {
                        m.insert(idx_val.to_display(), val.clone());
                        ambito.asignar(nombre.to_string(), ValorGUI::Mapa(m));
                    }
                    ValorGUI::Texto(s) => {
                        if let Ok(serde_json::Value::Object(mut map)) =
                            serde_json::from_str::<serde_json::Value>(&s)
                        {
                            map.insert(idx_val.to_display(), val.to_json_value());
                            if let Ok(nuevo) =
                                serde_json::to_string(&serde_json::Value::Object(map))
                            {
                                ambito.asignar(nombre.to_string(), ValorGUI::Texto(nuevo));
                            }
                        } else if let Ok(serde_json::Value::Array(mut arr)) =
                            serde_json::from_str::<serde_json::Value>(&s)
                        {
                            let idx = match &idx_val {
                                ValorGUI::Entero(n) => *n as usize,
                                _ => {
                                    if let Ok(n) = idx_val.to_display().parse::<usize>() {
                                        n
                                    } else {
                                        0
                                    }
                                }
                            };
                            if idx < arr.len() {
                                arr[idx] = val.to_json_value();
                            } else {
                                arr.push(val.to_json_value());
                            }
                            if let Ok(nuevo) =
                                serde_json::to_string(&serde_json::Value::Array(arr))
                            {
                                ambito.asignar(nombre.to_string(), ValorGUI::Texto(nuevo));
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Mantener compatibilidad con el store (claves con corchetes)
            let key = format!("{}[{}]", nombre, idx_val.to_display());
            store.set(&key, val.to_json_value());
            Ok(val)
        }

        Declaracion::LlamadaFuncion { nombre, argumentos } => {
            let mut args = Vec::new();
            for arg in argumentos {
                args.push(evaluar_expresion(arg, ambito, store, declaraciones)?);
            }
            // Manejar nombres con punto: "objeto.metodo" → método(objeto, args...)
            if let Some(dot_pos) = nombre.find('.') {
                let obj_name = &nombre[..dot_pos];
                let method = &nombre[dot_pos + 1..];
                let obj_val = if let Some(val) = ambito.obtener(obj_name) {
                    val.clone()
                } else if let Some(json_val) = store.get(obj_name) {
                    ValorGUI::from_serde(&json_val)
                } else {
                    return Err(format!("Variable '{}' no encontrada", obj_name));
                };
                let mut method_args = vec![obj_val];
                method_args.extend(args);
                let result = ejecutar_funcion(method, &method_args, declaraciones, store)?;
                // Métodos que mutan (ej: params.empujar): actualizar variable original
                if method == "empujar" {
                    ambito.asignar(obj_name.to_string(), result.clone());
                    store.set(obj_name, result.to_json_value());
                }
                Ok(result)
            } else {
                ejecutar_funcion(nombre, &args, declaraciones, store)
            }
        }

        Declaracion::Expresion(expr) => {
            // Retornar el resultado de la expresión directamente.
            // Esto permite que retornos dentro de Coincidir, Ok, etc.
            // se propaguen al bloque contenedor (evaluar_bloque).
            evaluar_expresion(expr, ambito, store, declaraciones)
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
        | Declaracion::ImportarExterna(_)
        | Declaracion::Enum { .. }
        | Declaracion::Rasgo { .. }
        | Declaracion::Implementacion { .. }
        | Declaracion::AccesoMiembro { .. }
        | Declaracion::Romper
        | Declaracion::Continuar => Ok(ValorGUI::Nulo),
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
            // Si el nombre contiene un punto (ej: "params.empujar"),
            // el compilador Forja genera LlamadaFuncion con nombre "objeto.metodo".
            // Resolver: buscar el objeto en el scope y anteponerlo como primer argumento,
            // luego llamar al método (la parte después del punto).
            if let Some(dot_pos) = nombre.find('.') {
                let obj_name = &nombre[..dot_pos];
                let method = &nombre[dot_pos + 1..];
                let obj_val = if let Some(val) = ambito.obtener(obj_name) {
                    val.clone()
                } else if let Some(json_val) = store.get(obj_name) {
                    ValorGUI::from_serde(&json_val)
                } else {
                    return Err(format!("Variable '{}' no encontrada", obj_name));
                };
                let mut method_args = vec![obj_val];
                method_args.extend(args);
                let result = ejecutar_funcion(method, &method_args, declaraciones, store)?;
                // Métodos que mutan (ej: params.empujar): actualizar variable original
                if method == "empujar" {
                    ambito.asignar(obj_name.to_string(), result.clone());
                    store.set(obj_name, result.to_json_value());
                }
                return Ok(result);
            } else {
                ejecutar_funcion(nombre, &args, declaraciones, store)
            }
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
        Expresion::Instanciacion { clase, argumentos } => {
            let mut mapa = serde_json::Map::new();
            mapa.insert("__clase".to_string(), serde_json::Value::String(clase.clone()));
            for (i, arg) in argumentos.iter().enumerate() {
                let val = evaluar_expresion(arg, ambito, store, declaraciones)?;
                mapa.insert(format!("arg_{}", i), val.to_json_value());
            }
            let json_str = serde_json::to_string(&serde_json::Value::Object(mapa))
                .map_err(|e| format!("Error serializando instancia: {}", e))?;
            Ok(ValorGUI::Texto(json_str))
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

        // ── Index (arr[0]) / Mapa["clave"] ───────────────────
        Expresion::Index { objeto, indice } => {
            let obj = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            let idx = evaluar_expresion(indice, ambito, store, declaraciones)?;
            // Acceso directo a mapa (ValorGUI::Mapa) por clave
            if let ValorGUI::Mapa(m) = &obj {
                let key = idx.to_display();
                return Ok(m.get(&key).cloned().unwrap_or(ValorGUI::Nulo));
            }
            let obj_str = obj.to_display();
            match idx {
                ValorGUI::Entero(n) => {
                    let idx_num = n as usize;
                    if let Ok(serde_json::Value::Array(arr)) =
                        serde_json::from_str::<serde_json::Value>(&obj_str)
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
                ValorGUI::Texto(key) => {
                    // Acceso a mapa por clave string (ej: filtros["tipo"])
                    if let Ok(serde_json::Value::Object(map)) =
                        serde_json::from_str::<serde_json::Value>(&obj_str)
                    {
                        match map.get(&key) {
                            Some(val) => Ok(ValorGUI::from_serde(val)),
                            None => Ok(ValorGUI::Nulo),
                        }
                    } else if let Ok(serde_json::Value::Array(arr)) =
                        serde_json::from_str::<serde_json::Value>(&obj_str)
                    {
                        // Si key es string numérico, intentar como índice
                        if let Ok(n) = key.parse::<usize>() {
                            if n < arr.len() {
                                Ok(ValorGUI::from_serde(&arr[n]))
                            } else {
                                Err(format!("Índice {} fuera de rango (len={})", n, arr.len()))
                            }
                        } else {
                            Err(format!("Clave '{}' no encontrada en array", key))
                        }
                    } else {
                        Err(format!("No se puede acceder por clave '{}' a un valor no-objeto", key))
                    }
                }
                _ => {
                    Err("Índice debe ser un entero o una clave de texto".to_string())
                }
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
            // array es un Index(objeto, indice): actualizar el mapa/array en el
            // ámbito (dashboard["saldo_total"] = s). Sin esto, el mapa queda
            // vacío y luego las lecturas devuelven nulo.
            if let Expresion::Index { objeto, indice } = array.as_ref() {
                let obj_nombre = match objeto.as_ref() {
                    Expresion::Identificador { nombre, .. } => Some(nombre.clone()),
                    _ => None,
                };
                if let Some(nombre) = obj_nombre {
                    let idx_val = evaluar_expresion(indice, ambito, store, declaraciones)?;
                    if let Some(obj) = ambito.obtener(&nombre).cloned() {
                        match obj {
                            ValorGUI::Mapa(mut m) => {
                                m.insert(idx_val.to_display(), val.clone());
                                ambito.asignar(nombre.clone(), ValorGUI::Mapa(m));
                            }
                            ValorGUI::Texto(s) => {
                                if let Ok(serde_json::Value::Object(mut map)) =
                                    serde_json::from_str::<serde_json::Value>(&s)
                                {
                                    map.insert(idx_val.to_display(), val.to_json_value());
                                    if let Ok(nuevo) =
                                        serde_json::to_string(&serde_json::Value::Object(map))
                                    {
                                        ambito.asignar(nombre.clone(), ValorGUI::Texto(nuevo));
                                    }
                                } else if let Ok(serde_json::Value::Array(mut arr)) =
                                    serde_json::from_str::<serde_json::Value>(&s)
                                {
                                    let idx = match &idx_val {
                                        ValorGUI::Entero(n) => *n as usize,
                                        _ => idx_val.to_display().parse::<usize>().unwrap_or(0),
                                    };
                                    if idx < arr.len() {
                                        arr[idx] = val.to_json_value();
                                    } else {
                                        arr.push(val.to_json_value());
                                    }
                                    if let Ok(nuevo) =
                                        serde_json::to_string(&serde_json::Value::Array(arr))
                                    {
                                        ambito.asignar(nombre.clone(), ValorGUI::Texto(nuevo));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    // Mantener compatibilidad con el store (claves con corchetes)
                    let key = format!("{}[{}]", nombre, idx_val.to_display());
                    store.set(&key, val.to_json_value());
                }
            }
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
        Expresion::Algo(inner) => {
            let val = evaluar_expresion(inner, ambito, store, declaraciones)?;
           Ok(val)
        }

        // ── Design by Contract ────────────────────────────────
        Expresion::Resultado => {
            // El compilador Forja genera Expresion::Resultado para referirse
            // al resultado de la expresión anterior (ej: la variable "resultado"
            // en "variable resultado = fn(); coincidir (resultado) { ... }").
            // Buscar en el ámbito local y luego en el store.
            if let Some(val) = ambito.obtener("resultado") {
                Ok(val.clone())
            } else if let Some(json_val) = store.get("resultado") {
                Ok(ValorGUI::from_serde(&json_val))
            } else {
                Ok(ValorGUI::Nulo)
            }
        }
        Expresion::Anterior(inner) => evaluar_expresion(inner, ambito, store, declaraciones),

        // ── Ternario ──────────────────────────────────────────
        Expresion::Ternario {
            condicion,
            si_verdadero,
            si_falso,
        } => {
            let cond = evaluar_expresion(condicion, ambito, store, declaraciones)?;
            let b = match &cond {
                ValorGUI::Booleano(b) => *b,
                ValorGUI::Entero(n) => *n != 0,
                _ => false,
            };
            if b {
                evaluar_expresion(si_verdadero, ambito, store, declaraciones)
            } else {
                evaluar_expresion(si_falso, ambito, store, declaraciones)
            }
        }

        // ── Llamada a método ──────────────────────────────────
        Expresion::LlamadaMetodo {
            objeto,
            metodo,
            argumentos,
        } => {
            let mut args = Vec::new();
            let obj_val = evaluar_expresion(objeto, ambito, store, declaraciones)?;
            args.push(obj_val.clone());
            for arg in argumentos {
                args.push(evaluar_expresion(arg, ambito, store, declaraciones)?);
            }
            let result = ejecutar_funcion(metodo, &args, declaraciones, store)?;
            // Métodos que mutan (ej: params.empujar(x)): actualizar variable original
            if metodo == "empujar" {
                if let Expresion::Identificador { nombre, .. } = objeto.as_ref() {
                    ambito.asignar(nombre.clone(), result.clone());
                    store.set(nombre, result.to_json_value());
                } else {
                }
            }
            Ok(result)
        }
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
        Patron::Constructor(nombre, subpatrones) => {
           match (nombre.as_str(), valor) {
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
            }
        }
    }
}

// ─── Funciones públicas para integración ───────────────────────

/// Inicializa el estado evaluando:
/// 1. Variables de módulo (declaradas fuera de funciones)
/// 2. La función `main()` para inicializar el estado dinámico
pub fn inicializar_estado(declaraciones: &[Declaracion], store: &mut VariableStore) {
    // 1. Procesar variables de módulo (declaraciones fuera de funciones)
    //    Usa el evaluador completo para soportar llamadas a funciones
    //    (ej: conexion = _sqlite_abrir("forjanzas.db")) en lugar del
    //    match limitado a literales que convertía todo a Nulo.
    let mut ambito = Ambito::new();
    for decl in declaraciones {
        match decl {
            Declaracion::Variable { .. } | Declaracion::Asignacion { .. } => {
                // Usar evaluar_declaracion que maneja correctamente
                // todo tipo de expresión, incluyendo llamadas a funciones
                let _ = evaluar_declaracion(decl, &mut ambito, store, declaraciones);
            }
            _ => {}
        }
    }

    // 2. Ejecutar declaraciones de main() para inicializar estado dinámico.
    //    Procesa Variable, Asignacion, LlamadaFuncion y Expresion.
    //    Las funciones GUI del layout (columna, fila, boton, etc.) se evalúan
    //    pero devuelven Nulo o error, que se ignora silenciosamente.
    for decl in declaraciones {
        if let Declaracion::Funcion { nombre, cuerpo, .. } = decl {
            if nombre == "main" {
                for d in cuerpo {
                    match d {
                        Declaracion::Variable { .. } | Declaracion::Asignacion { .. } => {
                            match evaluar_declaracion(d, &mut ambito, store, declaraciones) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("[inicializar_estado] main: {} — ignorado", e);
                                }
                            }
                        }
                        Declaracion::LlamadaFuncion { nombre, .. } => {
                            // Solo ejecutar funciones reales (nativas o declaradas en el AST).
                            // Los widgets del layout GUI (andamio, barra_superior, navegador,
                            // etc.) no deben evaluarse aquí: se omiten silenciosamente para
                            // evitar falsos errores tipo "Variable 'buscar_transacciones' no encontrada".
                            if es_funcion_ejecutable(nombre, declaraciones) {
                                let _ = evaluar_declaracion(d, &mut ambito, store, declaraciones);
                            }
                        }
                        Declaracion::Expresion(..) => {
                            let _ = evaluar_declaracion(d, &mut ambito, store, declaraciones);
                        }
                        _ => {} // Ignorar Funcion, Clase, etc.
                    }
                }

                // 3. Copiar todas las variables del ámbito local al store global.
                //    Esto asegura que las variables definidas en main() (como conexion_bd,
                //    buscar_transacciones, etc.) sean accesibles desde los callbacks
                //    de los widgets, que se ejecutan en un ámbito nuevo.
                for (nombre, valor) in ambito.obtener_todas() {
                    store.set(nombre, valor.to_json_value());
                }

                return;
            }
        }
    }
}
