// Forja GUI — Pickers: DatePicker, TimePicker, ColorPicker
//
// Implementaciones como vistas Xilem compuestas (flex + button + label)
// consistentes con el resto de gui_nativa.rs.

use crate::*;
use crate::view::{self, Axis};
use crate::Length;
use crate::theme::{MaterialTheme, ColorScheme};
use crate::gui_nativa::{AppStateNativo, ValorGUI};
use chrono::{NaiveDate, Datelike, Timelike, Local};

// ═══════════════════════════════════════════════════════════════════
// CONSTANTES COMPARTIDAS
// ═══════════════════════════════════════════════════════════════════

/// Días de la semana abreviados (español)
const DIAS_SEMANA: &[&str] = &["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

/// Nombres de meses en español
const MESES: &[&str] = &[
    "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio",
    "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre",
];

/// Colores predefinidos para ColorPicker (basados en Material Design)
pub const COLORES_PICKER: &[&[u8; 3]] = &[
    // Rojos
    b"\xE5\x39\x35", b"\xE8\x69\x60", b"\xEF\x9A\x90", b"\xFF\xCD\xD2",
    // Rosas
    b"\xD8\x1B\x60", b"\xEC\x40\x7A", b"\xF0\x62\x92", b"\xF8\xBB\xD0",
    // Púrpuras
    b"\x8E\x24\xAA", b"\xAB\x47\xBC", b"\xCE\x93\xD8", b"\xE1\xBE\xE7",
    // Azules
    b"\x1E\x88\xE5", b"\x42\xA5\xF5", b"\x64\xB5\xF6", b"\xBB\xDE\xFB",
    // Cian
    b"\x00\x96\x88", b"\x26\xA6\x9A", b"\x4D\xB6\xAC", b"\xB2\xDF\xDB",
    // Verde
    b"\x43\xA0\x47", b"\x66\xBB\x6A", b"\x81\xC7\x84", b"\xC8\xE6\xC9",
    // Lima
    b"\xC0\xCA\x33", b"\xD4\xE1\x57", b"\xDC\xE7\x75", b"\xF0\xF4\xC3",
    // Amarillo
    b"\xFD\xD8\x35", b"\xFF\xEE\x58", b"\xFF\xF1\x76", b"\xFF\xF9\xC4",
    // Naranja
    b"\xFB\x8C\x00", b"\xFF\xA7\x26", b"\xFF\xB7\x4D", b"\xFF\xE0\xB2",
    // Marrón
    b"\x6D\x4C\x41", b"\x8D\x6E\x63", b"\xA1\x88\x7F", b"\xD7\xCC\xC8",
    // Gris
    b"\x75\x75\x75", b"\x9E\x9E\x9E", b"\xBD\xBD\xBD", b"\xEE\xEE\xEE",
    // Pizarra
    b"\x45\x50\x5A", b"\x54\x6E\x7A", b"\x78\x90\x9C", b"\xCF\xD8\xDC",
];

/// Nombres descriptivos de cada color (para tooltip/label)
pub const NOMBRES_COLORES: &[&str] = &[
    "Rojo 900", "Rojo 600", "Rojo 300", "Rojo 100",
    "Rosa 900", "Rosa 600", "Rosa 300", "Rosa 100",
    "Púrpura 900", "Púrpura 600", "Púrpura 300", "Púrpura 100",
    "Azul 900", "Azul 600", "Azul 300", "Azul 100",
    "Cian 900", "Cian 600", "Cian 300", "Cian 100",
    "Verde 900", "Verde 600", "Verde 300", "Verde 100",
    "Lima 900", "Lima 600", "Lima 300", "Lima 100",
    "Amarillo 900", "Amarillo 600", "Amarillo 300", "Amarillo 100",
    "Naranja 900", "Naranja 600", "Naranja 300", "Naranja 100",
    "Marrón 900", "Marrón 600", "Marrón 300", "Marrón 100",
    "Gris 900", "Gris 600", "Gris 300", "Gris 100",
    "Pizarra 900", "Pizarra 600", "Pizarra 300", "Pizarra 100",
];

// ═══════════════════════════════════════════════════════════════════
// DATEPICKER — Selector de fecha con calendario visual
// ═══════════════════════════════════════════════════════════════════

/// Construye un DatePicker Xilem View que permite seleccionar una fecha
/// con navegación entre meses.
///
/// Usa chrono para calcular días reales del mes y primer día de la semana.
pub fn date_picker_view(
    variable: &str,
    data: &mut AppStateNativo,
    scheme: &ColorScheme,
    _theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let var_name = variable.to_string();
    
    // Leer fecha actual de la variable o usar hoy
    let current_val = data.leer(&var_name).to_string();
    let today = Local::now().naive_local().date();
    
    // Parsear fecha actual (YYYY-MM-DD) o usar hoy
    let (year, mut month, day) = if current_val.len() == 10 && current_val.contains('-') {
        let parts: Vec<&str> = current_val.split('-').collect();
        if parts.len() == 3 {
            let y = parts[0].parse::<i32>().unwrap_or(today.year());
            let m = parts[1].parse::<u32>().unwrap_or(today.month());
            let d = parts[2].parse::<u32>().unwrap_or(today.day());
            (y, m, d)
        } else {
            (today.year(), today.month(), today.day())
        }
    } else {
        (today.year(), today.month(), today.day())
    };
    
    // Validar mes
    if month < 1 { month = 1; }
    if month > 12 { month = 12; }
    
    // Calcular días del mes y primer día de la semana
    let days_in_month = calc_days_in_month(year, month);
    let first_weekday = calc_first_weekday(year, month); // 0=Dom, 1=Lun...
    
    // Convertir a 0=Lun, 6=Dom para nuestra grilla
    let offset: usize = if first_weekday == 0 { 6 } else { first_weekday - 1 } as usize;
    
    // Header con mes/año y navegación
    let month_name = MESES.get(month as usize - 1).unwrap_or(&"");
    let header_text = format!("{} {}", month_name, year);
    
    let on_surface: Color = scheme.on_surface.into();
    let on_surface_variant: Color = scheme.on_surface_variant.into();
    let primary: Color = scheme.primary.into();
    let _surface_variant: Color = scheme.surface_variant.into();
    let primary_container: Color = scheme.primary_container.into();
    
    // Variable compartida para navegación (usamos closures)
    // Nota: Como Xilem 0.4 no tiene estado local fácil entre rebuilds,
    // usamos la variable del store para persistir año/mes/día
    
    // Botón mes anterior
    let var_prev = var_name.clone();
    let prev_btn = view::button(
        view::label("◀").text_size(16.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_prev).to_string();
            let today = Local::now().naive_local().date();
            let (mut y, mut m, d) = parse_fecha(&val, today);
            if m == 1 { m = 12; y -= 1; } else { m -= 1; }
            let fecha_str = format!("{:04}-{:02}-{:02}", y, m, d.min(calc_days_in_month(y, m)));
            data.escribir(&var_prev, ValorGUI::Texto(fecha_str));
        },
    );
    
    // Botón mes siguiente
    let var_next = var_name.clone();
    let next_btn = view::button(
        view::label("▶").text_size(16.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_next).to_string();
            let today = Local::now().naive_local().date();
            let (mut y, mut m, d) = parse_fecha(&val, today);
            if m == 12 { m = 1; y += 1; } else { m += 1; }
            let fecha_str = format!("{:04}-{:02}-{:02}", y, m, d.min(calc_days_in_month(y, m)));
            data.escribir(&var_next, ValorGUI::Texto(fecha_str));
        },
    );
    
    // Header row
    let header_row = view::flex(Axis::Horizontal, (
        Box::new(prev_btn) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(view::label(header_text).text_size(16.0).weight(FontWeight::BOLD).color(on_surface)) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(next_btn) as Box<AnyWidgetView<AppStateNativo>>,
    )).gap(Length::px(12.0)).cross_axis_alignment(CrossAxisAlignment::Center);
    
    // Días de la semana header
    let mut day_headers: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for d in DIAS_SEMANA {
        day_headers.push(
            Box::new(view::sized_box(
                view::label(d.to_string()).text_size(11.0).weight(FontWeight::BOLD).color(on_surface_variant)
            ).width(Length::px(36.0)).height(Length::px(36.0))) as Box<AnyWidgetView<AppStateNativo>>
        );
    }
    let day_header_row = view::flex(Axis::Horizontal, (day_headers,)).gap(Length::px(2.0));
    
    // Generar filas de días
    let mut all_rows: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    all_rows.push(Box::new(header_row) as Box<AnyWidgetView<AppStateNativo>>);
    all_rows.push(Box::new(day_header_row) as Box<AnyWidgetView<AppStateNativo>>);
    
    let total_cells = offset + days_in_month as usize;
    let num_rows = (total_cells + 6) / 7; // ceiling division
    
    for row in 0..num_rows {
        let mut day_cells: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
        for col in 0..7 {
            let cell_idx = row * 7 + col;
            if cell_idx < offset || cell_idx >= offset + days_in_month as usize {
                // Celda vacía (día fuera del mes)
                day_cells.push(
                    Box::new(view::sized_box(view::label(String::new())).width(Length::px(36.0)).height(Length::px(36.0))) as Box<AnyWidgetView<AppStateNativo>>
                );
            } else {
                let dia = (cell_idx - offset + 1) as u32;
                let var_cell = var_name.clone();
                let is_selected = dia == day;
                
                let cell = if is_selected {
                    // Día seleccionado: fondo primary_container
                    let lbl = view::label(dia.to_string()).text_size(12.0).weight(FontWeight::BOLD).color(on_surface);
                    Box::new(view::sized_box(lbl)
                        .width(Length::px(36.0)).height(Length::px(36.0))
                        .background(Background::Color(primary_container))
                        .corner_radius(18.0)) as Box<AnyWidgetView<AppStateNativo>>
                } else {
                    let btn = view::button(
                        view::label(dia.to_string()).text_size(12.0).color(on_surface),
                        move |data: &mut AppStateNativo| {
                            // Necesitamos año/mes actuales
                            let val = data.leer(&var_cell).to_string();
                            let today = Local::now().naive_local().date();
                            let (y, m, _) = parse_fecha(&val, today);
                            let fecha_str = format!("{:04}-{:02}-{:02}", y, m, dia);
                            data.escribir(&var_cell, ValorGUI::Texto(fecha_str));
                        },
                    );
                    Box::new(view::sized_box(btn).width(Length::px(36.0)).height(Length::px(36.0))) as Box<AnyWidgetView<AppStateNativo>>
                };
                day_cells.push(cell);
            }
        }
        all_rows.push(
            Box::new(view::flex(Axis::Horizontal, (day_cells,)).gap(Length::px(2.0))) as Box<AnyWidgetView<AppStateNativo>>
        );
    }
    
    Box::new(view::flex(Axis::Vertical, (all_rows,)).gap(Length::px(4.0)).cross_axis_alignment(CrossAxisAlignment::Center))
}

/// Parsea una fecha del formato "YYYY-MM-DD" o devuelve hoy
fn parse_fecha(val: &str, today: NaiveDate) -> (i32, u32, u32) {
    if val.len() == 10 && val.contains('-') {
        let parts: Vec<&str> = val.split('-').collect();
        if parts.len() == 3 {
            let y = parts[0].parse::<i32>().unwrap_or(today.year());
            let m = parts[1].parse::<u32>().unwrap_or(today.month()).max(1).min(12);
            let d = parts[2].parse::<u32>().unwrap_or(today.day()).max(1).min(31);
            return (y, m, d);
        }
    }
    (today.year(), today.month(), today.day())
}

/// Calcula el primer día de la semana para un mes/año (0=Dom, 1=Lun...)
fn calc_first_weekday(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.weekday().num_days_from_sunday())
        .unwrap_or(0)
}

/// Calcula los días que tiene un mes
fn calc_days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .and_then(|d| d.pred_opt())
            .map(|d| d.day())
            .unwrap_or(31)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .and_then(|d| d.pred_opt())
            .map(|d| d.day())
            .unwrap_or(30)
    }
}

// ═══════════════════════════════════════════════════════════════════
// TIMEPICKER — Selector de hora con botones +/-
// ═══════════════════════════════════════════════════════════════════

/// Construye un TimePicker Xilem View con ajuste de horas y minutos
pub fn time_picker_view(
    variable: &str,
    data: &mut AppStateNativo,
    scheme: &ColorScheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let var_name = variable.to_string();
    let current_val = data.leer(variable).to_string();
    let now = Local::now();
    
    // Parsear hora actual (HH:MM) o usar ahora
    let (hour, minute) = if current_val.len() >= 4 && current_val.contains(':') {
        let parts: Vec<&str> = current_val.split(':').collect();
        let h = parts[0].parse::<u32>().unwrap_or(now.hour());
        let m = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(now.minute());
        (h.min(23), m.min(59))
    } else {
        (now.hour(), now.minute())
    };
    
    let on_surface: Color = scheme.on_surface.into();
    let primary: Color = scheme.primary.into();
    let surface_variant: Color = scheme.surface_variant.into();
    let _primary_container: Color = scheme.primary_container.into();
    
    // Función para formatear hora
    let _fmt_hora = |h: u32, m: u32| -> String {
        format!("{:02}:{:02}", h, m)
    };
    
    let hour_str = format!("{:02}", hour);
    let min_str = format!("{:02}", minute);
    
    // Botón hora+
    let var_hup = var_name.clone();
    let hour_up = view::button(
        view::label("▲").text_size(12.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_hup).to_string();
            let (h, m) = parse_hora(&val);
            let new_h = if h >= 23 { 0 } else { h + 1 };
            data.escribir(&var_hup, ValorGUI::Texto(format!("{:02}:{:02}", new_h, m)));
        },
    );
    
    // Botón hora-
    let var_hdn = var_name.clone();
    let hour_down = view::button(
        view::label("▼").text_size(12.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_hdn).to_string();
            let (h, m) = parse_hora(&val);
            let new_h = if h == 0 { 23 } else { h - 1 };
            data.escribir(&var_hdn, ValorGUI::Texto(format!("{:02}:{:02}", new_h, m)));
        },
    );
    
    // Label hora
    let hour_label = view::label(hour_str).text_size(24.0).weight(FontWeight::BOLD).color(on_surface);
    let hour_box = view::sized_box(hour_label)
        .width(Length::px(48.0)).height(Length::px(48.0))
        .background(Background::Color(surface_variant))
        .corner_radius(8.0);
    
    // Columna hora
    let hour_col = view::flex(Axis::Vertical, (
        Box::new(hour_up) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(hour_box) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(hour_down) as Box<AnyWidgetView<AppStateNativo>>,
    )).gap(Length::px(2.0)).cross_axis_alignment(CrossAxisAlignment::Center);
    
    // Separador ":"
    let sep = view::label(":").text_size(24.0).weight(FontWeight::BOLD).color(on_surface);
    
    // Botón minuto+
    let var_mup = var_name.clone();
    let min_up = view::button(
        view::label("▲").text_size(12.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_mup).to_string();
            let (h, m) = parse_hora(&val);
            let new_m = if m >= 59 { 0 } else { m + 1 };
            data.escribir(&var_mup, ValorGUI::Texto(format!("{:02}:{:02}", h, new_m)));
        },
    );
    
    // Botón minuto-
    let var_mdn = var_name.clone();
    let min_down = view::button(
        view::label("▼").text_size(12.0).color(primary),
        move |data: &mut AppStateNativo| {
            let val = data.leer(&var_mdn).to_string();
            let (h, m) = parse_hora(&val);
            let new_m = if m == 0 { 59 } else { m - 1 };
            data.escribir(&var_mdn, ValorGUI::Texto(format!("{:02}:{:02}", h, new_m)));
        },
    );
    
    // Label minuto
    let min_label = view::label(min_str).text_size(24.0).weight(FontWeight::BOLD).color(on_surface);
    let min_box = view::sized_box(min_label)
        .width(Length::px(48.0)).height(Length::px(48.0))
        .background(Background::Color(surface_variant))
        .corner_radius(8.0);
    
    // Columna minuto
    let min_col = view::flex(Axis::Vertical, (
        Box::new(min_up) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(min_box) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(min_down) as Box<AnyWidgetView<AppStateNativo>>,
    )).gap(Length::px(2.0)).cross_axis_alignment(CrossAxisAlignment::Center);
    
    // Fila principal: hora : minuto
    let picker_row = view::flex(Axis::Horizontal, (
        Box::new(hour_col) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(sep) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(min_col) as Box<AnyWidgetView<AppStateNativo>>,
    )).gap(Length::px(8.0)).cross_axis_alignment(CrossAxisAlignment::Center);
    
    Box::new(view::sized_box(picker_row))
}

/// Parsea hora "HH:MM" → (hora, minuto)
fn parse_hora(val: &str) -> (u32, u32) {
    if val.len() >= 4 && val.contains(':') {
        let parts: Vec<&str> = val.split(':').collect();
        let h = parts[0].parse::<u32>().unwrap_or(12).min(23);
        let m = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0).min(59);
        (h, m)
    } else {
        let now = Local::now();
        (now.hour(), now.minute())
    }
}

// ═══════════════════════════════════════════════════════════════════
// COLORPICKER — Selector de color con grid de colores predefinidos
// ═══════════════════════════════════════════════════════════════════

/// Construye un ColorPicker Xilem View con:
/// - Grid de 48 colores predefinidos (Material Design)
/// - Preview del color seleccionado
/// - Campo hex editable
pub fn color_picker_view(
    variable: &str,
    data: &mut AppStateNativo,
    scheme: &ColorScheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let var_name = variable.to_string();
    let current_val = data.leer(variable).to_string();
    
    let on_surface: Color = scheme.on_surface.into();
    let surface_variant: Color = scheme.surface_variant.into();
    let outline: Color = scheme.outline.into();
    
    // Validar/parsear color actual
    let current_hex = if current_val.starts_with('#') && current_val.len() == 7 {
        current_val.clone()
    } else {
        "#E53935".to_string() // default: rojo
    };
    
    // Input para hex
    let hex_var = var_name.clone();
    let hex_input = view::text_input(
        current_hex.clone(),
        move |data: &mut AppStateNativo, new_val: String| {
            let clean = if new_val.starts_with('#') { new_val } else { format!("#{}", new_val) };
            data.escribir(&hex_var, ValorGUI::Texto(clean));
        },
    ).placeholder("#RRGGBB");
    
    let hex_input_box = view::sized_box(hex_input)
        .width(Length::px(120.0))
        .background(Background::Color(surface_variant))
        .border_color(outline)
        .border_width(1.0)
        .corner_radius(4.0);
    
    // Preview del color actual
    let preview_color = hex_to_color(&current_hex);
    let preview = view::sized_box(view::label("  "))
        .width(Length::px(32.0)).height(Length::px(32.0))
        .background(Background::Color(preview_color))
        .corner_radius(4.0)
        .border_color(outline)
        .border_width(1.0);
    
    // Fila: preview + hex input
    let top_row = view::flex(Axis::Horizontal, (
        Box::new(preview) as Box<AnyWidgetView<AppStateNativo>>,
        Box::new(hex_input_box) as Box<AnyWidgetView<AppStateNativo>>,
    )).gap(Length::px(8.0)).cross_axis_alignment(CrossAxisAlignment::Center);
    
    // Grid de colores: 12 columnas × 4 filas
    let mut grid_rows: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    let cols = 12;
    
    for row in 0..4 {
        let mut row_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
        for col in 0..cols {
            let idx = row * cols + col;
            if idx >= COLORES_PICKER.len() {
                break;
            }
            let rgb = COLORES_PICKER[idx];
            let hex_str = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
            let _color_name = NOMBRES_COLORES.get(idx).unwrap_or(&"");
            
            let var_click = var_name.clone();
            let is_selected = current_hex.to_uppercase() == hex_str.to_uppercase();
            
            let swatch_color = Color::from_rgba8(rgb[0], rgb[1], rgb[2], 255);
            
            if is_selected {
                // Seleccionado: borde primary más grueso
                let swatch = view::sized_box(view::label("").text_size(8.0))
                    .width(Length::px(24.0)).height(Length::px(24.0))
                    .background(Background::Color(swatch_color))
                    .corner_radius(4.0)
                    .border_color(scheme.primary.into())
                    .border_width(2.0);
                row_widgets.push(Box::new(swatch) as Box<AnyWidgetView<AppStateNativo>>);
            } else {
                let swatch_btn = view::button(
                    view::sized_box(view::label("").text_size(8.0))
                        .width(Length::px(24.0)).height(Length::px(24.0))
                        .background(Background::Color(swatch_color))
                        .corner_radius(4.0),
                    move |data: &mut AppStateNativo| {
                        data.escribir(&var_click, ValorGUI::Texto(hex_str.clone()));
                    },
                );
                row_widgets.push(Box::new(swatch_btn) as Box<AnyWidgetView<AppStateNativo>>);
            }
        }
        grid_rows.push(
            Box::new(view::flex(Axis::Horizontal, (row_widgets,)).gap(Length::px(4.0))) as Box<AnyWidgetView<AppStateNativo>>
        );
    }
    
    // Título
    let title = view::label("Selector de Color").text_size(14.0).weight(FontWeight::BOLD).color(on_surface);
    
    let mut all: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    all.push(Box::new(title) as Box<AnyWidgetView<AppStateNativo>>);
    all.push(Box::new(top_row) as Box<AnyWidgetView<AppStateNativo>>);
    for row in grid_rows { all.push(row); }
    
    Box::new(view::flex(Axis::Vertical, (all,)).gap(Length::px(6.0)))
}

/// Convierte un string hex "#RRGGBB" a Color
fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return Color::from_rgba8(r, g, b, 255);
        }
    }
    Color::from_rgba8(0xE5, 0x39, 0x35, 255) // default red
}
