// Material Design 3 — Indicadores: LinearProgress, CircularProgress, Badge,
// Skeleton, EmptyState, ErrorState, Avatar, AvatarGroup
// Funciones de rendering extraídas de gui_nativa.rs

use crate::gui_nativa::{AppStateNativo, AvatarVariant, Layout};
use crate::helpers::*;
use crate::layout::AsUnit as _;
use crate::view;
use crate::Length;
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme, RgbColor, ShapeFamily};
use forja_syntax::ast::Declaracion;
use xilem::style::Style;

// ─── LinearProgress ──────────────────────────────────────────────

pub(crate) fn render_linear_progress(
    variable: &str,
    indeterminado: &bool,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    // Widget nativo: ocupa el ancho disponible y toma sus colores del property
    // set del tema (antes se dibujaba a mano con 300px fijos).
    let progreso = if *indeterminado {
        None
    } else {
        Some((data.leer(variable).to_f64() / 100.0).clamp(0.0, 1.0))
    };
    let _ = theme; // los colores vienen de theme_props::default_properties
    Box::new(view::sized_box(view::progress_bar(progreso)).width(Length::px(
        (data.window_width - 64.0).max(120.0),
    )))
}

// ─── LinearProgressValue: barra con valor directo ───────────────────

pub(crate) fn render_linear_progress_value(
    val: &f64,
    _theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &_theme.scheme;
    let track_color: crate::Color = scheme.surface_variant.into();
    let indicator_color: crate::Color = scheme.primary.into();
    let valor = (*val / 100.0).clamp(0.0, 1.0);
    let filled_width = 300.0 * valor;
    let empty_width = 300.0 * (1.0 - valor);
    Box::new(
        view::sized_box(view::zstack((
            view::sized_box(view::label(String::new()))
                .dims((Length::px(300.0), Length::px(4.0)))
                .background(Background::Color(track_color))
                .corner_radius(2.0.px()),
            view::flex(
                crate::Axis::Horizontal,
                (
                    view::sized_box(view::label(String::new()))
                        .dims((Length::px(filled_width), Length::px(4.0)))
                        .background(Background::Color(indicator_color))
                        .corner_radius(2.0.px()),
                    view::sized_box(view::label(String::new()))
                        .dims((Length::px(empty_width), Length::px(4.0))),
                ),
            ),
        )))
        .width(Length::px(300.0)),
    )
}

// ─── CircularProgress ────────────────────────────────────────────

pub(crate) fn render_circular_progress(
    variable: &str,
    size: &f64,
    indeterminado: &bool,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let s = *size;
    let track_color: crate::Color = scheme.surface_variant.into();
    let indicator_color: crate::Color = scheme.primary.into();
    if *indeterminado {
        Box::new(
            view::sized_box(
                view::label("⟳")
                    .text_size((s * 0.5) as f32)
                    .color(indicator_color),
            )
            .dims((Length::px(s), Length::px(s)))
            .background(Background::Color(track_color))
            .corner_radius((s / 2.0).px()),
        )
    } else {
        let _valor = data.leer(variable).to_f64();
        Box::new(
            view::sized_box(
                view::label(format!("{:.0}%", _valor))
                    .text_size((s * 0.3) as f32)
                    .color(indicator_color),
            )
            .dims((Length::px(s), Length::px(s)))
            .border_color(indicator_color)
            .border_width(4.0.px())
            .corner_radius((s / 2.0).px())
            .background(Background::Color(track_color)),
        )
    }
}

// ─── Badge ──────────────────────────────────────────────────────

pub(crate) fn render_badge(
    child: &Layout,
    valor: &Option<String>,
    dot: &bool,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let inner = crate::gui_nativa::layout_a_view(child, data, prog, theme);
    let bg_color: crate::Color = scheme.error.into();
    let fg_color: crate::Color = scheme.on_error.into();
    if *dot {
        let dot_w = view::sized_box(view::label(String::new()))
            .dims((Length::px(8.0), Length::px(8.0)))
            .background(Background::Color(bg_color))
            .corner_radius(theme.shapes.small.px());
        Box::new(view::zstack((
            inner,
            Box::new(dot_w) as Box<AnyWidgetView<AppStateNativo>>,
        )))
    } else {
        let num = valor.clone().unwrap_or_default();
        let badge = view::sized_box(view::label(num).text_size(11.0).color(fg_color))
            .dims((Length::px(18.0), Length::px(18.0)))
            .background(Background::Color(bg_color))
            .corner_radius(theme.shapes.small.px());
        Box::new(view::zstack((
            inner,
            Box::new(badge) as Box<AnyWidgetView<AppStateNativo>>,
        )))
    }
}

// ─── Skeleton ───────────────────────────────────────────────────

pub(crate) fn render_skeleton(
    ancho: &f64,
    alto: &f64,
    tipo: &str,
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let sk_color: crate::Color = scheme.surface_variant.into();
    let radius = match tipo {
        "circulo" => ancho / 2.0,
        "tarjeta" => theme.shapes.for_family(ShapeFamily::Container),
        _ => theme.shapes.for_family(ShapeFamily::TextField),
    };
    Box::new(
        view::sized_box(view::label(String::new()))
            .dims((Length::px(*ancho), Length::px(*alto)))
            .background(Background::Color(sk_color))
            .corner_radius(radius.px()),
    )
}

// ─── EmptyState ─────────────────────────────────────────────────

pub(crate) fn render_empty_state(
    icono: &str,
    mensaje: &str,
    accion_texto: &Option<String>,
    accion_cb: &Option<String>,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let fg_var: crate::Color = scheme.on_surface_variant.into();
    let p = prog.to_vec();
    let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    if !icono.is_empty() {
        children.push(Box::new(view::label(icono.to_string()).text_size(48.0))
            as Box<AnyWidgetView<AppStateNativo>>);
    }
    children.push(Box::new(
        view::label(mensaje.to_string())
            .text_size(16.0)
            .color(fg_var),
    ) as Box<AnyWidgetView<AppStateNativo>>);
    if let Some(texto) = accion_texto {
        if let Some(cb_name) = accion_cb {
            let cb = cb_name.clone();
            let p = p.clone();
            let btn = view::button(
                view::label(texto.clone())
                    .text_size(14.0)
                    .weight(FontWeight::MEDIUM)
                    .color(scheme.primary.into()),
                move |data: &mut AppStateNativo| {
                    if !cb.is_empty() {
                        ejecutar_callback_y_actualizar(&cb, data, &p);
                    }
                },
            );
            children.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
        } else {
            children.push(Box::new(
                view::label(texto.clone())
                    .text_size(14.0)
                    .weight(FontWeight::MEDIUM)
                    .color(scheme.primary.into()),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        }
    }
    Box::new(
        view::flex(crate::Axis::Vertical, (children,))
            .main_axis_alignment(crate::MainAxisAlignment::Center)
            .gap(Length::px(12.0)),
    )
}

// ─── ErrorState ─────────────────────────────────────────────────

pub(crate) fn render_error_state(
    mensaje: &str,
    on_retry: &Option<String>,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let fg: crate::Color = scheme.on_surface.into();
    let error_color: crate::Color = scheme.error.into();
    let p = prog.to_vec();
    let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    children.push(
        Box::new(view::sized_box(crate::svg_icon::<AppStateNativo>(
            "warning",
            48.0,
            scheme.error,
            crate::IconStyle::Filled,
        ))) as Box<AnyWidgetView<AppStateNativo>>,
    );
    children.push(
        Box::new(view::label(mensaje.to_string()).text_size(16.0).color(fg))
            as Box<AnyWidgetView<AppStateNativo>>,
    );
    if let Some(cb_name) = on_retry {
        let cb = cb_name.clone();
        let p = p.clone();
        let btn = view::button(
            view::label("Reintentar")
                .text_size(14.0)
                .weight(FontWeight::MEDIUM)
                .color(error_color),
            move |data: &mut AppStateNativo| {
                if !cb.is_empty() {
                    ejecutar_callback_y_actualizar(&cb, data, &p);
                }
            },
        );
        children.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
    }
    Box::new(
        view::flex(crate::Axis::Vertical, (children,))
            .main_axis_alignment(crate::MainAxisAlignment::Center)
            .gap(Length::px(12.0)),
    )
}

// ─── Avatar ─────────────────────────────────────────────────────

pub(crate) fn render_avatar(
    texto: &str,
    variant: &AvatarVariant,
    tamaño: &f64,
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let t = *tamaño;
    let bg_color: crate::Color = scheme.primary_container.into();
    let fg_color: crate::Color = scheme.on_primary_container.into();
    let fg_color_rgb: RgbColor = scheme.on_primary_container;
    match variant {
        AvatarVariant::Text => {
            let initials: String = texto.chars().take(2).collect();
            Box::new(
                view::sized_box(
                    view::label(initials)
                        .text_size((t * 0.4) as f32)
                        .weight(FontWeight::BOLD)
                        .color(fg_color),
                )
                .dims((Length::px(t), Length::px(t)))
                .background(Background::Color(bg_color))
                .corner_radius((t / 2.0).px()),
            )
        }
        AvatarVariant::Icon => Box::new(
            view::sized_box(
                view::label(texto.to_string())
                    .text_size((t * 0.5) as f32)
                    .color(fg_color),
            )
            .dims((Length::px(t), Length::px(t)))
            .background(Background::Color(bg_color))
            .corner_radius((t / 2.0).px()),
        ),
        AvatarVariant::Image => Box::new(
            view::sized_box(crate::svg_icon::<AppStateNativo>(
                "image",
                t * 0.5,
                fg_color_rgb,
                crate::IconStyle::Filled,
            ))
            .dims((Length::px(t), Length::px(t)))
            .background(Background::Color(bg_color))
            .corner_radius((t / 2.0).px()),
        ),
    }
}

// ─── AvatarGroup ────────────────────────────────────────────────

pub(crate) fn render_avatar_group(
    avatares: &[String],
    max: &usize,
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let bg_color: crate::Color = scheme.primary_container.into();
    let fg_color: crate::Color = scheme.on_primary_container.into();
    let avatar_size = 32.0;
    let overlap = 12.0;
    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    let count = avatares.len().min(*max);
    for i in 0..count {
        let initials: String = avatares[i].chars().take(2).collect();
        let avatar = view::sized_box(
            view::label(initials)
                .text_size(12.0)
                .weight(FontWeight::BOLD)
                .color(fg_color),
        )
        .dims((Length::px(avatar_size), Length::px(avatar_size)))
        .background(Background::Color(bg_color))
        .corner_radius((avatar_size / 2.0).px())
        .border_color(scheme.surface.into())
        .border_width(2.0.px());
        widgets.push(Box::new(
            view::sized_box(avatar).width(Length::px(avatar_size + i as f64 * overlap)),
        ) as Box<AnyWidgetView<AppStateNativo>>);
    }
    if avatares.len() > *max {
        let remaining = avatares.len() - *max;
        let more = view::sized_box(
            view::label(format!("+{}", remaining))
                .text_size(11.0)
                .weight(FontWeight::BOLD)
                .color(fg_color),
        )
        .dims((Length::px(avatar_size), Length::px(avatar_size)))
        .background(Background::Color(scheme.surface_variant.into()))
        .corner_radius((avatar_size / 2.0).px())
        .border_color(scheme.surface.into())
        .border_width(2.0.px());
        widgets.push(Box::new(more) as Box<AnyWidgetView<AppStateNativo>>);
    }
    Box::new(view::flex(crate::Axis::Horizontal, (widgets,)).gap(Length::px(-overlap)))
}
