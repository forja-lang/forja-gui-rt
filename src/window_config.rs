// Configuración de la ventana declarada desde el código Forja.
//
//     funcion main() {
//         ventana("Mi app", 1200, 800, "icono.png", 800, 600, verdadero, verdadero)
//         tema_fluent(columna(...))
//     }
//
// Los argumentos son posicionales y los de la derecha son opcionales: la llamada
// se lee del AST antes de crear la ventana y no produce ningún widget.

use forja_syntax::ast::*;
use xilem::winit::window::Icon as XilemIcon;

/// Configuración de la ventana nativa. Los valores por defecto son los que usaba
/// el runtime antes de existir esta función.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    /// Título de la barra. Si es `None` se arma con el diseño del tema.
    pub titulo: Option<String>,
    pub ancho: f64,
    pub alto: f64,
    /// Tamaño mínimo en px lógicos (evita que la ventana se achique al punto de
    /// que el layout se rompa).
    pub ancho_minimo: Option<f64>,
    pub alto_minimo: Option<f64>,
    /// Ruta a un PNG/ICO para el icono de la ventana y la barra de tareas.
    pub icono: Option<String>,
    pub redimensionable: bool,
    /// `false` = ventana sin barra de título ni borde del sistema (para dibujar
    /// el chrome propio).
    pub decoraciones: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            titulo: None,
            ancho: 1200.0,
            alto: 800.0,
            ancho_minimo: None,
            alto_minimo: None,
            icono: None,
            redimensionable: true,
            decoraciones: true,
        }
    }
}

impl WindowConfig {
    /// Carga el icono desde disco (PNG, JPG, BMP, ICO o cualquier formato que
    /// entienda el crate `image`) y lo convierte al tipo de winit.
    pub fn cargar_icono(&self) -> Option<XilemIcon> {
        let ruta = self.icono.as_ref()?;
        let img = image::open(ruta).ok()?.into_rgba8();
        let (ancho, alto) = (img.width(), img.height());
        XilemIcon::from_rgba(img.into_raw(), ancho, alto).ok()
    }
}

/// Nombres aceptados para declarar la ventana.
const FN_VENTANA: &[&str] = &[
    "ventana",
    "configurar_ventana",
    "window",
    "configurar_ventana_gui",
];

/// Busca `ventana(...)` en `main` (o en el nivel superior) y arma la config.
///
/// Es tolerante a propósito: un argumento con el tipo equivocado se ignora y se
/// queda el valor por defecto, porque un error acá impediría abrir la ventana.
pub fn extraer_config_ventana(decls: &[Declaracion]) -> WindowConfig {
    let mut cfg = WindowConfig::default();

    for decl in decls {
        match decl {
            // `ventana(...)` suelto a nivel superior
            otra => aplicar(&mut cfg, otra),
        }
        if let Declaracion::Funcion { nombre, cuerpo, .. } = decl {
            if nombre == "main" {
                for sentencia in cuerpo {
                    aplicar(&mut cfg, sentencia);
                }
            }
        }
    }
    cfg
}

fn aplicar(cfg: &mut WindowConfig, decl: &Declaracion) {
    let (nombre, argumentos) = match decl {
        Declaracion::LlamadaFuncion { nombre, argumentos } => (nombre, argumentos),
        Declaracion::Expresion(Expresion::LlamadaFuncion { nombre, argumentos }) => {
            (nombre, argumentos)
        }
        _ => return,
    };
    if !FN_VENTANA.contains(&nombre.as_str()) {
        return;
    }

    // ventana(titulo, ancho, alto, icono, min_ancho, min_alto, redimensionable, decoraciones)
    if let Some(Expresion::LiteralTexto(t)) = argumentos.first() {
        cfg.titulo = Some(t.clone());
    }
    if let Some(v) = numero(argumentos.get(1)) {
        cfg.ancho = v;
    }
    if let Some(v) = numero(argumentos.get(2)) {
        cfg.alto = v;
    }
    if let Some(Expresion::LiteralTexto(icono)) = argumentos.get(3) {
        if !icono.is_empty() {
            cfg.icono = Some(icono.clone());
        }
    }
    if let Some(v) = numero(argumentos.get(4)) {
        cfg.ancho_minimo = Some(v);
    }
    if let Some(v) = numero(argumentos.get(5)) {
        cfg.alto_minimo = Some(v);
    }
    if let Some(Expresion::LiteralBooleano(b)) = argumentos.get(6) {
        cfg.redimensionable = *b;
    }
    if let Some(Expresion::LiteralBooleano(b)) = argumentos.get(7) {
        cfg.decoraciones = *b;
    }
}

fn numero(expr: Option<&Expresion>) -> Option<f64> {
    match expr? {
        Expresion::LiteralNumero(n) => Some(*n as f64),
        Expresion::LiteralDecimal(d) => Some(*d),
        _ => None,
    }
}

/// Despega el proceso de la consola cuando la app GUI se lanzó desde el
/// Explorador o como ejecutable de consola.
///
/// Sin esto, un binario de consola abre una ventana negra que aparece y
/// desaparece antes de que se dibuje la GUI. `FreeConsole` sólo afecta a este
/// proceso: si lo lanzaste desde una terminal (`forja ejecutar --native`), la
/// terminal sigue viva y sigue mostrando la salida del padre.
///
/// Se puede mantener la consola con `FORJA_CONSOLA=1` (útil para ver panics).
#[cfg(target_os = "windows")]
pub fn ocultar_consola_si_corresponde() {
    if std::env::var("FORJA_CONSOLA")
        .map(|v| v != "0")
        .unwrap_or(false)
    {
        return;
    }
    // Si el proceso es dueño de la consola (lanzado con doble clic) o la heredó,
    // despegarse evita la ventana negra. No falla si no hay consola.
    #[link(name = "kernel32")]
    extern "system" {
        fn FreeConsole() -> i32;
    }
    unsafe {
        FreeConsole();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn ocultar_consola_si_corresponde() {}
