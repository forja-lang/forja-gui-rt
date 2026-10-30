// Material Design 3 — Botones: FAB, IconButton, SegmentedButton
// Funciones de rendering extraídas de gui_nativa.rs

use crate::gui_nativa::{AppStateNativo, FabSize, IconButtonVariant};
use crate::helpers::*;
use crate::layout::AsUnit as _;
use crate::view;
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme, RgbColor, ShapeFamily};
use forja_syntax::ast::Declaracion;
use xilem::style::Style;

// ─── FAB (Floating Action Button) ───────────────────────────────────

pub(crate) fn render_fab(
    icono: &str,
    callback: &str,
    size: &FabSize,
    texto_extendido: &Option<String>,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let scheme = &theme.scheme;

    let fab_icon_size = match size {
        FabSize::Small => 18.0,
        FabSize::Medium => 24.0,
        FabSize::Large => 36.0,
    };
    let fg_rgb: RgbColor = scheme.on_primary_container;
    let fg: crate::Color = fg_rgb.into();
    let bg: crate::Color = scheme.primary_container.into();
    // Icono SVG real (Vello). Si el nombre no está en el catálogo (p. ej. el
    // usuario pasó texto o un emoji propio), se muestra como etiqueta.
    let icono_view: Box<AnyWidgetView<AppStateNativo>> =
        if crate::icons::catalog::by_name(icono).is_some() {
            Box::new(crate::svg_icon::<AppStateNativo>(
                icono,
                fab_icon_size,
                fg_rgb,
                crate::IconStyle::Filled,
            ))
        } else {
            Box::new(
                view::label(icono.to_string())
                    .text_size(fab_icon_size as f32)
                    .color(fg),
            )
        };
    let mut partes: Vec<Box<AnyWidgetView<AppStateNativo>>> = vec![icono_view];
    if let Some(ext) = texto_extendido {
        partes.push(Box::new(
            view::label(ext.clone())
                .text_size(14.0)
                .weight(FontWeight::MEDIUM)
                .color(fg),
        ));
    }
    let contenido = view::flex(crate::Axis::Horizontal, (partes,))
        .cross_axis_alignment(crate::CrossAxisAlignment::Center)
        .gap(crate::Length::px(8.0));
    let cb_log = cb.clone();
    let btn = view::button(contenido, move |data: &mut AppStateNativo| {
        eprintln!("[FAB] Click detectado, ejecutando callback '{}'", &cb_log);
        ejecutar_callback_y_actualizar(&cb, data, &prog);
    });
    // FAB flotante: botón con fondo, padding y esquinas redondeadas.
    Box::new(
        btn.padding(16.0.px())
            .background(Background::Color(bg))
            .corner_radius(theme.shapes.extra_large.px()),
    )
}

// ─── IconButton (4 variantes) ──────────────────────────────────────

pub(crate) fn render_icon_button(
    icono: &str,
    callback: &str,
    variant: &IconButtonVariant,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let scheme = &theme.scheme;
    match variant {
        IconButtonVariant::Standard => {
            let fg_rgb: RgbColor = scheme.on_surface_variant;
            let icon_view =
                crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            Box::new(view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            }))
        }
        IconButtonVariant::Filled => {
            let fg_rgb: RgbColor = scheme.on_primary;
            let bg: crate::Color = scheme.primary.into();
            let icon_view =
                crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(
                btn.background(Background::Color(bg))
                    .corner_radius(theme.shapes.for_family(ShapeFamily::Button).px()),
            )
        }
        IconButtonVariant::Tonal => {
            let fg_rgb: RgbColor = scheme.on_secondary_container;
            let bg: crate::Color = scheme.secondary_container.into();
            let icon_view =
                crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(
                btn.background(Background::Color(bg))
                    .corner_radius(theme.shapes.for_family(ShapeFamily::Button).px()),
            )
        }
        IconButtonVariant::Outlined => {
            let fg_rgb: RgbColor = scheme.primary;
            let border: crate::Color = scheme.outline.into();
            let icon_view =
                crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(
                btn.border_color(border)
                    .border_width(1.0.px())
                    .corner_radius(theme.shapes.for_family(ShapeFamily::Button).px()),
            )
        }
    }
}

// ─── SegmentedButton ────────────────────────────────────────────────

pub(crate) fn render_segmented_button(
    opciones: &[String],
    seleccionados: &[bool],
    callback: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let scheme = &theme.scheme;
    let label_style = get_text_style(&theme.typography, "label_large");

    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();

    for (i, texto) in opciones.iter().enumerate() {
        let cb_inner = cb.clone();
        let t = texto.clone();
        let prog_inner = prog.clone();
        let is_selected = seleccionados.get(i).copied().unwrap_or(false);

        if is_selected {
            let fg: crate::Color = scheme.on_secondary_container.into();
            let bg: crate::Color = scheme.secondary_container.into();
            let label = view::label(t.clone())
                .text_size(label_style.font_size as f32)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            widgets.push(Box::new(
                btn.background(Background::Color(bg))
                    .corner_radius(theme.shapes.small.px()),
            ));
        } else {
            let fg: crate::Color = scheme.on_surface.into();
            let border: crate::Color = scheme.outline.into();
            let label = view::label(t.clone())
                .text_size(label_style.font_size as f32)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            widgets.push(Box::new(
                btn.border_color(border)
                    .border_width(1.0.px())
                    .corner_radius(theme.shapes.small.px()),
            ));
        }
    }

    Box::new(view::flex(crate::Axis::Horizontal, (widgets,)).gap(crate::Length::px(0.0)))
}
