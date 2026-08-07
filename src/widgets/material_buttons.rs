// Material Design 3 — Botones: FAB, IconButton, SegmentedButton
// Funciones de rendering extraídas de gui_nativa.rs

use crate::view;
use crate::helpers::*;
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme, RgbColor};
use crate::gui_nativa::{AppStateNativo, FabSize, IconButtonVariant};
use forja::ast::Declaracion;
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
    let icon_label = crate::icons::catalog::fallback_emoji(icono);
    let texto_fab = match texto_extendido {
        Some(ext) => format!("{} {}", icon_label, ext),
        None => icon_label.to_string(),
    };
    let label = view::label(texto_fab)
        .text_size(fab_icon_size)
        .weight(FontWeight::MEDIUM)
        .color(fg);
    let cb_log = cb.clone();
    let btn = view::button(label, move |data: &mut AppStateNativo| {
        eprintln!("[FAB] Click detectado, ejecutando callback '{}'", &cb_log);
        ejecutar_callback_y_actualizar(&cb, data, &prog);
    });
    // FAB flotante: botón con fondo, padding y esquinas redondeadas.
    Box::new(
        btn.padding(16.0)
            .background(Background::Color(bg))
            .corner_radius(28.0),
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
            let icon_view = crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            Box::new(view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            }))
        }
        IconButtonVariant::Filled => {
            let fg_rgb: RgbColor = scheme.on_primary;
            let bg: crate::Color = scheme.primary.into();
            let icon_view = crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(btn.background(Background::Color(bg)).corner_radius(20.0))
        }
        IconButtonVariant::Tonal => {
            let fg_rgb: RgbColor = scheme.on_secondary_container;
            let bg: crate::Color = scheme.secondary_container.into();
            let icon_view = crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(btn.background(Background::Color(bg)).corner_radius(20.0))
        }
        IconButtonVariant::Outlined => {
            let fg_rgb: RgbColor = scheme.primary;
            let border: crate::Color = scheme.outline.into();
            let icon_view = crate::svg_icon::<AppStateNativo>(icono, 24.0, fg_rgb, crate::IconStyle::Filled);
            let btn = view::button(icon_view, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            });
            Box::new(
                btn.border_color(border)
                    .border_width(1.0)
                    .corner_radius(20.0),
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
                btn.background(Background::Color(bg)).corner_radius(8.0),
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
                    .border_width(1.0)
                    .corner_radius(8.0),
            ));
        }
    }

    Box::new(view::flex(crate::view::Axis::Horizontal, (widgets,)).gap(crate::Length::px(0.0)))
}
