// Material Design 3 — Navegación: Navigator, NavigationBar, NavigationRail,
// NavigationDrawer, TopAppBar, BottomAppBar, Tabs, SearchBar, SearchView
// Funciones de rendering extraídas de gui_nativa.rs

use crate::view;
use crate::helpers::*;
use crate::gui_nativa::{
    AppStateNativo, IconAction, Layout, NavigatorAnim, NavigatorScreen,
    NavigatorType, NavItem, ValorGUI,
};
use crate::{AnyWidgetView, Background, FontWeight, MaterialTheme, RgbColor};
use crate::Length;
use crate::theme::motion::EASE_EMPHASIZED;
use forja::ast::Declaracion;
use xilem::style::Style;

// ─── Navigator (navegación por pantallas) ────────────────────────

pub(crate) fn render_navigator(
    screens: &[NavigatorScreen],
    current_var: &str,
    nav_type: &NavigatorType,
    anim: &NavigatorAnim,
    on_change: &Option<String>,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();

    // Leer la pantalla actual desde el store reactivo
    let current_id = data.leer(current_var).to_string();
    let current_idx = screens.iter().position(|s| s.id == current_id).unwrap_or(0);
    let idx = current_idx % screens.len();

    // Obtener la pantalla actual y renderizar su contenido
    let current_screen = &screens[idx];
    let a11y_screen_name = current_screen.titulo.clone();
    data.a11y_focus("navigation", &a11y_screen_name, "", "Pantalla activa");

    // Evaluar el contenido: si content_fn está presente, buscar la función en el AST
    let mut deferred_layout = Layout::Spacer(0.0);
    let content_layout = if let (Layout::Spacer(0.0), Some(fn_name)) = (&*current_screen.contenido, &current_screen.content_fn) {
        let mut found = false;
        for decl in prog {
            if let forja::ast::Declaracion::Funcion { nombre, cuerpo, .. } = decl {
                if nombre == fn_name {
                    if let Some(forja::ast::Declaracion::Retornar { valor }) = cuerpo.iter().find(|d| matches!(d, forja::ast::Declaracion::Retornar { .. })) {
                        if let Some(expr) = valor {
                            match crate::gui_nativa::expr_a_layout(expr) {
                                Some(layout) => {
                                    deferred_layout = layout;
                                    found = true;
                                }
                                None => {
                                    eprintln!("[Navigator] content_fn '{}' retornar expr no convertible a layout", fn_name);
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        if found { &deferred_layout } else {
            eprintln!("[Navigator] content_fn '{}' NO encontrada en AST (screens: {})", fn_name, screens.len());
            &*current_screen.contenido
        }
    } else {
        &*current_screen.contenido
    };
    let content = crate::gui_nativa::layout_a_view(content_layout, data, prog, theme);

    // Animación de transición entre pantallas
    let content = match anim {
        NavigatorAnim::None => content,
        NavigatorAnim::Fade | NavigatorAnim::Slide => {
            if current_id != data.nav_prev_screen {
                data.nav_transition_start(&current_id);
            }
            let elapsed = data.nav_anim_start
                .map(|s| s.elapsed().as_secs_f64())
                .unwrap_or(0.0);
            let duration = 0.25;
            let progress = (elapsed / duration).clamp(0.0, 1.0);
            let eased = EASE_EMPHASIZED.apply(progress);

            if eased >= 1.0 {
                content
            } else {
                let overlay_alpha = 1.0 - eased;
                let fade_color = RgbColor(0, 0, 0).with_alpha(overlay_alpha as f64 * 0.5);
                Box::new(
                    view::sized_box(content)
                        .background(Background::Color(fade_color)),
                )
            }
        }
    };

    // Pre-extraer datos de navegación
    let nav_titles: Vec<String> = screens.iter().map(|s| s.titulo.clone()).collect();
    let nav_icons: Vec<String> = screens
        .iter()
        .map(|s| s.icono.clone().unwrap_or_else(|| "•".to_string()))
        .collect();

    let cb_name = on_change.clone().unwrap_or_else(|| current_var.to_string());

    // Construir la navegación según el tipo
    match nav_type {
        NavigatorType::None => content,
        NavigatorType::BottomBar => {
            let cv = current_var.to_string();
            let cb = cb_name.clone();
            let sc = scheme.clone();
            let label_style = get_text_style(&theme.typography, "label_small");
            let mut items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for i in 0..screens.len() {
                let cv_inner = cv.clone();
                let cb_inner = cb.clone();
                let titulo = nav_titles[i].clone();
                let icono = nav_icons[i].clone();
                let sel = i == idx;
                let btn_idx = i;
                let fg_rgb: RgbColor = if sel { sc.primary } else { sc.on_surface_variant };
                let fg: crate::Color = fg_rgb.into();
                let pill_bg: crate::Color = if sel {
                    sc.secondary_container.into()
                } else {
                    crate::Color::TRANSPARENT
                };
                let icono_view = view::sized_box(crate::svg_icon::<AppStateNativo>(&icono, 24.0, fg_rgb, crate::IconStyle::Filled))
                    .padding(8.0)
                    .corner_radius(20.0)
                    .background(Background::Color(pill_bg));
                let w = view::flex(
                    crate::view::Axis::Vertical,
                    (
                        icono_view,
                        view::label(titulo)
                            .text_size(label_style.font_size as f32)
                            .weight(if sel { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                            .color(fg),
                    ),
                )
                .gap(Length::px(4.0));
                let p_clone = p.clone();
                let btn = view::button(w, move |data: &mut AppStateNativo| {
                    data.escribir(&cv_inner, ValorGUI::Entero(btn_idx as i64));
                    data.escribir("indice", ValorGUI::Entero(btn_idx as i64));
                    ejecutar_callback_y_actualizar(&cb_inner, data, &p_clone);
                });
                items.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
            }
            let bar = Box::new(
                view::flex(crate::view::Axis::Horizontal, (items,))
                    .gap(Length::px(0.0))
                    .main_axis_alignment(crate::MainAxisAlignment::SpaceEvenly)
                    .background(Background::Color(sc.surface.into()))
                    .border_color(sc.outline_variant.into())
                    .border_width(1.0)
                    .padding(0.0),
            ) as Box<AnyWidgetView<AppStateNativo>>;
            let scrollable_content = view::portal(content);
            let spacer = xilem::view::FlexExt::flex(
                view::sized_box(view::label("")),
                1.0,
            );
            let nav_overlay = Box::new(
                view::sized_box(
                    view::flex(crate::view::Axis::Vertical, (spacer, bar))
                        .must_fill_major_axis(true)
                        .cross_axis_alignment(crate::CrossAxisAlignment::Fill),
                ).height(Length::px(data.window_height))
            ) as Box<AnyWidgetView<AppStateNativo>>;
            Box::new(view::zstack((
                Box::new(scrollable_content) as Box<AnyWidgetView<AppStateNativo>>,
                nav_overlay,
            )))
        }
        NavigatorType::Tabs => {
            let cv = current_var.to_string();
            let cb = cb_name.clone();
            let sc = scheme.clone();
            let label_style = get_text_style(&theme.typography, "label_large");
            let mut items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for i in 0..screens.len() {
                let cv_inner = cv.clone();
                let cb_inner = cb.clone();
                let titulo = nav_titles[i].clone();
                let sel = i == idx;
                let btn_idx = i;
                let fg: crate::Color = if sel { sc.primary.into() } else { sc.on_surface_variant.into() };
                let tab = view::flex(
                    crate::view::Axis::Vertical,
                    (
                        view::label(titulo)
                            .text_size(label_style.font_size as f32)
                            .weight(if sel { FontWeight::BOLD } else { FontWeight::MEDIUM })
                            .color(fg),
                        if sel {
                            Box::new(
                                view::sized_box(view::label(String::new()))
                                    .height(Length::px(3.0))
                                    .background(Background::Color(sc.primary.into())),
                            ) as Box<AnyWidgetView<AppStateNativo>>
                        } else {
                            Box::new(
                                view::sized_box(view::label(String::new()))
                                    .height(Length::px(3.0)),
                            ) as Box<AnyWidgetView<AppStateNativo>>
                        },
                    ),
                )
                .gap(Length::px(4.0));
                let p_clone = p.clone();
                let btn = view::button(tab, move |data: &mut AppStateNativo| {
                    data.escribir(&cv_inner, ValorGUI::Entero(btn_idx as i64));
                    data.escribir("indice", ValorGUI::Entero(btn_idx as i64));
                    ejecutar_callback_y_actualizar(&cb_inner, data, &p_clone);
                });
                items.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
            }
            let tabs = Box::new(
                view::flex(crate::view::Axis::Horizontal, (items,))
                    .gap(Length::px(0.0))
                    .main_axis_alignment(crate::MainAxisAlignment::SpaceEvenly)
                    .background(Background::Color(sc.surface.into()))
                    .border_color(sc.outline_variant.into())
                    .border_width(1.0)
                    .padding(0.0),
            ) as Box<AnyWidgetView<AppStateNativo>>;
            let scrollable_content = view::portal(content);
            let spacer = xilem::view::FlexExt::flex(
                view::sized_box(view::label("")),
                1.0,
            );
            let nav_overlay = Box::new(
                view::sized_box(
                    view::flex(crate::view::Axis::Vertical, (tabs, spacer))
                        .must_fill_major_axis(true)
                        .cross_axis_alignment(crate::CrossAxisAlignment::Fill),
                ).height(Length::px(data.window_height))
            ) as Box<AnyWidgetView<AppStateNativo>>;
            Box::new(view::zstack((
                Box::new(scrollable_content) as Box<AnyWidgetView<AppStateNativo>>,
                nav_overlay,
            )))
        }
        NavigatorType::Rail | NavigatorType::Drawer => {
            let cv = current_var.to_string();
            let cb = cb_name.clone();
            let sc = scheme.clone();
            let mut items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for i in 0..screens.len() {
                let cv_inner = cv.clone();
                let cb_inner = cb.clone();
                let titulo = nav_titles[i].clone();
                let icono = nav_icons[i].clone();
                let sel = i == idx;
                let btn_idx = i;
                let fg_rgb: RgbColor = if sel { sc.on_secondary_container } else { sc.on_surface_variant };
                let fg: crate::Color = fg_rgb.into();
                let bg: crate::Color = if sel {
                    sc.secondary_container.into()
                } else {
                    crate::Color::TRANSPARENT
                };
                let w = view::flex(
                    crate::view::Axis::Vertical,
                    (
                        crate::svg_icon::<AppStateNativo>(&icono, 24.0, fg_rgb, crate::IconStyle::Filled),
                        view::label(titulo).text_size(10.0).color(fg),
                    ),
                )
                .gap(Length::px(2.0));
                let p_clone = p.clone();
                let btn = view::button(w, move |data: &mut AppStateNativo| {
                    data.escribir(&cv_inner, ValorGUI::Entero(btn_idx as i64));
                    data.escribir("indice", ValorGUI::Entero(btn_idx as i64));
                    ejecutar_callback_y_actualizar(&cb_inner, data, &p_clone);
                });
                items.push(Box::new(
                    view::sized_box(btn)
                        .background(Background::Color(bg))
                        .corner_radius(16.0),
                ) as Box<AnyWidgetView<AppStateNativo>>);
            }
            let rail = Box::new(
                view::flex(crate::view::Axis::Vertical, (items,))
                    .gap(Length::px(4.0))
                    .background(Background::Color(sc.surface.into())),
            );
            let content_flex = xilem::view::FlexExt::flex(content, 1.0);
            Box::new(
                view::flex(crate::view::Axis::Horizontal, (rail, content_flex))
                    .must_fill_major_axis(true),
            )
        }
    }
}

// ─── NavigationBar ──────────────────────────────────────────────

pub(crate) fn render_navigation_bar(
    items: &[NavItem],
    seleccion: &usize,
    on_change: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();
    let cb = on_change.to_string();

    let mut nav_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let cb_inner = cb.clone();
        let p_inner = p.clone();
        let idx = i;
        let is_selected = i == *seleccion;

        let fg_color: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.on_surface_variant.into()
        };

        let label_style = get_text_style(&theme.typography, "label_small");
        let item_widget = view::flex(
            crate::view::Axis::Vertical,
            (
                view::label(item.icono.clone())
                    .text_size(24.0)
                    .color(fg_color),
                view::label(item.label.clone())
                    .text_size(label_style.font_size as f32)
                    .weight(if is_selected { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                    .color(fg_color),
            ),
        )
        .gap(Length::px(2.0));

        let btn = view::button(item_widget, move |data: &mut AppStateNativo| {
            data.escribir(&cb_inner, ValorGUI::Entero(idx as i64));
            ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
        });

        nav_items.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    Box::new(
        view::flex(crate::view::Axis::Horizontal, (nav_items,))
            .gap(Length::px(0.0))
            .background(Background::Color(scheme.surface.into())),
    )
}

// ─── NavigationRail ─────────────────────────────────────────────

pub(crate) fn render_navigation_rail(
    items: &[NavItem],
    seleccion: &usize,
    on_change: &str,
    extended: &bool,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();
    let cb = on_change.to_string();

    let mut nav_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let cb_inner = cb.clone();
        let p_inner = p.clone();
        let idx = i;
        let is_selected = i == *seleccion;

        let fg_color: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.on_surface_variant.into()
        };
        let bg_color: crate::Color = if is_selected {
            scheme.primary_container.into()
        } else {
            crate::Color::TRANSPARENT
        };

        let icon = view::label(item.icono.clone())
            .text_size(24.0)
            .color(fg_color);

        let content: Box<AnyWidgetView<AppStateNativo>> = if *extended {
            let label_style = get_text_style(&theme.typography, "label_medium");
            Box::new(
                view::flex(
                    crate::view::Axis::Horizontal,
                    (
                        icon,
                        view::label(item.label.clone())
                            .text_size(label_style.font_size as f32)
                            .color(fg_color),
                    ),
                )
                .gap(Length::px(8.0)),
            ) as Box<AnyWidgetView<AppStateNativo>>
        } else {
            Box::new(
                view::flex(
                    crate::view::Axis::Vertical,
                    (
                        icon,
                        view::label(item.label.clone())
                            .text_size(10.0)
                            .color(fg_color),
                    ),
                )
                .gap(Length::px(2.0)),
            ) as Box<AnyWidgetView<AppStateNativo>>
        };

        let btn = view::button(content, move |data: &mut AppStateNativo| {
            data.escribir(&cb_inner, ValorGUI::Entero(idx as i64));
            ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
        });

        let styled_btn = view::sized_box(btn)
            .background(Background::Color(bg_color))
            .corner_radius(16.0);

        nav_items.push(Box::new(styled_btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let axis = if *extended {
        crate::view::Axis::Horizontal
    } else {
        crate::view::Axis::Vertical
    };
    Box::new(
        view::flex(axis, (nav_items,))
            .gap(Length::px(4.0))
            .background(Background::Color(scheme.surface.into())),
    )
}

// ─── NavigationDrawer ───────────────────────────────────────────

pub(crate) fn render_navigation_drawer(
    items: &[NavItem],
    seleccion: &usize,
    on_change: &str,
    modal: &bool,
    visible: &str,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();
    let cb = on_change.to_string();

    if *modal {
        let show = data.leer(visible).to_string() == "true";
        if !show {
            return Box::new(view::sized_box(view::label(String::new())));
        }
    }

    let mut nav_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let cb_inner = cb.clone();
        let p_inner = p.clone();
        let idx = i;
        let is_selected = i == *seleccion;

        let fg_color: crate::Color = if is_selected {
            scheme.on_secondary_container.into()
        } else {
            scheme.on_surface_variant.into()
        };
        let bg_color: crate::Color = if is_selected {
            scheme.secondary_container.into()
        } else {
            crate::Color::TRANSPARENT
        };

        let label_style = get_text_style(&theme.typography, "label_large");
        let item_content = view::flex(
            crate::view::Axis::Horizontal,
            (
                view::label(item.icono.clone())
                    .text_size(24.0)
                    .color(fg_color),
                view::label(item.label.clone())
                    .text_size(label_style.font_size as f32)
                    .color(fg_color),
            ),
        )
        .gap(Length::px(16.0));

        let btn = view::button(item_content, move |data: &mut AppStateNativo| {
            data.escribir(&cb_inner, ValorGUI::Entero(idx as i64));
            ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
        });

        let styled_item = view::sized_box(btn)
            .background(Background::Color(bg_color))
            .corner_radius(12.0);

        nav_items.push(Box::new(styled_item) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let drawer =
        view::sized_box(view::flex(crate::view::Axis::Vertical, (nav_items,)).gap(Length::px(4.0)))
            .background(Background::Color(scheme.surface.into()))
            .corner_radius(16.0);

    if *modal {
        let overlay_color: crate::Color = RgbColor(0, 0, 0).with_alpha(0.32);
        Box::new(view::zstack((
            view::sized_box(view::label(String::new()))
                .background(Background::Color(overlay_color)),
            Box::new(drawer) as Box<AnyWidgetView<AppStateNativo>>,
        )))
    } else {
        Box::new(drawer)
    }
}

// ─── TopAppBar ──────────────────────────────────────────────────

pub(crate) fn render_top_app_bar(
    titulo: &str,
    acciones: &[IconAction],
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();

    let title_size = 22.0;
    let fg_title: crate::Color = scheme.on_surface.into();

    let title_label = view::label(titulo.to_string())
        .text_size(title_size as f32)
        .weight(FontWeight::BOLD)
        .color(fg_title);

    let mut action_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for action in acciones.iter() {
        let cb_inner = action.callback.clone();
        let p_inner = p.clone();
        let icon_fg_rgb: RgbColor = scheme.on_surface_variant;
        let icon_view = crate::svg_icon::<AppStateNativo>(&action.icono, 24.0, icon_fg_rgb, crate::IconStyle::Filled);
        let icon_btn = view::button(
            icon_view,
            move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
            },
        );
        action_widgets.push(Box::new(icon_btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let bar = view::flex(
        crate::view::Axis::Horizontal,
        (
            title_label,
            view::flex(crate::view::Axis::Horizontal, (action_widgets,)).gap(Length::px(4.0)),
        ),
    )
    .gap(Length::px(8.0));

    Box::new(
        view::sized_box(bar)
            .background(Background::Color(scheme.surface.into()))
            .padding(16.0),
    )
}

// ─── BottomAppBar ───────────────────────────────────────────────

pub(crate) fn render_bottom_app_bar(
    acciones: &[IconAction],
    fab: &Option<Box<Layout>>,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();

    let mut action_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for action in acciones.iter() {
        let cb_inner = action.callback.clone();
        let p_inner = p.clone();
        let icon_fg_rgb: RgbColor = scheme.on_surface_variant;
        let icon_view = crate::svg_icon::<AppStateNativo>(&action.icono, 24.0, icon_fg_rgb, crate::IconStyle::Filled);
        let icon_btn = view::button(
            icon_view,
            move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
            },
        );
        action_widgets.push(Box::new(icon_btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    children.push(Box::new(
        view::flex(crate::view::Axis::Horizontal, (action_widgets,)).gap(Length::px(8.0)),
    ) as Box<AnyWidgetView<AppStateNativo>>);

    if let Some(f) = fab {
        children.push(crate::gui_nativa::layout_a_view(f, data, prog, theme));
    }

    Box::new(
        view::sized_box(view::flex(crate::view::Axis::Horizontal, (children,)).gap(Length::px(16.0)))
            .background(Background::Color(scheme.surface.into()))
            .padding(8.0),
    )
}

// ─── Tabs ───────────────────────────────────────────────────────

pub(crate) fn render_tabs(
    tabs: &[String],
    seleccion: &usize,
    on_change: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let p = prog.to_vec();
    let cb = on_change.to_string();

    let mut tab_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, tab) in tabs.iter().enumerate() {
        let cb_inner = cb.clone();
        let p_inner = p.clone();
        let idx = i;
        let t = tab.clone();
        let is_selected = i == *seleccion;

        let fg_color: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.on_surface_variant.into()
        };

        let label_style = get_text_style(&theme.typography, "label_large");
        let tab_content = view::flex(
            crate::view::Axis::Vertical,
            (
                view::label(t.clone())
                    .text_size(label_style.font_size as f32)
                    .weight(if is_selected { FontWeight::BOLD } else { FontWeight::MEDIUM })
                    .color(fg_color),
                if is_selected {
                    Box::new(
                        view::sized_box(view::label(String::new()))
                            .height(Length::px(3.0))
                            .background(Background::Color(scheme.primary.into())),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                } else {
                    Box::new(
                        view::sized_box(view::label(String::new())).height(Length::px(3.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                },
            ),
        )
        .gap(Length::px(4.0));

        let btn = view::button(tab_content, move |data: &mut AppStateNativo| {
            data.escribir(&cb_inner, ValorGUI::Entero(idx as i64));
            ejecutar_callback_y_actualizar(&cb_inner, data, &p_inner);
        });

        tab_widgets.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    Box::new(view::flex(crate::view::Axis::Horizontal, (tab_widgets,)).gap(Length::px(0.0)))
}

// ─── SearchBar ──────────────────────────────────────────────────

pub(crate) fn render_search_bar(
    placeholder: &str,
    variable: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let ph = placeholder.to_string();

    let bg: crate::Color = scheme.surface_variant.into();
    let icon_fg: crate::Color = scheme.on_surface_variant.into();

    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, ValorGUI::Texto(new_val));
    })
    .placeholder(ph.as_str());

    Box::new(
        view::flex(
            crate::view::Axis::Horizontal,
            (view::label("🔍 ").text_size(18.0).color(icon_fg), ti),
        )
        .gap(Length::px(8.0))
        .background(Background::Color(bg))
        .corner_radius(24.0)
        .padding(12.0),
    )
}

// ─── SearchView ─────────────────────────────────────────────────

pub(crate) fn render_search_view(
    resultados: &[Layout],
    visible: &str,
    data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let show = data.leer(visible).to_string() == "true";
    if show {
        let mut result_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
        for r in resultados.iter() {
            result_widgets.push(crate::gui_nativa::layout_a_view(r, data, prog, theme));
        }
        Box::new(
            view::sized_box(
                view::flex(crate::view::Axis::Vertical, (result_widgets,)).gap(Length::px(8.0)),
            )
            .background(Background::Color(scheme.surface.into()))
            .corner_radius(16.0),
        )
    } else {
        Box::new(view::sized_box(view::label(String::new())))
    }
}
