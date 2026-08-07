// Material Design 3 — Tarjetas, Listas y Tablas: MaterialCard, MaterialListItem,
// MaterialList, MaterialListControl, MaterialListSelection, DynamicList,
// MaterialDataTable, MaterialSurface, MaterialScaffold
// Funciones de rendering extraídas de gui_nativa.rs

use crate::view;
use crate::helpers::*;
use crate::gui_nativa::{AppStateNativo, CardVariant, Layout, ValorGUI};
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme};
use crate::Length;
use forja::ast::Declaracion;
use std::collections::HashMap;
use xilem::style::Style;

// ─── MaterialCard ───────────────────────────────────────────────

pub(crate) fn render_card(
    child: &Layout,
    variant: &CardVariant,
    seleccionado: &bool,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let inner = crate::gui_nativa::layout_a_view(child, data, prog, theme);
    let padded = view::sized_box(inner).padding(16.0);
    let base = view::sized_box(padded).corner_radius(12.0);
    match variant {
        CardVariant::Filled => {
            let bg: crate::Color = scheme.surface_variant.into();
            Box::new(base.background(Background::Color(bg)))
        }
        CardVariant::Elevated => {
            let bg: crate::Color = scheme.surface_variant.into();
            let border: crate::Color = scheme.outline_variant.into();
            Box::new(
                base.background(Background::Color(bg))
                    .border_color(border)
                    .border_width(0.5),
            )
        }
        CardVariant::Outlined => {
            let bg: crate::Color = scheme.surface.into();
            let border: crate::Color = scheme.outline_variant.into();
            Box::new(
                base.background(Background::Color(bg))
                    .border_color(border)
                    .border_width(1.0),
            )
        }
        CardVariant::Selectable => {
            if *seleccionado {
                let bg: crate::Color = scheme.secondary_container.into();
                let border: crate::Color = scheme.secondary.into();
                Box::new(
                    base.background(Background::Color(bg))
                        .border_color(border)
                        .border_width(1.0),
                )
            } else {
                let bg: crate::Color = scheme.surface_variant.into();
                Box::new(base.background(Background::Color(bg)))
            }
        }
    }
}

// ─── MaterialListItem ────────────────────────────────────────────

pub(crate) fn render_list_item(
    leading: &Option<Box<Layout>>,
    titulo: &str,
    subtitulo: &Option<String>,
    trailing: &Option<Box<Layout>>,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let fg: crate::Color = scheme.on_surface.into();
    let fg_var: crate::Color = scheme.on_surface_variant.into();

    let mut text_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    text_children.push(
        Box::new(
            view::label(titulo.to_string())
                .text_size(16.0)
                .weight(FontWeight::MEDIUM)
                .color(fg),
        ) as Box<AnyWidgetView<AppStateNativo>>,
    );

    if let Some(sub) = subtitulo {
        if !sub.is_empty() {
            text_children.push(
                Box::new(view::label(sub.clone()).text_size(14.0).color(fg_var))
                    as Box<AnyWidgetView<AppStateNativo>>,
            );
        }
    }

    let text_col = Box::new(
        view::flex(crate::view::Axis::Vertical, (text_children,)).gap(Length::px(2.0)),
    ) as Box<AnyWidgetView<AppStateNativo>>;

    let leading_view = leading
        .as_ref()
        .map(|l| crate::gui_nativa::layout_a_view(l, data, prog, theme))
        .unwrap_or_else(|| {
            Box::new(view::sized_box(view::label(String::new())))
                as Box<AnyWidgetView<AppStateNativo>>
        });
    let trailing_view = trailing
        .as_ref()
        .map(|t| crate::gui_nativa::layout_a_view(t, data, prog, theme))
        .unwrap_or_else(|| {
            Box::new(view::sized_box(view::label(String::new())))
                as Box<AnyWidgetView<AppStateNativo>>
        });

    let row = view::flex(
        crate::view::Axis::Horizontal,
        (
            leading_view,
            xilem::view::FlexExt::flex(text_col, 1.0),
            trailing_view,
        ),
    );

    let bg: crate::Color = scheme.surface_variant.into();
    Box::new(
        view::sized_box(row)
            .padding(12.0)
            .background(Background::Color(bg))
            .corner_radius(12.0),
    )
}

// ─── MaterialList ───────────────────────────────────────────────

pub(crate) fn render_list(
    items: &[Layout],
    dividers: &bool,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        widgets.push(crate::gui_nativa::layout_a_view(item, data, prog, theme));
        if *dividers && i < items.len() - 1 {
            widgets.push(Box::new(
                view::sized_box(view::label(String::new())).height(Length::px(1.0)),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        }
    }
    Box::new(view::flex(crate::view::Axis::Vertical, (widgets,)).gap(Length::px(0.0)))
}

// ─── MaterialListControl ────────────────────────────────────────

pub(crate) fn render_list_control(
    items: &[Layout],
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for item in items.iter() {
        widgets.push(crate::gui_nativa::layout_a_view(item, data, prog, theme));
    }
    Box::new(view::flex(crate::view::Axis::Vertical, (widgets,)).gap(Length::px(0.0)))
}

// ─── MaterialListSelection ──────────────────────────────────────

pub(crate) fn render_list_selection(
    items: &[Layout],
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for item in items.iter() {
        widgets.push(crate::gui_nativa::layout_a_view(item, data, prog, theme));
    }
    Box::new(view::flex(crate::view::Axis::Vertical, (widgets,)).gap(Length::px(0.0)))
}

// ─── DynamicList ────────────────────────────────────────────────

pub(crate) fn render_dynamic_list(
    variable: &str,
    item_fn: &str,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let var_name = variable.to_string();
    let fn_name = item_fn.to_string();
    let p = prog.to_vec();
    let store = data.store.clone();
    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();

    if let Some(v) = store.get(&var_name) {
        let s = match &v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
            if let serde_json::Value::Array(arr) = json {
                for decl in prog {
                    if let forja::ast::Declaracion::Funcion { nombre, parametros, cuerpo, .. } = decl {
                        if nombre == &fn_name {
                            if let Some(forja::ast::Declaracion::Retornar { valor }) = cuerpo.iter().find(|d| matches!(d, forja::ast::Declaracion::Retornar { .. })) {
                                if let Some(expr) = valor {
                                    for item in &arr {
                                        let mut params = HashMap::new();
                                        if let Some(param) = parametros.first() {
                                            params.insert(
                                                param.nombre.clone(),
                                                ValorGUI::from_serde(item),
                                            );
                                        }
                                        if let Some(layout) = crate::gui_nativa::expr_a_layout_item(expr, &params, &store, &p) {
                                            widgets.push(crate::gui_nativa::layout_a_view(&layout, data, &p, theme));
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
    Box::new(
        view::flex(crate::view::Axis::Vertical, (widgets,))
            .gap(Length::px(6.0))
            .cross_axis_alignment(crate::CrossAxisAlignment::Fill),
    )
}

// ─── MaterialDataTable ──────────────────────────────────────────

pub(crate) fn render_data_table(
    columnas: &[String],
    filas: &[Vec<String>],
    ordenable: &bool,
    col_orden: &usize,
    orden_asc: &bool,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let label_style = get_text_style(&theme.typography, "label_small");
    let fg_header: crate::Color = scheme.on_surface.into();
    let fg_body: crate::Color = scheme.on_surface.into();
    let bg_header: crate::Color = scheme.surface_variant.into();
    let bg_row1: crate::Color = scheme.surface.into();
    let bg_row2: crate::Color = scheme.surface_variant.into();

    // Header row
    let mut header_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, col) in columnas.iter().enumerate() {
        let is_ordered = *ordenable && i == *col_orden;
        let display = if is_ordered {
            format!("{} {}", col, if *orden_asc { "↑" } else { "↓" })
        } else {
            col.clone()
        };
        let hdr = view::label(display)
            .text_size(label_style.font_size as f32)
            .weight(FontWeight::BOLD)
            .color(fg_header);
        header_widgets.push(Box::new(view::sized_box(hdr).padding(8.0))
            as Box<AnyWidgetView<AppStateNativo>>);
    }

    let header_row =
        Box::new(view::flex(crate::view::Axis::Horizontal, (header_widgets,)).gap(Length::px(8.0)))
            as Box<AnyWidgetView<AppStateNativo>>;

    let header_container = Box::new(
        view::sized_box(header_row)
            .background(Background::Color(bg_header))
            .corner_radius(4.0),
    );

    // Body rows
    let mut body_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (row_idx, fila) in filas.iter().enumerate() {
        let mut cell_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
        for celda in fila.iter() {
            let cell = view::label(celda.clone()).text_size(14.0).color(fg_body);
            cell_widgets.push(Box::new(view::sized_box(cell).padding(8.0))
                as Box<AnyWidgetView<AppStateNativo>>);
        }
        let row_bg = if row_idx % 2 == 0 { bg_row1 } else { bg_row2 };
        let row =
            Box::new(view::flex(crate::view::Axis::Horizontal, (cell_widgets,)).gap(Length::px(8.0)))
                as Box<AnyWidgetView<AppStateNativo>>;
        body_widgets.push(Box::new(
            view::sized_box(row).background(Background::Color(row_bg)),
        ) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let body = Box::new(view::flex(crate::view::Axis::Vertical, (body_widgets,)).gap(Length::px(0.0)));

    Box::new(view::flex(crate::view::Axis::Vertical, (header_container, body)).gap(Length::px(4.0)))
}

// ─── MaterialSurface ────────────────────────────────────────────

pub(crate) fn render_surface(
    child: &Layout,
    color_role: &str,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let inner = crate::gui_nativa::layout_a_view(child, data, prog, theme);
    let bg: crate::Color = match color_role {
        "tonal" => scheme.secondary_container.into(),
        "primary" => scheme.primary.into(),
        _ => scheme.surface.into(),
    };
    Box::new(
        view::sized_box(inner)
            .background(Background::Color(bg))
            .corner_radius(12.0),
    )
}

// ─── MaterialScaffold ───────────────────────────────────────────

pub(crate) fn render_scaffold(
    top: &Option<Box<Layout>>,
    body: &Layout,
    bottom: &Option<Box<Layout>>,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let body_view = crate::gui_nativa::layout_a_view(body, data, prog, theme);
    let body_flex = xilem::view::FlexExt::flex(body_view, 1.0);
    let tv = top
        .as_ref()
        .map(|t| crate::gui_nativa::layout_a_view(t, data, prog, theme))
        .unwrap_or_else(|| {
            Box::new(view::sized_box(view::label(String::new())))
                as Box<AnyWidgetView<AppStateNativo>>
        });
    let bv = bottom
        .as_ref()
        .map(|b| crate::gui_nativa::layout_a_view(b, data, prog, theme))
        .unwrap_or_else(|| {
            Box::new(view::sized_box(view::label(String::new())))
                as Box<AnyWidgetView<AppStateNativo>>
        });
    Box::new(
        view::sized_box(
            view::flex(crate::view::Axis::Vertical, (tv, body_flex, bv))
                .gap(Length::px(0.0))
                .must_fill_major_axis(true)
                .cross_axis_alignment(crate::CrossAxisAlignment::Fill),
        )
        .width(Length::px(data.window_width.max(200.0)))
        .height(Length::px(data.window_height.max(200.0)))
    )
}
