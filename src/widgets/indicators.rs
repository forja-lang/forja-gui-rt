// Material Design 3 — Indicadores: LinearProgress, CircularProgress, Badge,
// Skeleton, EmptyState, ErrorState, Avatar, AvatarGroup
// Funciones de rendering extraídas de gui_nativa.rs

use crate::view;
use crate::helpers::*;
use crate::gui_nativa::{AppStateNativo, AvatarVariant, Layout};
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme};
use crate::Length;
use forja::ast::Declaracion;
use xilem::style::Style;

// ─── LinearProgress ──────────────────────────────────────────────

pub(crate) fn render_linear_progress(
    variable: &str,
    indeterminado: &bool,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let track_color: crate::Color = scheme.surface_variant.into();
    let indicator_color: crate::Color = scheme.primary.into();
    if *indeterminado {
        Box::new(
            view::sized_box(view::zstack((
                view::sized_box(view::label(String::new()))
                    .width(Length::px(300.0))
                    .height(Length::px(4.0))
                    .background(Background::Color(track_color))
                    .corner_radius(2.0),
                view::sized_box(view::label(String::new()))
                    .width(Length::px(60.0))
                    .height(Length::px(4.0))
                    .background(Background::Color(indicator_color))
                    .corner_radius(2.0),
            )))
            .width(Length::px(300.0)),
        )
    } else {
        let valor = (data.leer(variable).to_f64() / 100.0).clamp(0.0, 1.0);
        let filled_width = 300.0 * valor;
        let empty_width = 300.0 * (1.0 - valor);
        Box::new(
            view::sized_box(view::zstack((
                view::sized_box(view::label(String::new()))
                    .width(Length::px(300.0))
                    .height(Length::px(4.0))
                    .background(Background::Color(track_color))
                    .corner_radius(2.0),
                view::flex(
                    crate::view::Axis::Horizontal,
                    (
                        view::sized_box(view::label(String::new()))
                            .width(Length::px(filled_width))
                            .height(Length::px(4.0))
                            .background(Background::Color(indicator_color))
                            .corner_radius(2.0),
                        view::sized_box(view::label(String::new()))
                            .width(Length::px(empty_width))
                            .height(Length::px(4.0)),
                    ),
                ),
            )))
            .width(Length::px(300.0)),
        )
    }
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
                .width(Length::px(300.0))
                .height(Length::px(4.0))
                .background(Background::Color(track_color))
                .corner_radius(2.0),
            view::flex(
                crate::view::Axis::Horizontal,
                (
                    view::sized_box(view::label(String::new()))
                        .width(Length::px(filled_width))
                        .height(Length::px(4.0))
                        .background(Background::Color(indicator_color))
                        .corner_radius(2.0),
                    view::sized_box(view::label(String::new()))
                        .width(Length::px(empty_width))
                        .height(Length::px(4.0)),
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
            .width(Length::px(s))
            .height(Length::px(s))
            .background(Background::Color(track_color))
            .corner_radius(s / 2.0),
        )
    } else {
        let _valor = data.leer(variable).to_f64();
        Box::new(
            view::sized_box(
                view::label(format!("{:.0}%", _valor))
                    .text_size((s * 0.3) as f32)
                    .color(indicator_color),
            )
            .width(Length::px(s))
            .height(Length::px(s))
            .border_color(indicator_color)
            .border_width(4.0)
            .corner_radius(s / 2.0)
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
            .width(Length::px(8.0))
            .height(Length::px(8.0))
            .background(Background::Color(bg_color))
            .corner_radius(4.0);
        Box::new(view::zstack((
            inner,
            Box::new(dot_w) as Box<AnyWidgetView<AppStateNativo>>,
        )))
    } else {
        let num = valor.clone().unwrap_or_default();
        let badge = view::sized_box(view::label(num).text_size(11.0).color(fg_color))
            .width(Length::px(18.0))
            .height(Length::px(18.0))
            .background(Background::Color(bg_color))
            .corner_radius(9.0);
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
        "tarjeta" => 12.0,
        _ => 4.0,
    };
    Box::new(
        view::sized_box(view::label(String::new()))
            .width(Length::px(*ancho))
            .height(Length::px(*alto))
            .background(Background::Color(sk_color))
            .corner_radius(radius),
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
    children.push(
        Box::new(view::label(mensaje.to_string()).text_size(16.0).color(fg_var))
            as Box<AnyWidgetView<AppStateNativo>>,
    );
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
        view::flex(crate::view::Axis::Vertical, (children,))
            .gap(Length::px(12.0))
            .main_axis_alignment(crate::MainAxisAlignment::Center),
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
    children
        .push(Box::new(view::label("⚠️").text_size(48.0))
            as Box<AnyWidgetView<AppStateNativo>>);
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
        view::flex(crate::view::Axis::Vertical, (children,))
            .gap(Length::px(12.0))
            .main_axis_alignment(crate::MainAxisAlignment::Center),
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
                .width(Length::px(t))
                .height(Length::px(t))
                .background(Background::Color(bg_color))
                .corner_radius(t / 2.0),
            )
        }
        AvatarVariant::Icon => Box::new(
            view::sized_box(
                view::label(texto.to_string())
                    .text_size((t * 0.5) as f32)
                    .color(fg_color),
            )
            .width(Length::px(t))
            .height(Length::px(t))
            .background(Background::Color(bg_color))
            .corner_radius(t / 2.0),
        ),
        AvatarVariant::Image => Box::new(
            view::sized_box(view::label("🖼").text_size((t * 0.5) as f32))
                .width(Length::px(t))
                .height(Length::px(t))
                .background(Background::Color(bg_color))
                .corner_radius(t / 2.0),
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
        .width(Length::px(avatar_size))
        .height(Length::px(avatar_size))
        .background(Background::Color(bg_color))
        .corner_radius(avatar_size / 2.0)
        .border_color(scheme.surface.into())
        .border_width(2.0);
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
        .width(Length::px(avatar_size))
        .height(Length::px(avatar_size))
        .background(Background::Color(scheme.surface_variant.into()))
        .corner_radius(avatar_size / 2.0)
        .border_color(scheme.surface.into())
        .border_width(2.0);
        widgets.push(Box::new(more) as Box<AnyWidgetView<AppStateNativo>>);
    }
    Box::new(view::flex(crate::view::Axis::Horizontal, (widgets,)).gap(Length::px(-overlap)))
}
