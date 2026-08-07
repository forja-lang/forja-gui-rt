// ─── Funciones helper extraídas de gui_nativa.rs ──────────────────
// Este módulo centraliza las funciones de utilidad compartidas:
// extracción de argumentos AST, temas/colores, callbacks y builders de botones.

use crate::*;
use crate::gui_nativa::*;
use forja::ast::*;

// ─── Helpers para extraer argumentos de funciones Forja ─────────────

pub(crate) fn extraer_texto(args: &[Expresion], index: usize) -> String {
    args.get(index)
        .map(|a| match a {
            Expresion::LiteralTexto(s) => s.clone(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_callback(args: &[Expresion], index: usize) -> String {
    args.get(index)
        .map(|a| match a {
            Expresion::Referencia { expr, .. } => match expr.as_ref() {
                Expresion::Identificador { nombre: n, .. } => n.clone(),
                // Soporte: &cambiar_pantalla(indice) → extraer "cambiar_pantalla"
                Expresion::LlamadaFuncion { nombre: n, .. } => n.clone(),
                _ => String::new(),
            },
            Expresion::Identificador { nombre: n, .. } => n.clone(),
            // Soporte directo: cambiar_pantalla(indice) → extraer "cambiar_pantalla"
            Expresion::LlamadaFuncion { nombre: n, .. } => n.clone(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_booleano(args: &[Expresion], index: usize) -> bool {
    args.get(index)
        .and_then(|a| match a {
            Expresion::LiteralBooleano(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false)
}

pub(crate) fn extraer_array_strings(args: &[Expresion], index: usize) -> Vec<String> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LiteralTexto(s) => Some(s.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_array_bool(args: &[Expresion], index: usize) -> Vec<bool> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LiteralBooleano(b) => Some(*b),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_array_arrays_strings(args: &[Expresion], index: usize) -> Vec<Vec<String>> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::Arreglo(inner) => Some(
                            inner
                                .iter()
                                .filter_map(|x| match x {
                                    Expresion::LiteralTexto(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>(),
                        ),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_nav_items(args: &[Expresion], index: usize) -> Vec<NavItem> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LlamadaFuncion { nombre, argumentos }
                            if nombre == "item_navegacion" =>
                        {
                            let icono = argumentos
                                .first()
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            let label = argumentos
                                .get(1)
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            let badge = argumentos
                                .get(2)
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .unwrap_or(None);
                            Some(NavItem {
                                icono,
                                label,
                                badge,
                            })
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_navigator_screens(args: &[Expresion], index: usize) -> Vec<NavigatorScreen> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LlamadaFuncion { nombre, argumentos }
                            if nombre == "pantalla" || nombre == "screen" =>
                        {
                            let id = argumentos
                                .first()
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            let titulo = argumentos
                                .get(1)
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            let (contenido, content_fn) = argumentos
                                .get(2)
                                .map(|arg| {
                                    let layout = expr_a_layout(arg);
                                    match layout {
                                        Some(l) => (l, None),
                                        None => {
                                            // Si expr_a_layout falló (ej: función personalizada),
                                            // extraer el nombre de la función para evaluarla después
                                            let fn_name = match arg {
                                                Expresion::LlamadaFuncion { nombre, .. } => Some(nombre.clone()),
                                                _ => None,
                                            };
                                            (Layout::Spacer(0.0), fn_name)
                                        }
                                    }
                                })
                                .unwrap_or((Layout::Spacer(0.0), None));
                            let icono = argumentos
                                .get(3)
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .unwrap_or(None);
                            Some(NavigatorScreen {
                                id,
                                titulo,
                                icono,
                                contenido: Box::new(contenido),
                                badge: None,
                                content_fn,
                            })
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_icon_actions(args: &[Expresion], index: usize) -> Vec<IconAction> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LlamadaFuncion { nombre, argumentos }
                            if nombre == "boton_icono" || nombre == "icon_button" =>
                        {
                            let icono = argumentos
                                .first()
                                .map(|a| match a {
                                    Expresion::LiteralTexto(s) => s.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            let callback = argumentos
                                .get(1)
                                .map(|a| match a {
                                    Expresion::Referencia { expr, .. } => match expr.as_ref() {
                                        Expresion::Identificador { nombre: n, .. } => n.clone(),
                                        _ => String::new(),
                                    },
                                    Expresion::Identificador { nombre: n, .. } => n.clone(),
                                    _ => String::new(),
                                })
                                .unwrap_or_default();
                            Some(IconAction { icono, callback })
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => {
                let result = args.get(index).and_then(|e| match e {
                    Expresion::LlamadaFuncion { nombre, argumentos }
                        if nombre == "boton_icono" || nombre == "icon_button" =>
                    {
                        let icono = argumentos
                            .first()
                            .map(|a| match a {
                                Expresion::LiteralTexto(s) => s.clone(),
                                _ => String::new(),
                            })
                            .unwrap_or_default();
                        let callback = argumentos
                            .get(1)
                            .map(|a| match a {
                                Expresion::Referencia { expr, .. } => match expr.as_ref() {
                                    Expresion::Identificador { nombre: n, .. } => n.clone(),
                                    _ => String::new(),
                                },
                                Expresion::Identificador { nombre: n, .. } => n.clone(),
                                _ => String::new(),
                            })
                            .unwrap_or_default();
                        Some(vec![IconAction { icono, callback }])
                    }
                    _ => None,
                });
                result
            }
        })
        .unwrap_or_default()
}

pub(crate) fn extraer_f64(args: &[Expresion], index: usize) -> f64 {
    args.get(index)
        .and_then(|a| match a {
            Expresion::LiteralNumero(n) => Some(*n as f64),
            Expresion::LiteralDecimal(f) => Some(*f),
            _ => None,
        })
        .unwrap_or(0.0)
}

pub(crate) fn extraer_usize(args: &[Expresion], index: usize) -> usize {
    args.get(index)
        .and_then(|a| match a {
            Expresion::LiteralNumero(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(0)
}

pub(crate) fn extraer_array_f64(args: &[Expresion], index: usize) -> Vec<f64> {
    args.get(index)
        .and_then(|a| match a {
            Expresion::Arreglo(exprs) => Some(
                exprs
                    .iter()
                    .filter_map(|e| match e {
                        Expresion::LiteralNumero(n) => Some(*n as f64),
                        Expresion::LiteralDecimal(f) => Some(*f),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            Expresion::LiteralNumero(n) => Some(vec![*n as f64]),
            Expresion::LiteralDecimal(f) => Some(vec![*f]),
            _ => None,
        })
        .unwrap_or_default()
}

// ─── Helpers de tema/color ─────────────────────────────────────────

/// Obtiene un color del esquema por su nombre de role
pub(crate) fn get_color_role(scheme: &ColorScheme, role: &str) -> RgbColor {
    match role {
        "primary" => scheme.primary,
        "on_primary" => scheme.on_primary,
        "primary_container" => scheme.primary_container,
        "on_primary_container" => scheme.on_primary_container,
        "secondary" => scheme.secondary,
        "on_secondary" => scheme.on_secondary,
        "secondary_container" => scheme.secondary_container,
        "on_secondary_container" => scheme.on_secondary_container,
        "tertiary" => scheme.tertiary,
        "on_tertiary" => scheme.on_tertiary,
        "tertiary_container" => scheme.tertiary_container,
        "on_tertiary_container" => scheme.on_tertiary_container,
        "error" => scheme.error,
        "on_error" => scheme.on_error,
        "error_container" => scheme.error_container,
        "on_error_container" => scheme.on_error_container,
        "surface" => scheme.surface,
        "on_surface" => scheme.on_surface,
        "surface_variant" => scheme.surface_variant,
        "on_surface_variant" => scheme.on_surface_variant,
        "background" => scheme.background,
        "on_background" => scheme.on_background,
        "outline" => scheme.outline,
        "outline_variant" => scheme.outline_variant,
        "inverse_surface" => scheme.inverse_surface,
        "inverse_on_surface" => scheme.inverse_on_surface,
        "inverse_primary" => scheme.inverse_primary,
        _ => scheme.primary,
    }
}

/// Convierte un color role (string) a Rgba para usar con chart_widgets
pub(crate) fn color_role_to_rgba(scheme: &ColorScheme, role: &str) -> Rgba {
    let rgb = get_color_role(scheme, role);
    Rgba::new(rgb.0, rgb.1, rgb.2, 255)
}

/// Devuelve un color según el signo del texto del monto:
/// "-$..." → error (rojo), "+$..." → terciario (verde), resto → on_surface.
pub(crate) fn color_segun_monto(scheme: &ColorScheme, texto: &str) -> Color {
    let t = texto.trim_start();
    if t.starts_with('-') {
        scheme.error.into()
    } else if t.starts_with('+') {
        scheme.tertiary.into()
    } else {
        scheme.on_surface.into()
    }
}

/// Obtiene el TextStyle de la escala tipográfica por nombre de estilo
pub(crate) fn get_text_style(typography: &TypeScale, style: &str) -> TextStyle {
    match style {
        "display_large" => typography.display_large,
        "display_medium" => typography.display_medium,
        "display_small" => typography.display_small,
        "headline_large" => typography.headline_large,
        "headline_medium" => typography.headline_medium,
        "headline_small" => typography.headline_small,
        "title_large" => typography.title_large,
        "title_medium" => typography.title_medium,
        "title_small" => typography.title_small,
        "body_large" => typography.body_large,
        "body_medium" => typography.body_medium,
        "body_small" => typography.body_small,
        "label_large" => typography.label_large,
        "label_medium" => typography.label_medium,
        "label_small" => typography.label_small,
        _ => typography.body_medium,
    }
}

/// Obtiene el radio de forma del sistema por nombre de familia
pub(crate) fn get_shape_radius(shapes: &ShapeSystem, family: &str) -> f64 {
    match family {
        "none" => shapes.none,
        "extra_small" | "extrasmall" => shapes.extra_small,
        "small" => shapes.small,
        "medium" => shapes.medium,
        "large" => shapes.large,
        "extra_large" | "extralarge" => shapes.extra_large,
        "full" => shapes.full,
        "button" => shapes.for_family(ShapeFamily::Button),
        "surface" => shapes.for_family(ShapeFamily::Surface),
        "container" => shapes.for_family(ShapeFamily::Container),
        _ => shapes.small,
    }
}

// ─── Helpers de alineación ─────────────────────────────────────────

/// Parsea un string de alineación a MainAxisAlignment de Xilem
pub(crate) fn parse_alignment(s: &str) -> MainAxisAlignment {
    match s.to_lowercase().as_str() {
        "start" | "inicio" | "izquierda" => MainAxisAlignment::Start,
        "center" | "centro" | "centrado" => MainAxisAlignment::Center,
        "end" | "fin" | "derecha" => MainAxisAlignment::End,
        "space_between" | "espacio_entre" => MainAxisAlignment::SpaceBetween,
        "space_around" | "espacio_alrededor" => MainAxisAlignment::SpaceAround,
        "space_evenly" | "espacio_igual" => MainAxisAlignment::SpaceEvenly,
        _ => MainAxisAlignment::Start,
    }
}

// ─── Colores ────────────────────────────────────────────────────────

/// Parsea un nombre de color a `Color` de Vello (usado por ColoredLabel legacy)
pub(crate) fn color_desde_nombre(nombre: &str) -> Color {
    match nombre.to_lowercase().as_str() {
        "rojo" | "red" => palette::css::RED,
        "azul" | "blue" => palette::css::BLUE,
        "verde" | "green" => palette::css::GREEN,
        "blanco" | "white" => palette::css::WHITE,
        "negro" | "black" => palette::css::BLACK,
        "gris" | "gray" | "grey" => palette::css::GRAY,
        "naranja" | "orange" => palette::css::ORANGE,
        "morado" | "purple" => palette::css::PURPLE,
        "amarillo" | "yellow" => palette::css::YELLOW,
        "cian" | "cyan" => palette::css::CYAN,
        "rosa" | "pink" => palette::css::PINK,
        "azul_marino" | "navy" => palette::css::NAVY,
        "plateado" | "silver" => palette::css::SILVER,
        "marron" | "brown" => palette::css::BROWN,
        "defecto" | "default" => palette::css::WHITE,
        _ => palette::css::WHITE,
    }
}

/// Parsea un string de color (hex #RRGGBB o nombre/role del tema) a RgbColor.
/// Soporta roles del tema: "primary", "secondary", "tertiary", etc.
pub(crate) fn parse_color(s: &str) -> Option<RgbColor> {
    if s.starts_with('#') {
        return RgbColor::from_hex(s);
    }
    match s.to_lowercase().as_str() {
        "primary" | "secundario" => Some(RgbColor(103, 80, 164)), // #6750A4
        "secondary" => Some(RgbColor(98, 91, 113)),               // #625B71
        "tertiary" | "terciario" => Some(RgbColor(125, 82, 96)),  // #7D5260
        "error" => Some(RgbColor(179, 38, 30)),                   // #B3261E
        "surface" | "superficie" => Some(RgbColor(255, 251, 254)), // #FFFBFE
        "primary_container" => Some(RgbColor(234, 221, 255)),     // #EADDFF
        "secondary_container" => Some(RgbColor(232, 222, 248)),   // #E8DEF8
        "tertiary_container" => Some(RgbColor(255, 216, 228)),    // #FFD8E4
        _ => {
            // Intentar como nombre de color estándar
            let c = RgbColor::from(s);
            if c != RgbColor(0, 0, 0) || s == "negro" || s == "black" {
                Some(c)
            } else {
                None
            }
        }
    }
}

// ─── Callback: ejecutar funciones Forja ─────────────────────────────

/// Actualiza el state con el resultado de un callback.
/// Usa el evaluador tree-walking completo con acceso mutable al store.
pub(crate) fn ejecutar_callback_y_actualizar(
    nombre_fn: &str,
    state: &mut AppStateNativo,
    programa: &[Declaracion],
) {
    match crate::evaluador::ejecutar_funcion(nombre_fn, &[], programa, &mut state.store) {
        Ok(valor) => {
            state.store.set("resultado", valor.to_json_value());
        }
        Err(e) => {
            eprintln!("Error ejecutando callback '{}': {}", nombre_fn, e);
            state.store.set(
                "resultado",
                serde_json::Value::String(format!("Error: {}", e)),
            );
        }
    }
}

// ─── Builders de botones Material Design 3 ──────────────────────────

/// Crea un botón Material Design 3 unificado.
/// Reemplaza las 5 variantes repetidas de MaterialButton.
/// Nota: El builder de xilem usa type-state, así que cada branch
/// construye y hace Box independientemente.
pub(crate) fn make_material_button(
    texto: &str,
    fg: Color,
    bg: Option<Color>,
    border: Option<Color>,
    label_style: TextStyle,
    cb: &str,
    prog: &[Declaracion],
    corner_radius: f64,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let label = view::label(texto.to_string())
        .text_size(label_style.font_size as f32)
        .weight(FontWeight::MEDIUM)
        .color(fg);
    let cb_owned = cb.to_string();
    let prog_owned = prog.to_vec();
    let btn = view::button(label, move |data: &mut AppStateNativo| {
        ejecutar_callback_y_actualizar(&cb_owned, data, &prog_owned);
    });
    // corner_radius SIEMPRE debe ser el último en la cadena builder
    if let Some(bg_color) = bg {
        Box::new(btn.background(Background::Color(bg_color)).corner_radius(corner_radius))
    } else if let Some(border_color) = border {
        Box::new(btn.border_color(border_color).border_width(1.0).corner_radius(corner_radius))
    } else {
        Box::new(btn.corner_radius(corner_radius))
    }
}

/// Crea un chip Material Design 3 unificado.
/// Reemplaza las 4 variantes repetidas de Chip.
pub(crate) fn make_chip_button(
    texto: &str,
    fg: Color,
    bg: Option<Color>,
    border: Option<Color>,
    label_style: TextStyle,
    cb: &str,
    prog: &[Declaracion],
    corner_radius: f64,
) -> Box<AnyWidgetView<AppStateNativo>> {
    // Chips usan la misma lógica que material buttons
    make_material_button(texto, fg, bg, border, label_style, cb, prog, corner_radius)
}
