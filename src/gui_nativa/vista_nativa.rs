//! Layouts → widgets de xilem (`layout_a_vista`).
//!
//! Traduce el árbol de [`Layout`] a vistas de xilem con type erasure
//! (`AnyWidgetView`), aplicando el tema Material You.

use super::*;
use crate::view;
use crate::*;

/// Convierte Layout a xilem usando AnyWidgetView para type erasure.
/// Recibe el tema actual como referencia para aplicar colores, tipografía y formas.
pub fn layout_a_vista<'a>(
    layout: &'a Layout,
    data: &'a mut AppStateNativo,
    _prog: &'a [Declaracion],
    theme: &'a MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    if crate::perf::activo() {
        crate::perf::contar_nodo();
    }
    match layout {
        // ─── Layouts existentes (compatibilidad hacia atrás) ──────
        Layout::Column {
            children,
            gap,
            alignment,
        } => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            let ma = parsear_alignment(alignment);
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.ventana_ancho - 16.0).max(200.0),
                Orientation::Landscape => (data.ventana_ancho - 16.0).clamp(200.0, 720.0),
            };
            Box::new(
                view::sized_box(
                    view::flex(Axis::Vertical, (widgets,))
                        .main_axis_alignment(ma)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(Length::px(*gap)),
                )
                .width(Length::px(ancho))
                .padding(16.0.px()),
            )
        }
        Layout::CenteredColumn(hijos) => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in hijos {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            // Ancho dinámico: en portrait usa el ancho completo, en landscape limita a 720dp máximo
            let ancho = match data.orientation {
                Orientation::Portrait => (data.ventana_ancho - 16.0).max(200.0),
                Orientation::Landscape => (data.ventana_ancho - 16.0).clamp(200.0, 720.0),
            };
            Box::new(
                view::sized_box(
                    view::flex(Axis::Vertical, (widgets,))
                        .main_axis_alignment(MainAxisAlignment::Center)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .dims(xilem::masonry::properties::Dimensions::height(
                            xilem::masonry::layout::Dim::Stretch,
                        )),
                )
                .width(Length::px(ancho))
                .padding(16.0.px()),
            )
        }
        Layout::Row {
            children,
            gap,
            alignment,
        } => {
            // 5.9: Fila con soporte de Flex para tarjetas y list items.
            // NOTA: FlexExt::flex() no funciona con Box<AnyWidgetView> (type erasure),
            // ya que Pod<DynWidget> no implementa AnyElement<FlexElement>.
            // Se maneja con tuplas de tamaño fijo para los casos más comunes (1-3 hijos).
            let ma = parsear_alignment(alignment);
            let flex_eligible = children.iter().all(|h| {
                matches!(
                    h,
                    Layout::MaterialCard { .. } | Layout::ElementoListaMaterial { .. }
                )
            });
            if flex_eligible && !children.is_empty() && children.len() <= 3 {
                // Casos con 1-3 hijos flex: usar tuplas con FlexExt
                match children.len() {
                    1 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let row =
                            view::flex(Axis::Horizontal, (xilem::view::FlexExt::flex(a, 1.0),))
                                .main_axis_alignment(ma)
                                .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    2 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let b = layout_a_vista(&children[1], data, _prog, theme);
                        let row = view::flex(
                            Axis::Horizontal,
                            (
                                xilem::view::FlexExt::flex(a, 1.0),
                                xilem::view::FlexExt::flex(b, 1.0),
                            ),
                        )
                        .main_axis_alignment(ma)
                        .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    3 => {
                        let a = layout_a_vista(&children[0], data, _prog, theme);
                        let b = layout_a_vista(&children[1], data, _prog, theme);
                        let c = layout_a_vista(&children[2], data, _prog, theme);
                        let row = view::flex(
                            Axis::Horizontal,
                            (
                                xilem::view::FlexExt::flex(a, 1.0),
                                xilem::view::FlexExt::flex(b, 1.0),
                                xilem::view::FlexExt::flex(c, 1.0),
                            ),
                        )
                        .main_axis_alignment(ma)
                        .gap(Length::px(*gap));
                        return Box::new(row) as Box<AnyWidgetView<AppStateNativo>>;
                    }
                    _ => {}
                }
            }
            // Caso general: sin flex
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            Box::new(
                view::flex(Axis::Horizontal, (widgets,))
                    .main_axis_alignment(ma)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(Length::px(*gap)),
            )
        }
        Layout::PilaZ(hijos) => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in hijos {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            Box::new(view::zstack((widgets,)))
        }
        Layout::Portal(hijo) => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // El área desplazable ocupa el ancho de la ventana (menos un margen
            // para el scrollbar). El contenido interno se estira con cross Fill.
            let ancho = (data.ventana_ancho - 18.0).max(200.0);
            Box::new(view::portal(
                view::sized_box(inner)
                    .width(Length::px(ancho))
                    .padding(16.0.px()),
            ))
        }
        Layout::Container { hijo, max_ancho } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).width(Length::px(*max_ancho)))
        }
        Layout::Label { texto, es_variable } => {
            if *es_variable {
                let var_name = texto.clone();
                let txt = data.leer(&var_name).to_string();
                let gen = data.almacen.generation(&var_name);
                let color: forja_gui_rt::Color =
                    obtener_color_role(&theme.scheme, "on_surface").into();
                Box::new(memoize(
                    (gen, txt, color),
                    move |(_, text, clr): &(u64, String, forja_gui_rt::Color)| {
                        view::label(text.clone()).color(*clr)
                    },
                ))
            } else {
                let txt = texto.clone();
                let color = color_segun_monto(&theme.scheme, &txt);
                Box::new(view::label(txt).color(color))
            }
        }
        Layout::Title(texto) => {
            // Usar estilo headline_medium del tema
            let style = obtener_texto_style(&theme.typography, "headline_medium");
            let color: forja_gui_rt::Color = obtener_color_role(&theme.scheme, "on_surface").into();
            Box::new(
                view::label(texto.clone())
                    .text_size(style.font_tamano as f32)
                    .weight(FontWeight::BOLD)
                    .color(color),
            )
        }
        Layout::ColoredLabel { texto, color } => {
            // Roles del tema primero (primary, error, on_surface_variant, ...):
            // antes caía en blanco fijo y el texto desaparecía en modo claro.
            let c: Color = crate::theme_props::color_de_rol(theme, color)
                .map(Into::into)
                .or_else(|| color_desde_nombre(color))
                .unwrap_or_else(|| theme.scheme.on_surface.into());
            Box::new(view::label(texto.clone()).color(c))
        }
        Layout::VariableLabel { variable } => {
            let var_name = variable.clone();
            // Leer valor Y generación para memoize reactivo
            let txt = data.leer(&var_name).to_string();
            let gen = data.almacen.generation(&var_name);
            // Usar memoize para que este widget solo se reconstruya
            // cuando la generación de la variable cambie
            Box::new(memoize((gen, txt), move |(_, text): &(u64, String)| {
                view::variable_label(text.clone())
            }))
        }
        Layout::Button { texto, callback } => {
            let cb = callback.clone();
            let t = texto.clone();
            let prog = _prog.to_vec();
            Box::new(view::text_button(t, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb, data, &prog);
            }))
            // Nota: xilem 0.4 text_button no expone color de fondo directamente;
            // el color primario se aplica donde la API lo permita
        }
        Layout::EntradaTexto {
            variable,
            multiline: _,
            placeholder,
        } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_string();
            let gen = data.almacen.generation(&var_name);
            let ph = placeholder.clone();
            let tc: forja_gui_rt::Color = theme.scheme.on_surface.into();
            Box::new(memoize(
                (gen, val.clone(), var_name.clone(), ph.clone(), tc),
                move |(_, text, vn, pl, tc): &(
                    u64,
                    String,
                    String,
                    String,
                    forja_gui_rt::Color,
                )| {
                    let vn = vn.clone();
                    let pl = pl.clone();
                    let mut ti = view::text_input(
                        text.clone(),
                        move |data: &mut AppStateNativo, nuevo_val: String| {
                            data.escribir(&vn, ValorGUI::Texto(nuevo_val));
                        },
                    )
                    .text_color(*tc);
                    if !pl.is_empty() {
                        ti = ti.placeholder(pl.as_str());
                    }
                    map_message(
                        ti,
                        |_data: &mut AppStateNativo, resultado: MessageResult<()>| match resultado {
                            MessageResult::Action(()) => MessageResult::Nop,
                            other => other,
                        },
                    )
                },
            ))
        }
        Layout::ProgressBar { variable } => {
            let var_name = variable.clone();
            let val_str = data.leer(&var_name).to_string();
            let num: f64 = val_str.parse().unwrap_or(0.0);
            let gen = data.almacen.generation(&var_name);
            Box::new(memoize((gen, num), move |(_, n): &(u64, f64)| {
                view::progress_bar(Some(*n))
            }))
        }
        Layout::Slider { variable, min, max } => {
            let var_name = variable.clone();
            let val = data.leer(&var_name).to_f64();
            let gen = data.almacen.generation(&var_name);
            let mn = *min;
            let mx = *max;
            if depuracion_activo() {
                eprintln!(
                    "[slider] {}: valor={} rango={}..{} gen={}",
                    var_name, val, mn, mx, gen
                );
            }
            Box::new(memoize(
                (gen, val, mn, mx, var_name.clone()),
                move |(_, v, mn2, mx2, vn): &(u64, f64, f64, f64, String)| {
                    let vn = vn.clone();
                    view::slider(
                        *mn2,
                        *mx2,
                        *v,
                        move |data: &mut AppStateNativo, nuevo_val: f64| {
                            data.escribir(&vn, ValorGUI::Decimal(nuevo_val));
                        },
                    )
                },
            ))
        }
        Layout::Checkbox { variable } => {
            let var_name = variable.clone();
            let txt = variable.clone();
            let checked = data.leer(&var_name).to_bool();
            let gen = data.almacen.generation(&var_name);
            Box::new(memoize(
                (gen, checked, txt, var_name.clone()),
                move |(_, chk, label_txt, vn): &(u64, bool, String, String)| {
                    let vn = vn.clone();
                    let casilla = view::checkbox(
                        label_txt.clone(),
                        *chk,
                        move |data: &mut AppStateNativo, new_checked: bool| {
                            data.escribir(&vn, ValorGUI::Booleano(new_checked));
                        },
                    );
                    // En una columna el checkbox se estira a todo el ancho y el
                    // borde del widget cruza la pantalla; en una fila toma su
                    // ancho natural (caja + etiqueta).
                    view::flex(Axis::Horizontal, (casilla,))
                        .main_axis_alignment(MainAxisAlignment::Start)
                        .cross_axis_alignment(CrossAxisAlignment::Start)
                },
            ))
        }
        Layout::Prose(texto) => Box::new(view::prose(texto.clone())),
        // El `spinner` de xilem main pide todo el espacio disponible cuando se le
        // mide con `LenReq::FitContent`, así que hay que acotarlo a mano: el
        // indicador Material mide 24 dp (antes, en 0.4, el widget se achicaba solo).
        Layout::Spinner => {
            Box::new(view::sized_box(view::spinner()).dims((Length::px(24.0), Length::px(24.0))))
        }
        Layout::Separator => {
            let color: forja_gui_rt::Color = obtener_color_role(&theme.scheme, "outline").into();
            // `Dim::Stretch` + alto fijo: una línea de 1 px a todo el ancho. Un
            // `.height(..)` suelto dejaría el ancho en `auto` (0 px) y no se vería.
            Box::new(
                view::sized_box(view::label(String::new()))
                    .dims((crate::layout::Dim::Stretch, Length::px(1.0)))
                    .background(Background::Color(color)),
            )
        }
        Layout::Spacer(tamano) => {
            let t = *tamano;
            Box::new(
                view::sized_box(view::label(String::new())).dims((Length::px(t), Length::px(t))),
            )
        }

        // ─── Nuevos: variantes de tema Material You ───────────────

        // ThemeProvider: cambia el tema para los hijos
        // Acepta color semilla (#RRGGBB) o diseño nombrado ("fluent", "liquid_glass", con
        // sufijo ":dark"/"oscuro" para forzar modo oscuro)
        Layout::ThemeProvider { hijo, theme: seed } => {
            let nuevo_theme = MaterialTheme::named(seed, theme.es_dark);
            let inner = layout_a_vista(hijo, data, _prog, &nuevo_theme);
            // Cada diseño tiene su propio fondo (Mica en Fluent, vidrio en Liquid
            // Glass): lo pintamos acá para que se vea también si el wrapper no es
            // el nodo raíz.
            let bg: Color = nuevo_theme.scheme.background.into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // ColoredBox: aplica un color del esquema como fondo
        Layout::ColoredBox { hijo, color_role } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let bg: Color = obtener_color_role(&theme.scheme, color_role).into();
            Box::new(view::sized_box(inner).background(Background::Color(bg)))
        }

        // StyledLabel: etiqueta con estilo tipográfico predefinido
        Layout::StyledLabel { texto, style } => {
            let texto_style = obtener_texto_style(&theme.typography, style);
            let color = color_segun_monto(&theme.scheme, texto);
            Box::new(
                view::label(texto.clone())
                    .text_size(texto_style.font_tamano as f32)
                    .weight(texto_style.weight.to_xilem_weight())
                    .color(color),
            )
        }

        // DynamicText: texto evaluado en tiempo de render (concatenación,
        // etiqueta_dinamica, helpers del AST) con estilo tipográfico.
        Layout::DynamicText {
            expr,
            params,
            style,
        } => {
            let texto_style = obtener_texto_style(&theme.typography, style);
            let txt = expr_a_texto(expr, params, &data.almacen, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.almacen.global_generation();
            let fs = texto_style.font_tamano as f32;
            let wt = texto_style.weight.to_xilem_weight();
            Box::new(memoize(
                (gen, txt, color, fs, wt),
                move |(_, t, clr, size, w): &(
                    u64,
                    String,
                    forja_gui_rt::Color,
                    f32,
                    xilem::FontWeight,
                )| {
                    view::label(t.clone())
                        .text_size(*size)
                        .weight(*w)
                        .color(*clr)
                },
            ))
        }

        // DynamicLabel: etiqueta dinámica sin estilo tipográfico.
        Layout::DynamicLabel { expr, params } => {
            let txt = expr_a_texto(expr, params, &data.almacen, _prog);
            let color = color_segun_monto(&theme.scheme, &txt);
            let gen = data.almacen.global_generation();
            Box::new(memoize(
                (gen, txt, color),
                move |(_, t, clr): &(u64, String, forja_gui_rt::Color)| {
                    view::label(t.clone()).color(*clr)
                },
            ))
        }

        // ShapedBox: aplica border-radius al hijo
        Layout::ShapedBox { hijo, forma_family } => {
            let radius = obtener_forma_radius(&theme.shapes, forma_family);
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).corner_radius(radius.px()))
        }

        // 5.5: ElevatedBox — aplica corner_radius como indicador visual de elevación.
        // xilem 0.4: BoxShadow no está disponible en sized_box; se documenta la restricción.
        Layout::ElevatedBox {
            hijo,
            level: _level,
            forma_family,
        } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let forma_radius = obtener_forma_radius(&theme.shapes, forma_family);
            // NOTA: shadow_to_box_shadow(&shadow_cfg) calcula la sombra correctamente
            // pero xilem 0.4 no expone box-shadow en WidgetPod. Aplicar solo corner_radius.
            Box::new(view::sized_box(inner).corner_radius(forma_radius.px()))
        }

        // 5.6: StateLayerBox — overlay semi-transparente según el estado visual
        Layout::StateLayerBox { hijo, state } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let overlay_alpha: f64 = match state.as_str() {
                "hover" => 0.08,
                "pressed" => 0.12,
                "focused" => 0.12,
                "disabled" => 0.38,
                _ => 0.0,
            };
            if overlay_alpha > 0.0 {
                let overlay_color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                Box::new(view::zstack((Box::new(
                    view::sized_box(inner).background(Background::Color(overlay_color)),
                )
                    as Box<AnyWidgetView<AppStateNativo>>,)))
            } else {
                inner
            }
        }

        // ResponsiveLayout: 3 variantes según WindowSizeClass
        Layout::ResponsiveLayout {
            compact,
            medium,
            expanded,
        } => match data.ventana_tamano {
            VentanaTamanoClase::Expanded => layout_a_vista(expanded, data, _prog, theme),
            VentanaTamanoClase::Medium => layout_a_vista(medium, data, _prog, theme),
            VentanaTamanoClase::Compact => layout_a_vista(compact, data, _prog, theme),
        },

        // Padding: añade espacio alrededor del hijo
        Layout::Padding { hijo, amount } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(view::sized_box(inner).padding((*amount).px()))
        }

        // Expanded: llena el espacio disponible
        Layout::Expanded { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Envolver en flex vertical con must_fill para expandir el espacio disponible
            Box::new(view::flex(Axis::Vertical, (inner,)).dims(
                xilem::masonry::properties::Dimensions::height(
                    xilem::masonry::layout::Dim::Stretch,
                ),
            ))
        }

        // Centered: centra el hijo en el eje transversal
        Layout::Centered { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let widgets = vec![inner];
            Box::new(
                view::flex(Axis::Vertical, (widgets,))
                    .main_axis_alignment(MainAxisAlignment::Center)
                    .dims(xilem::masonry::properties::Dimensions::height(
                        xilem::masonry::layout::Dim::Stretch,
                    )),
            )
        }

        // ─── Layout responsive avanzado ────────────────────────

        // FlexLayout: flex con axis, gap y wrap configurables
        Layout::FlexLayout {
            children,
            axis,
            gap,
            envolver: _,
        } => {
            let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for h in children {
                widgets.push(layout_a_vista(h, data, _prog, theme));
            }
            let ax = match axis.to_lowercase().as_str() {
                "horizontal" | "h" | "fila" => Axis::Horizontal,
                _ => Axis::Vertical,
            };
            // Nota: xilem 0.4 no soporta wrap nativamente; se implementará
            // cuando la API lo exponga. Por ahora se usa flex con gap.
            Box::new(view::flex(ax, (widgets,)).gap(Length::px(*gap)))
        }

        // 5.11: Grid dinámico — widget grid real de Masonry.
        // Columnas FRACTION: ocupan todo el ancho disponible (responsivo al resize).
        // Filas Auto: cada fila toma la altura de su contenido (autosize).
        Layout::Grid {
            children,
            columnas,
            gap,
        } => {
            if children.is_empty() {
                return Box::new(view::sized_box(view::label("")).height(Length::px(0.0)));
            }
            // Determinar columnas según orientación y tamaño
            let effective_cols = match data.orientation {
                Orientation::Portrait => {
                    if data.ventana_ancho < 400.0 {
                        1
                    } else if data.ventana_ancho < 600.0 {
                        (*columnas).min(2)
                    } else {
                        *columnas
                    }
                }
                Orientation::Landscape => *columnas,
            }
            .max(1);
            let n_rows = children.len().div_ceil(effective_cols);

            let mut items = Vec::with_capacity(children.len());
            for (i, item) in children.iter().enumerate() {
                let widget = layout_a_vista(item, data, _prog, theme);
                items.push(view::grid_item(
                    widget,
                    GridParams::pos((i % effective_cols) as u16, (i / effective_cols) as u16),
                ));
            }

            Box::new(
                view::grid(items)
                    .columns(view::repeat_tracks(
                        effective_cols,
                        view::GridTrackSize::FRACTION,
                    ))
                    .rows(view::repeat_tracks(n_rows, view::GridTrackSize::Auto))
                    .gap(Length::px(*gap)),
            )
        }

        // 5.10: FlowLayout con wrap real — divide hijos en filas según ancho disponible
        Layout::FlowLayout { children, gap } => {
            if children.is_empty() {
                return Box::new(view::sized_box(view::label("")).height(Length::px(0.0)));
            }
            // Estimar anchos de cada hijo y renderizarlos
            let rendered: Vec<(Box<AnyWidgetView<AppStateNativo>>, f64)> = children
                .iter()
                .map(|h| {
                    let w = layout_a_vista(h, data, _prog, theme);
                    let hijo_ancho = estimate_hijo_ancho(h).unwrap_or(100.0);
                    (w, hijo_ancho)
                })
                .collect();

            let available_ancho = (data.ventana_ancho - 48.0).max(200.0);
            let mut rows: Vec<Vec<Box<AnyWidgetView<AppStateNativo>>>> = Vec::new();
            let mut actual_row: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            let mut actual_ancho = 0.0;

            for (widget, width) in rendered {
                if actual_row.is_empty() || actual_ancho + width + *gap <= available_ancho {
                    actual_row.push(widget);
                    actual_ancho += width + *gap;
                } else {
                    rows.push(actual_row);
                    actual_row = vec![widget];
                    actual_ancho = width;
                }
            }
            if !actual_row.is_empty() {
                rows.push(actual_row);
            }

            let row_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = rows
                .into_iter()
                .map(|row| {
                    Box::new(
                        view::flex(Axis::Horizontal, (row,))
                            .cross_axis_alignment(CrossAxisAlignment::Start)
                            .gap(Length::px(*gap)),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();

            Box::new(view::flex(Axis::Vertical, (row_widgets,)).gap(Length::px(*gap)))
        }

        // ─── Botones Material Design 3 ──────────────────────────────
        // NOTA: En xilem 0.4, Button no soporta ContentColor ni FontSize directamente.
        // Usamos view::button(view::label(...).color(...).text_size(...), callback)
        // para poder estilizar el texto, y .background()/.corner_radius()/.border_color()
        // en el Button wrapper para el contenedor.

        // ─── MaterialButton (5 variantes) ───────────────────────────
        Layout::MaterialButton {
            texto,
            callback,
            variant,
            icono: _,
            disabled: _,
        } => {
            let scheme = &theme.scheme;
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_large");
            match variant {
                ButtonVariant::Filled => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_primary.into(),
                        bg: Some(scheme.primary.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Tonal => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_secondary_container.into(),
                        bg: Some(scheme.secondary_container.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Outlined => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: None,
                        border: Some(scheme.outline.into()),
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Text => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: None,
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
                ButtonVariant::Elevated => crear_material_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.primary.into(),
                        bg: Some(scheme.surface.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    20.0,
                ),
            }
        }

        // ─── FAB (Floating Action Button) ───────────────────────────
        Layout::FAB {
            icono,
            callback,
            tamano,
            texto_extendido,
        } => crate::widgets::material_buttons::render_fab(
            icono,
            callback,
            tamano,
            texto_extendido,
            data,
            _prog,
            theme,
        ),

        // ─── IconButton (4 variantes) ──────────────────────────────
        Layout::IconButton {
            icono,
            callback,
            variant,
            seleccionado: _,
        } => crate::widgets::material_buttons::render_icon_button(
            icono, callback, variant, data, _prog, theme,
        ),

        // ─── SegmentedButton ────────────────────────────────────────
        Layout::SegmentedButton {
            opciones,
            seleccionados,
            callback,
            multiple: _,
        } => crate::widgets::material_buttons::render_segmented_button(
            opciones,
            seleccionados,
            callback,
            data,
            _prog,
            theme,
        ),

        // ─── Chip (4 variantes) ────────────────────────────────────
        Layout::Chip {
            texto,
            callback,
            variant,
            activo,
            on_quitar: _,
        } => {
            let scheme = &theme.scheme;
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_small");
            match variant {
                ChipVariant::Assist | ChipVariant::Suggestion => crear_chip_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_surface.into(),
                        bg: None,
                        border: Some(scheme.outline.into()),
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    8.0,
                ),
                ChipVariant::Filter => {
                    if *activo {
                        crear_chip_button(
                            texto,
                            ColoresBoton {
                                fg: scheme.on_secondary_container.into(),
                                bg: Some(scheme.secondary_container.into()),
                                border: None,
                            },
                            etiqueta_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    } else {
                        crear_chip_button(
                            texto,
                            ColoresBoton {
                                fg: scheme.on_surface.into(),
                                bg: None,
                                border: Some(scheme.outline.into()),
                            },
                            etiqueta_style,
                            callback,
                            _prog,
                            8.0,
                        )
                    }
                }
                ChipVariant::Entrada => crear_chip_button(
                    texto,
                    ColoresBoton {
                        fg: scheme.on_secondary_container.into(),
                        bg: Some(scheme.secondary_container.into()),
                        border: None,
                    },
                    etiqueta_style,
                    callback,
                    _prog,
                    8.0,
                ),
            }
        }

        // ─── Inputs Material Design 3 ──────────────────────────────────

        // ─── MaterialTextField ─────────────────────────────────────────
        Layout::CampoTextoMaterial {
            variable,
            etiqueta,
            placeholder,
            variant,
            multiline: _,
            error,
            counter: _,
        } => crate::widgets::material_inputs::render_texto_campo(
            variable,
            etiqueta,
            placeholder,
            variant,
            error,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── MaterialPasswordField ─────────────────────────────────────
        Layout::CampoContrasenaMaterial {
            variable,
            etiqueta,
            visible: _,
        } => crate::widgets::material_inputs::render_contrasena_campo(
            variable, etiqueta, data, _prog, theme,
        ),

        // ─── MaterialNumberField ───────────────────────────────────────
        Layout::CampoNumeroMaterial {
            variable,
            etiqueta,
            min,
            max,
            decimales: _,
        } => crate::widgets::material_inputs::render_numero_campo(
            variable, etiqueta, min, max, data, _prog, theme,
        ),

        // ─── MaterialSearchField ───────────────────────────────────────
        Layout::CampoBusquedaMaterial {
            variable,
            placeholder,
        } => crate::widgets::material_inputs::render_buscar_campo(
            variable,
            placeholder,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialDropdown ──────────────────────────────────────────
        Layout::MaterialDropdown {
            opciones,
            seleccionada,
            placeholder,
        } => crate::widgets::material_inputs::render_dropdown(
            opciones,
            seleccionada,
            placeholder,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialSelect ────────────────────────────────────────────
        Layout::MaterialSelect {
            opciones,
            seleccionada,
            etiqueta,
        } => crate::widgets::material_inputs::render_select(
            opciones,
            seleccionada,
            etiqueta,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialAutocomplete ──────────────────────────────────────
        Layout::MaterialAutocomplete {
            opciones: _,
            variable,
        } => crate::widgets::material_inputs::render_autocomplete(variable, data, _prog, theme),

        // ─── MaterialRadioGroup ────────────────────────────────────────
        Layout::MaterialRadioGroup {
            nombre: _,
            opciones,
            seleccion,
            callback,
            direction,
        } => crate::widgets::material_inputs::render_radio_group(
            opciones, seleccion, callback, direction, data, _prog, theme,
        ),

        // ─── MaterialSwitch ────────────────────────────────────────────
        Layout::MaterialSwitch { etiqueta, variable } => {
            crate::widgets::material_inputs::render_switch(etiqueta, variable, data, _prog, theme)
        }

        // ─── MaterialSliderDiscrete ────────────────────────────────────
        Layout::MaterialSliderDiscrete {
            variable,
            min,
            max,
            steps: _,
        } => crate::widgets::material_inputs::render_slider_discrete(
            variable, min, max, data, _prog, theme,
        ),

        // ─── MaterialSliderRange ───────────────────────────────────────
        Layout::MaterialSliderRange {
            variable_inicio,
            variable_fin,
            min,
            max,
        } => crate::widgets::material_inputs::render_slider_range(
            variable_inicio,
            variable_fin,
            min,
            max,
            data,
            _prog,
            theme,
        ),

        // ─── MaterialChipGroup ─────────────────────────────────────────
        Layout::MaterialChipGroup {
            chips,
            seleccion,
            callback,
            multiple: _,
        } => crate::widgets::material_inputs::render_chip_group(
            chips, seleccion, callback, data, _prog, theme,
        ),

        // ─── MaterialDatePicker ────────────────────────────────────────
        Layout::MaterialDatePicker { variable } => {
            crate::pickers::date_picker_vista(variable, data, &theme.scheme, theme)
        }

        // ─── MaterialTimePicker ────────────────────────────────────────
        Layout::MaterialTimePicker { variable } => {
            crate::pickers::time_picker_vista(variable, data, &theme.scheme, theme)
        }

        // ─── ColorPicker ───────────────────────────────────────────────
        Layout::ColorPicker { variable } => {
            crate::pickers::color_picker_vista(variable, data, &theme.scheme, theme)
        }

        // AspectRatioBox: caja con relación de aspecto fija
        Layout::AspectRatioBox { hijo, ratio } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Calcular alto basado en el ancho disponible y el ratio
            let available_ancho = (data.ventana_ancho - 48.0).max(200.0);
            let height = available_ancho / ratio;
            Box::new(view::sized_box(inner).dims((Length::px(available_ancho), Length::px(height))))
        }

        // ─── Tarjetas, Listas y Tablas ──────────────────────────────
        Layout::MaterialCard {
            hijo,
            variant,
            on_click: _,
            seleccionado,
        } => crate::widgets::material_cards::render_card(
            hijo,
            variant,
            seleccionado,
            data,
            _prog,
            theme,
        ),

        Layout::ElementoListaMaterial {
            leading,
            titulo,
            subtitulo,
            trailing,
            on_click: _,
        } => crate::widgets::material_cards::render_lista_elemento(
            leading, titulo, subtitulo, trailing, data, _prog, theme,
        ),

        Layout::ListaMaterial { items, dividers } => {
            crate::widgets::material_cards::render_lista(items, dividers, data, _prog, theme)
        }

        Layout::ControlListaMaterial {
            items,
            control_tipo: _,
            variables: _,
        } => crate::widgets::material_cards::render_lista_control(items, data, _prog, theme),

        Layout::SeleccionListaMaterial {
            items,
            seleccion: _,
            callback: _,
            multiple: _,
        } => crate::widgets::material_cards::render_lista_selection(items, data, _prog, theme),

        // ─── DynamicList: renderiza items desde una variable ────────
        // 5.4: NOTA sobre virtualización: view::virtual_scroll() de xilem 0.4 requiere
        // un closure que retorne un WidgetView concreto (no type-erased). Dado que
        // render_dynamic_list usa expr_a_layout_item → Box<AnyWidgetView>, la virtualización
        // directa no es compatible sin un wrapper intermedio. Se documenta para futura
        // migración cuando xilem soporte virtual_scroll con type erasure.
        Layout::ListaDinamica {
            variable,
            elemento_fn,
        } => crate::widgets::material_cards::render_dynamic_lista(
            variable,
            elemento_fn,
            data,
            _prog,
            theme,
        ),

        Layout::TablaDatosMaterial {
            columnas,
            filas,
            ordenable,
            seleccionable: _,
            col_orden,
            orden_asc,
        } => crate::widgets::material_cards::render_data_tabla(
            columnas,
            filas,
            ordenable,
            col_orden,
            orden_asc,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        Layout::MaterialSurface { hijo, color_role } => {
            crate::widgets::material_cards::render_surface(hijo, color_role, data, _prog, theme)
        }

        Layout::MaterialScaffold {
            top,
            body,
            bottom,
            fab: _,
        } => crate::widgets::material_cards::render_scaffold(top, body, bottom, data, _prog, theme),

        // ═══════════════════════════════════════════════════════════════
        // FEEDBACK Y SUPERPOSICIONES (Material Design 3)
        // ═══════════════════════════════════════════════════════════════

        // ─── DialogOverlay ───────────────────────────────────────────
        // Diálogo superpuesto con overlay semitransparente + fade+scale animado
        Layout::DialogOverlay { dialog, visible } => {
            let show = data.leer(visible).to_string() == "true";
            // Detectar cambio y animar progreso
            data.anim_visibility_changed(visible, show);
            let progress = data.anim_progress(visible, 200.0);
            let eased = EASE_EMPHASIZED.aplicar(progress);

            if show || progress > 0.0 {
                let inner = layout_a_vista(dialog, data, _prog, theme);
                // Overlay con opacidad animada (0 → 0.32)
                let overlay_alpha = eased * 0.32;
                let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                // Diálogo con escala animada (0.95 → 1.0)
                let dialog_scale = 0.95 + eased * 0.05;

                Box::new(view::zstack((
                    view::sized_box(view::label(String::new()))
                        .background(Background::Color(overlay_color)),
                    view::transformed(view::sized_box(inner).width(Length::px(312.0)))
                        .scale_non_uniform(dialog_scale, dialog_scale),
                )))
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── DialogAlert ─────────────────────────────────────────────
        // Tarjeta de diálogo con título, mensaje y botones
        Layout::DialogAlert {
            titulo,
            mensaje,
            confirmar_texto,
            cancelar_texto,
            on_confirm,
            on_cancel,
        } => {
            let scheme = &theme.scheme;
            let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(0.32);
            let surface_bg: Color = scheme.surface.into();
            let fg: Color = scheme.on_surface.into();
            let fg_var: Color = scheme.on_surface_variant.into();
            let primary_color: Color = scheme.primary.into();
            let prog = _prog.to_vec();
            let cb_confirm = on_confirm.clone();
            let cb_cancel = on_cancel.clone();

            // Botón Confirmar
            let confirm_btn = {
                let cb = cb_confirm.clone();
                let p = prog.clone();
                view::button(
                    view::label(confirmar_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(primary_color),
                    move |data: &mut AppStateNativo| {
                        if !cb.is_empty() {
                            ejecutar_callback_y_actualizar(&cb, data, &p);
                        }
                    },
                )
            };

            // Botón Cancelar (solo si hay texto)
            let tiene_cancel = !cancelar_texto.is_empty();
            let cancel_btn = if tiene_cancel {
                let cb = cb_cancel.clone();
                let p = prog.clone();
                Some(view::button(
                    view::label(cancelar_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(primary_color),
                    move |data: &mut AppStateNativo| {
                        if !cb.is_empty() {
                            ejecutar_callback_y_actualizar(&cb, data, &p);
                        }
                    },
                ))
            } else {
                None
            };

            // Fila de botones
            let button_row: Box<AnyWidgetView<AppStateNativo>> = if tiene_cancel {
                Box::new(
                    view::flex(
                        Axis::Horizontal,
                        (
                            cancel_btn.unwrap(),
                            view::sized_box(view::label(String::new())).width(Length::px(8.0)),
                            confirm_btn,
                        ),
                    )
                    .main_axis_alignment(MainAxisAlignment::End),
                )
            } else {
                Box::new(
                    view::flex(Axis::Horizontal, (confirm_btn,))
                        .main_axis_alignment(MainAxisAlignment::End),
                )
            };

            // Título
            let title_etiqueta = {
                let title_style = obtener_texto_style(&theme.typography, "title_large");
                view::label(titulo.clone())
                    .text_size(title_style.font_tamano as f32)
                    .weight(FontWeight::BOLD)
                    .color(fg)
            };

            // Mensaje
            let mensaje_etiqueta = {
                let body_style = obtener_texto_style(&theme.typography, "body_medium");
                view::label(mensaje.clone())
                    .text_size(body_style.font_tamano as f32)
                    .color(fg_var)
            };

            // Card del diálogo
            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    mensaje_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(16.0)),
                    button_row,
                ),
            );

            // Card del diálogo con padding interior
            let dialog_inner = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                .padding(24.0.px());

            // Contenedor externo con ancho máximo
            let dialog_container = view::sized_box(dialog_inner).width(Length::px(312.0));

            // Overlay completo
            Box::new(view::zstack((
                view::sized_box(view::label(String::new()))
                    .background(Background::Color(overlay_color)),
                dialog_container,
            )))
        }

        // ─── DialogCustom ────────────────────────────────────────────
        // Diálogo personalizado con hijo arbitrario
        Layout::DialogCustom {
            titulo,
            hijo,
            on_cerrar: _,
        } => {
            let scheme = &theme.scheme;
            let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(0.32);
            let surface_bg: Color = scheme.surface.into();
            let fg: Color = scheme.on_surface.into();

            let title_style = obtener_texto_style(&theme.typography, "title_large");
            let title_etiqueta = view::label(titulo.clone())
                .text_size(title_style.font_tamano as f32)
                .weight(FontWeight::BOLD)
                .color(fg);

            let inner = layout_a_vista(hijo, data, _prog, theme);

            let dialog_card = view::flex(
                Axis::Vertical,
                (
                    title_etiqueta,
                    view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    inner,
                ),
            );

            let dialog_container = view::sized_box(dialog_card)
                .background(Background::Color(surface_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                .padding(24.0.px());

            Box::new(view::zstack((
                view::sized_box(view::label(String::new()))
                    .background(Background::Color(overlay_color)),
                dialog_container,
            )))
        }

        // ─── BottomSheet ─────────────────────────────────────────────
        // Panel que emerge desde abajo (Standard, Modal, Expanded)
        // Animación: slide-up con fade del overlay
        Layout::BottomSheet {
            hijo,
            variant,
            visible,
            on_dismiss: _,
        } => {
            let show = data.leer(visible).to_string() == "true";
            // Detectar cambio y animar
            data.anim_visibility_changed(visible, show);
            let progress = data.anim_progress(visible, 250.0);
            let eased = EASE_EMPHASIZED.aplicar(progress);

            if show || progress > 0.0 {
                let scheme = &theme.scheme;
                let inner = layout_a_vista(hijo, data, _prog, theme);
                let sheet_bg: Color = scheme.surface.into();
                // Slide-up: offset_y desde 300px hasta 0
                let sheet_offset_y = 300.0 * (1.0 - eased);

                match variant {
                    SheetVariant::Standard => {
                        // Sin overlay, solo panel con slide-up
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new())),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                    SheetVariant::Modal => {
                        // Overlay con fade + panel con slide-up
                        let overlay_alpha = eased * 0.32;
                        let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new()))
                                .background(Background::Color(overlay_color)),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                    SheetVariant::Expanded => {
                        // Overlay con fade + panel expandido con slide-up
                        let overlay_alpha = eased * 0.32;
                        let overlay_color: Color = RgbColor(0, 0, 0).con_alpha(overlay_alpha);
                        let sheet = view::transformed(
                            view::sized_box(inner)
                                .background(Background::Color(sheet_bg))
                                .corner_radius(
                                    obtener_forma_radius(&theme.shapes, "extra_large").px(),
                                ),
                        )
                        .translate((0.0, sheet_offset_y));
                        Box::new(view::zstack((
                            view::sized_box(view::label(String::new()))
                                .background(Background::Color(overlay_color)),
                            Box::new(sheet) as Box<AnyWidgetView<AppStateNativo>>,
                        )))
                    }
                }
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── Snackbar ────────────────────────────────────────────────
        // Barra inferior de notificación temporal
        Layout::Snackbar {
            mensaje,
            accion_texto,
            accion_callback,
            duracion: _,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show {
                let scheme = &theme.scheme;
                let bg: Color = scheme.inverse_surface.into();
                let fg: Color = scheme.inverse_on_surface.into();
                let action_fg: Color = scheme.inverse_primary.into();
                let prog = _prog.to_vec();

                // Texto del mensaje
                let body_style = obtener_texto_style(&theme.typography, "body_medium");
                let mensaje_etiqueta = view::label(mensaje.clone())
                    .text_size(body_style.font_tamano as f32)
                    .color(fg);

                // Botón de acción (opcional)
                let tiene_action = accion_texto.is_some() && accion_callback.is_some();
                let action_btn: Option<Box<AnyWidgetView<AppStateNativo>>> = if tiene_action {
                    let at = accion_texto.clone().unwrap_or_default();
                    let cb = accion_callback.clone().unwrap_or_default();
                    let p = prog.clone();
                    Some(Box::new(view::button(
                        view::label(at.clone())
                            .text_size(14.0)
                            .weight(FontWeight::MEDIUM)
                            .color(action_fg),
                        move |data: &mut AppStateNativo| {
                            if !cb.is_empty() {
                                ejecutar_callback_y_actualizar(&cb, data, &p);
                            }
                        },
                    )) as Box<AnyWidgetView<AppStateNativo>>)
                } else {
                    None
                };

                // Construir fila: mensaje + botón
                let mut row_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                row_children.push(Box::new(mensaje_etiqueta) as Box<AnyWidgetView<AppStateNativo>>);
                if let Some(btn) = action_btn {
                    row_children.push(Box::new(
                        view::sized_box(view::label(String::new())).width(Length::px(16.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                    row_children.push(btn);
                }

                let row = view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(8.0));

                let snackbar = view::sized_box(row)
                    .background(Background::Color(bg))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                    .padding(16.0.px());

                Box::new(snackbar)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── Tooltip ─────────────────────────────────────────────────
        // Pequeña etiqueta de ayuda contextual
        Layout::Tooltip { hijo, texto } => {
            let scheme = &theme.scheme;
            let tooltip_bg: Color = scheme.on_surface.into();
            let tooltip_fg: Color = scheme.surface.into();

            let inner = layout_a_vista(hijo, data, _prog, theme);
            let etiqueta_style = obtener_texto_style(&theme.typography, "label_small");
            let tooltip_etiqueta = view::label(texto.clone())
                .text_size(etiqueta_style.font_tamano as f32)
                .color(tooltip_fg);

            let tooltip_box = view::sized_box(tooltip_etiqueta)
                .background(Background::Color(tooltip_bg))
                .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                .padding(8.0.px());

            // Mostrar tooltip debajo del contenido
            Box::new(view::flex(Axis::Vertical, (inner, tooltip_box)).gap(Length::px(4.0)))
        }

        // ─── Menu ────────────────────────────────────────────────────
        // Menú desplegable con lista de opciones
        Layout::Menu {
            items,
            on_select,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show && !items.is_empty() {
                let scheme = &theme.scheme;
                let surface_bg: Color = scheme.surface.into();
                let fg: Color = scheme.on_surface.into();
                let prog = _prog.to_vec();
                let cb = on_select.clone();

                let mut menu_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let cb_inner = cb.clone();
                    let t = item.clone();
                    let p = prog.clone();
                    let elemento_etiqueta = view::label(t.clone()).text_size(14.0).color(fg);
                    let elemento_btn =
                        view::button(elemento_etiqueta, move |data: &mut AppStateNativo| {
                            if !cb_inner.is_empty() {
                                ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                            }
                        });
                    menu_items.push(Box::new(elemento_btn) as Box<AnyWidgetView<AppStateNativo>>);
                    if i < items.len() - 1 {
                        menu_items.push(Box::new(
                            view::sized_box(view::label(String::new())).height(Length::px(1.0)),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    }
                }

                let menu_box =
                    view::sized_box(view::flex(Axis::Vertical, (menu_items,)).gap(Length::px(0.0)))
                        .background(Background::Color(surface_bg))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());

                Box::new(menu_box)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── ContextMenu ─────────────────────────────────────────────
        // Menú contextual con lista de opciones
        Layout::ContextMenu {
            items,
            on_select,
            visible,
        } => {
            let show = data.leer(visible).to_string() == "true";
            if show && !items.is_empty() {
                let scheme = &theme.scheme;
                let surface_bg: Color = scheme.surface.into();
                let fg: Color = scheme.on_surface.into();
                let prog = _prog.to_vec();
                let cb = on_select.clone();

                let mut menu_items: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    let cb_inner = cb.clone();
                    let t = item.clone();
                    let p = prog.clone();
                    let elemento_etiqueta = view::label(t.clone()).text_size(14.0).color(fg);
                    let elemento_btn =
                        view::button(elemento_etiqueta, move |data: &mut AppStateNativo| {
                            if !cb_inner.is_empty() {
                                ejecutar_callback_y_actualizar(&cb_inner, data, &p);
                            }
                        });
                    menu_items.push(Box::new(elemento_btn) as Box<AnyWidgetView<AppStateNativo>>);
                    if i < items.len() - 1 {
                        menu_items.push(Box::new(
                            view::sized_box(view::label(String::new())).height(Length::px(1.0)),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    }
                }

                let menu_box =
                    view::sized_box(view::flex(Axis::Vertical, (menu_items,)).gap(Length::px(0.0)))
                        .background(Background::Color(surface_bg))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());

                Box::new(menu_box)
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ═══════════════════════════════════════════════════════════
        // Navegación Material You
        // ═══════════════════════════════════════════════════════════

        // ─── Navigator (navegación por pantallas) ────────────────────
        Layout::Navigator {
            screens,
            actual_variable,
            history_var: _,
            nav_tipo,
            anim,
            on_change,
        } => crate::widgets::navigation::render_navigator(
            screens,
            actual_variable,
            nav_tipo,
            anim,
            on_change,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── NavigationBar ───────────────────────────────────────────
        Layout::NavigationBar {
            items,
            seleccion,
            variable_seleccion,
            on_change,
        } => crate::widgets::navigation::render_navigation_bar(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            data,
            _prog,
            theme,
        ),

        // ─── NavigationRail ──────────────────────────────────────────
        Layout::NavigationRail {
            items,
            seleccion,
            variable_seleccion,
            on_change,
            extended,
        } => crate::widgets::navigation::render_navigation_rail(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            extended,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── NavigationDrawer ────────────────────────────────────────
        Layout::NavigationDrawer {
            items,
            seleccion,
            variable_seleccion,
            on_change,
            modal,
            visible,
        } => crate::widgets::navigation::render_navigation_drawer(
            items,
            seleccion,
            variable_seleccion.as_deref(),
            on_change,
            modal,
            visible,
            ContextoRender {
                data,
                prog: _prog,
                theme,
            },
        ),

        // ─── TopAppBar ───────────────────────────────────────────────
        Layout::TopAppBar {
            titulo,
            acciones,
            menu_visible: _,
            variant: _,
        } => crate::widgets::navigation::render_top_app_bar(titulo, acciones, data, _prog, theme),

        // ─── BottomAppBar ────────────────────────────────────────────
        Layout::BottomAppBar { acciones, fab } => {
            crate::widgets::navigation::render_bottom_app_bar(acciones, fab, data, _prog, theme)
        }

        // ─── Tabs ────────────────────────────────────────────────────
        Layout::Tabs {
            tabs,
            seleccion,
            on_change,
            scrollable: _,
        } => {
            crate::widgets::navigation::render_tabs(tabs, seleccion, on_change, data, _prog, theme)
        }

        // ─── SearchBar ───────────────────────────────────────────────
        Layout::BarraBusqueda {
            placeholder,
            on_buscar: _,
            variable,
        } => {
            crate::widgets::navigation::render_buscar_bar(placeholder, variable, data, _prog, theme)
        }

        // ─── SearchView ──────────────────────────────────────────────
        Layout::VistaBusqueda {
            query: _,
            resultados,
            visible,
        } => {
            crate::widgets::navigation::render_buscar_vista(resultados, visible, data, _prog, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // INDICADORES (Fase 7)
        // ═══════════════════════════════════════════════════════════════

        // ─── LinearProgress ──────────────────────────────────────────
        Layout::LinearProgress {
            variable,
            indeterminado,
        } => crate::widgets::indicators::render_linear_progress(
            variable,
            indeterminado,
            data,
            _prog,
            theme,
        ),

        // ─── LinearProgressValue: barra con valor directo (items de lista) ──
        Layout::ValorProgresoLineal(val) => {
            crate::widgets::indicators::render_linear_progress_valor(val, theme)
        }

        // ─── CircularProgress ────────────────────────────────────────
        Layout::CircularProgress {
            variable,
            tamano,
            indeterminado,
        } => crate::widgets::indicators::render_circular_progress(
            variable,
            tamano,
            indeterminado,
            data,
            _prog,
            theme,
        ),

        // ─── Badge ──────────────────────────────────────────────────
        Layout::Badge { hijo, valor, dot } => {
            crate::widgets::indicators::render_badge(hijo, valor, dot, data, _prog, theme)
        }

        // ─── Skeleton ───────────────────────────────────────────────
        Layout::Skeleton { ancho, alto, tipo } => {
            crate::widgets::indicators::render_skeleton(ancho, alto, tipo, theme)
        }

        // ─── EmptyState ─────────────────────────────────────────────
        Layout::EmptyState {
            icono,
            mensaje,
            accion_texto,
            accion_cb,
        } => crate::widgets::indicators::render_empty_state(
            icono,
            mensaje,
            accion_texto,
            accion_cb,
            data,
            _prog,
            theme,
        ),

        // ─── ErrorState ─────────────────────────────────────────────
        Layout::ErrorState { mensaje, on_retry } => {
            crate::widgets::indicators::render_error_state(mensaje, on_retry, data, _prog, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // AVATARES (Fase 7)
        // ═══════════════════════════════════════════════════════════════

        // ─── Avatar ─────────────────────────────────────────────────
        Layout::Avatar {
            texto,
            variant,
            tamaño,
        } => crate::widgets::indicators::render_avatar(texto, variant, tamaño, theme),

        // ─── AvatarGroup ────────────────────────────────────────────
        Layout::AvatarGroup { avatares, max } => {
            crate::widgets::indicators::render_avatar_group(avatares, max, theme)
        }

        // ═══════════════════════════════════════════════════════════════
        // MOTION (Fase 8)
        // ═══════════════════════════════════════════════════════════════

        // ─── FadeTransition ──────────────────────────────────────────
        // Fundido de entrada/salida con opacidad animada
        Layout::FadeTransition {
            hijo,
            visible,
            duracion,
        } => {
            let show = data.leer(visible).to_string() == "true";
            data.anim_visibility_changed(visible, show);
            let d = if *duracion > 0.0 { *duracion } else { 200.0 };
            let progress = data.anim_progress(visible, d);
            let eased = EASE_STANDARD.aplicar(progress);

            if show || progress > 0.0 {
                let inner = layout_a_vista(hijo, data, _prog, theme);
                // Fade mediante opacidad del overlay
                let overlay_alpha = 1.0 - eased;
                if overlay_alpha < 0.01 {
                    inner
                } else {
                    Box::new(view::sized_box(inner).background(Background::Color(
                        RgbColor(0, 0, 0).con_alpha(overlay_alpha),
                    )))
                }
            } else {
                Box::new(view::sized_box(view::label(String::new())))
            }
        }

        // ─── RippleEffect ───────────────────────────────────────────
        // Efecto de onda expansiva Material Design
        // NOTA: La integración completa requiere un View Xilem personalizado.
        // El RippleWidget de animated_widgets estará disponible para uso
        // directo con Pod cuando se requiera desde código Masonry nativo.
        Layout::RippleEffect { hijo, color: _ } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            Box::new(
                view::sized_box(inner)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_small").px()),
            )
        }

        // ─── PullToRefresh ────────────────────────────────────────────
        Layout::PullToRefresh {
            hijo,
            callback,
            refreshing,
        } => {
            let es_refreshing = data.leer(refreshing).to_bool();
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let ref_var = refreshing.clone();

            let inner = layout_a_vista(hijo, data, _prog, theme);

            if es_refreshing {
                // Si está refrescando, mostrar spinner real + texto
                let fg: Color = theme.scheme.on_surface.into();
                let refresh_row = view::flex(
                    Axis::Horizontal,
                    (vec![
                        Box::new(
                            view::sized_box(view::spinner())
                                .dims((Length::px(18.0), Length::px(18.0))),
                        ) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(view::label("Actualizando...").text_size(14.0).color(fg)),
                    ],),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0));
                let header = Box::new(view::sized_box(refresh_row).padding(8.0.px()))
                    as Box<AnyWidgetView<AppStateNativo>>;
                let column_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = vec![header, inner];
                Box::new(view::portal(
                    view::flex(Axis::Vertical, (column_children,)).gap(Length::px(4.0)),
                ))
            } else {
                // Envolver en PullToRefreshView con detección de gesto real
                let view = PullToRefreshVista::nuevo(inner, cb, ref_var, prog);
                Box::new(view)
            }
        }

        // ─── SwipeToDismiss ───────────────────────────────────────────
        Layout::SwipeToDismiss {
            hijo,
            on_dismiss,
            etiqueta,
            dismissed,
        } => {
            let es_dismissed = data.leer(dismissed).to_bool();
            let prog = _prog.to_vec();
            let cb = on_dismiss.clone();
            let dismiss_var = dismissed.clone();
            let action_texto = etiqueta.clone();

            if es_dismissed {
                // Ya descartado: mostrar opción de deshacer
                let scheme = &theme.scheme;
                let fg: Color = scheme.on_surface.into();
                let btn_fg: Color = scheme.primary.into();
                let undo_etiqueta = view::label("Elemento descartado").text_size(14.0).color(fg);
                let undo_btn = view::button(
                    view::label(action_texto.clone())
                        .text_size(14.0)
                        .weight(FontWeight::MEDIUM)
                        .color(btn_fg),
                    move |data: &mut AppStateNativo| {
                        data.escribir(&dismiss_var, ValorGUI::Booleano(false));
                    },
                );
                let row_children: Vec<Box<AnyWidgetView<AppStateNativo>>> = vec![
                    Box::new(undo_etiqueta) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(view::sized_box(view::label(String::new())).width(Length::px(8.0)))
                        as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(undo_btn) as Box<AnyWidgetView<AppStateNativo>>,
                ];
                Box::new(
                    view::sized_box(
                        view::flex(Axis::Horizontal, (row_children,)).gap(Length::px(4.0)),
                    )
                    .padding(12.0.px())
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px()),
                )
            } else {
                // Mostrar contenido envuelto en SwipeToDismissView con gesto real
                let inner = layout_a_vista(hijo, data, _prog, theme);
                let view = SwipeToDismissVista::nuevo(inner, cb, dismiss_var, action_texto, prog);
                Box::new(view)
            }
        }

        // ─── PinchZoom ──────────────────────────────────────────────
        Layout::PinchZoom {
            hijo,
            min_scale,
            max_scale,
        } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let view = PinchZoomVista::nuevo(inner, *min_scale, *max_scale);
            Box::new(view)
        }

        // ─── Rotate ─────────────────────────────────────────────────
        Layout::Rotate { hijo } => {
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let view = RotateVista::nuevo(inner);
            Box::new(view)
        }

        // ═══════════════════════════════════════════════════════════════
        // GRÁFICOS VECTORIALES (Fase 9) — Vello Scene
        // ═══════════════════════════════════════════════════════════════
        //
        // Reemplaza la emulación anterior (barras flex, puntos Unicode,
        // leyendas de texto) por dibujo vectorial real con Vello Scene.
        // Usa widgets Masonry personalizados con View wrappers Xilem.

        // ─── LineChart ────────────────────────────────────────────────
        Layout::GraficoLineas {
            datos,
            color,
            etiquetas,
        } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(LineaChartVista::<AppStateNativo>::nuevo(puntos))
        }

        // ─── BarChart ────────────────────────────────────────────────
        Layout::BarChart {
            datos,
            colores,
            etiquetas,
            apilado,
        } => {
            let colores_ciclo = [
                theme.scheme.primary,
                theme.scheme.secondary,
                theme.scheme.tertiary,
                theme.scheme.error,
                theme.scheme.primary_container,
                theme.scheme.secondary_container,
            ];
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let c_str = colores.get(i).cloned().unwrap_or_default();
                    let c = if c_str.is_empty() {
                        let rgb = colores_ciclo[i % colores_ciclo.len()];
                        Rgba::nuevo(rgb.0, rgb.1, rgb.2, 255)
                    } else {
                        color_role_to_rgba(&theme.scheme, &c_str)
                    };
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(BarChartVista::<AppStateNativo>::nuevo(puntos, *apilado))
        }

        // ─── PieChart / Donut ─────────────────────────────────────────
        Layout::PieChart {
            datos,
            etiquetas,
            donut,
        } => {
            let colores_pie = [
                theme.scheme.primary,
                theme.scheme.secondary,
                theme.scheme.tertiary,
                theme.scheme.error,
                theme.scheme.primary_container,
                theme.scheme.secondary_container,
                theme.scheme.tertiary_container,
                theme.scheme.error_container,
            ];
            let puntos: Vec<ChartDataPoint> = datos
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let rgb = colores_pie[i % colores_pie.len()];
                    let c = Rgba::nuevo(rgb.0, rgb.1, rgb.2, 255);
                    let etiqueta = etiquetas.get(i).cloned().unwrap_or_default();
                    ChartDataPoint::nuevo(&etiqueta, v, c)
                })
                .collect();
            Box::new(PieChartVista::<AppStateNativo>::nuevo(puntos, *donut, 0.5))
        }

        // ─── GaugeChart ──────────────────────────────────────────────
        Layout::GaugeChart {
            valor,
            min: _,
            max,
            color: _,
        } => {
            let threshold = *max * 0.75;
            Box::new(GaugeChartVista::<AppStateNativo>::nuevo(
                *valor, *max, threshold,
            ))
        }

        // ─── Sparkline ───────────────────────────────────────────────
        Layout::Sparkline { datos, color } => {
            let c = color_role_to_rgba(&theme.scheme, color);
            Box::new(SparklineVista::<AppStateNativo>::nuevo(datos.clone(), c))
        }

        // ═══════════════════════════════════════════════════════════════
        // AVANZADOS (Fase 9)
        // ═══════════════════════════════════════════════════════════════

        // ─── StarRating ──────────────────────────────────────────────
        Layout::StarRating {
            valor,
            variable,
            max,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let valor_actual = match variable.as_deref() {
                Some(nombre) => data.leer(nombre).to_f64().max(0.0) as usize,
                None => *valor,
            };
            let stars: Vec<Box<AnyWidgetView<AppStateNativo>>> = (0..*max)
                .map(|i| {
                    let llena = i < valor_actual;
                    let cb_name = cb.clone();
                    let p = prog.clone();
                    let star_color: Color = if llena {
                        scheme.primary.into()
                    } else {
                        scheme.on_surface_variant.into()
                    };
                    // La estrella se pinta como path: la fuente por defecto no
                    // trae los glifos ★/☆ y se veían cuadraditos.
                    let btn = view::button(
                        crate::star_widget::estrella::<AppStateNativo>(24.0, star_color, llena),
                        move |data: &mut AppStateNativo| {
                            data.escribir("rating", ValorGUI::Entero((i + 1) as i64));
                            if !cb_name.is_empty() {
                                ejecutar_callback_con_valor(
                                    &cb_name,
                                    Some(ValorGUI::Texto((i + 1).to_string())),
                                    data,
                                    &p,
                                );
                            }
                        },
                    );
                    Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            Box::new(view::flex(Axis::Horizontal, (stars,)).gap(Length::px(4.0)))
        }

        // ─── Stepper ─────────────────────────────────────────────────
        Layout::Stepper {
            pasos,
            actual,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let steps: Vec<Box<AnyWidgetView<AppStateNativo>>> = pasos
                .iter()
                .enumerate()
                .map(|(i, paso)| {
                    let active = i == *actual;
                    let completed = i < *actual;
                    let paso_valor = paso.clone();
                    let num_color: Color = if active {
                        scheme.on_primary.into()
                    } else if completed {
                        scheme.on_primary_container.into()
                    } else {
                        scheme.on_surface_variant.into()
                    };
                    let bg_color = if active {
                        scheme.primary.into()
                    } else if completed {
                        scheme.primary_container.into()
                    } else {
                        scheme.surface_variant.into()
                    };
                    let numero_texto = if completed {
                        "✓".to_string()
                    } else {
                        (i + 1).to_string()
                    };
                    let num: Box<AnyWidgetView<AppStateNativo>> = if completed {
                        Box::new(view::sized_box(crate::svg_icon::<AppStateNativo>(
                            "check",
                            16.0,
                            scheme.on_primary_container,
                            crate::IconStyle::Filled,
                        )))
                    } else {
                        Box::new(
                            view::label(numero_texto)
                                .text_size(14.0)
                                .weight(FontWeight::BOLD)
                                .color(num_color),
                        )
                    };
                    let circle = view::sized_box(num)
                        .dims((Length::px(32.0), Length::px(32.0)))
                        .background(Background::Color(bg_color))
                        .corner_radius(obtener_forma_radius(&theme.shapes, "small").px());
                    let etiqueta = view::label(paso.clone())
                        .text_size(12.0)
                        .color(scheme.on_surface.into());
                    let cb_name = cb.clone();
                    let p = prog.clone();
                    let btn = view::button(
                        view::flex(
                            Axis::Vertical,
                            (
                                Box::new(circle) as Box<AnyWidgetView<AppStateNativo>>,
                                Box::new(etiqueta) as Box<AnyWidgetView<AppStateNativo>>,
                            ),
                        )
                        .cross_axis_alignment(CrossAxisAlignment::Center)
                        .gap(Length::px(4.0)),
                        move |data: &mut AppStateNativo| {
                            data.escribir("step", ValorGUI::Entero(i as i64));
                            if !cb_name.is_empty() {
                                ejecutar_callback_con_valor(
                                    &cb_name,
                                    Some(ValorGUI::Texto(paso_valor.clone())),
                                    data,
                                    &p,
                                );
                            }
                        },
                    );
                    Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            Box::new(
                view::flex(Axis::Horizontal, (steps,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(16.0)),
            )
        }

        // ─── Breadcrumbs ─────────────────────────────────────────────
        Layout::Breadcrumbs { items, separador } => {
            let scheme = &theme.scheme;
            let crumbs: Vec<Box<AnyWidgetView<AppStateNativo>>> = items
                .iter()
                .enumerate()
                .flat_map(|(i, item)| {
                    let mut widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                    if i > 0 {
                        let sep = view::label(separador.clone())
                            .text_size(16.0)
                            .color(scheme.on_surface_variant.into());
                        widgets.push(Box::new(sep) as Box<AnyWidgetView<AppStateNativo>>);
                    }
                    let es_ultimo = i == items.len() - 1;
                    let lbl = view::label(item.clone())
                        .text_size(14.0)
                        .weight(if es_ultimo {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .color(if es_ultimo {
                            scheme.on_surface.into()
                        } else {
                            scheme.primary.into()
                        });
                    widgets.push(Box::new(lbl) as Box<AnyWidgetView<AppStateNativo>>);
                    widgets
                })
                .collect();
            Box::new(
                view::flex(Axis::Horizontal, (crumbs,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(4.0)),
            )
        }

        // ─── Calendar (con chrono) ──────────────────────────────────────
        Layout::Calendar {
            mes,
            año,
            seleccionado,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb_name = callback.clone();
            let mes_val = *mes;
            let año_val = *año;
            let fecha_sel = seleccionado.clone().unwrap_or_default();

            // Usar chrono para calcular datos reales del mes
            let primero_date = chrono::NaiveDate::from_ymd_opt(año_val, mes_val as u32, 1)
                .unwrap_or_else(|| {
                    let today = chrono::Local::now().naive_local().date();
                    chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
                });
            let days_in_month = if mes_val == 12 {
                chrono::NaiveDate::from_ymd_opt(año_val + 1, 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(31)
            } else {
                chrono::NaiveDate::from_ymd_opt(año_val, mes_val as u32 + 1, 1)
                    .and_then(|d| d.pred_opt())
                    .map(|d| d.day())
                    .unwrap_or(30)
            };
            let primero_weekday = primero_date.weekday().num_days_from_monday() as usize; // 0=Lun, 6=Dom
            let num_dias = days_in_month as usize;

            // Nombres de meses
            let nombres_mes = [
                "Enero",
                "Febrero",
                "Marzo",
                "Abril",
                "Mayo",
                "Junio",
                "Julio",
                "Agosto",
                "Septiembre",
                "Octubre",
                "Noviembre",
                "Diciembre",
            ];
            let mes_nombre = nombres_mes.get(mes_val as usize - 1).unwrap_or(&"");
            let title = view::label(format!("{} {}", mes_nombre, año_val))
                .text_size(16.0)
                .weight(FontWeight::BOLD)
                .color(scheme.on_surface.into());

            // Botones de navegación de meses
            let _anterior_cb = cb_name.clone();
            let _prog_anterior = prog.clone();
            let anterior_btn = view::button(
                view::label("◀")
                    .text_size(14.0)
                    .color(scheme.primary.into()),
                move |data: &mut AppStateNativo| {
                    // La navegación requiere reconstruir la vista; se maneja
                    // cambiando las variables de estado. Por ahora placeholder.
                    data.escribir("calendar_accion", ValorGUI::Texto("prev".to_string()));
                },
            );
            let _next_cb = cb_name.clone();
            let _prog_next = prog.clone();
            let next_btn = view::button(
                view::label("▶")
                    .text_size(14.0)
                    .color(scheme.primary.into()),
                move |data: &mut AppStateNativo| {
                    data.escribir("calendar_accion", ValorGUI::Texto("next".to_string()));
                },
            );
            let header_row = view::flex(
                Axis::Horizontal,
                (
                    Box::new(anterior_btn) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(title) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(next_btn) as Box<AnyWidgetView<AppStateNativo>>,
                ),
            )
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(Length::px(12.0));

            // Días de la semana header (L M M J V S D)
            let dias_semana = ["L", "M", "M", "J", "V", "S", "D"];
            let header_dias: Vec<Box<AnyWidgetView<AppStateNativo>>> = dias_semana
                .iter()
                .map(|d| {
                    Box::new(
                        view::sized_box(
                            view::label(d.to_string())
                                .text_size(11.0)
                                .weight(FontWeight::BOLD)
                                .color(scheme.on_surface_variant.into()),
                        )
                        .dims((Length::px(36.0), Length::px(36.0))),
                    ) as Box<AnyWidgetView<AppStateNativo>>
                })
                .collect();
            let day_header = view::flex(Axis::Horizontal, (header_dias,)).gap(Length::px(2.0));

            // Generar grilla de días con datos reales
            let total_cells = primero_weekday + num_dias;
            let num_rows = total_cells.div_ceil(7);
            let mut all_rows: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            all_rows.push(Box::new(header_row) as Box<AnyWidgetView<AppStateNativo>>);
            all_rows.push(Box::new(day_header) as Box<AnyWidgetView<AppStateNativo>>);

            for row in 0..num_rows {
                let mut day_cells: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
                for col in 0..7 {
                    let cell_idx = row * 7 + col;
                    if cell_idx < primero_weekday || cell_idx >= primero_weekday + num_dias {
                        day_cells.push(Box::new(
                            view::sized_box(view::label(String::new()))
                                .dims((Length::px(36.0), Length::px(36.0))),
                        )
                            as Box<AnyWidgetView<AppStateNativo>>);
                    } else {
                        let dia = (cell_idx - primero_weekday + 1) as u32;
                        let cb = cb_name.clone();
                        let p = prog.clone();
                        let fecha_str = format!("{:04}-{:02}-{:02}", año_val, mes_val, dia);
                        let es_selected = fecha_sel == fecha_str;

                        if es_selected {
                            let lbl = view::label(dia.to_string())
                                .text_size(12.0)
                                .weight(FontWeight::BOLD)
                                .color(scheme.on_surface.into());
                            day_cells.push(Box::new(
                                view::sized_box(lbl)
                                    .dims((Length::px(36.0), Length::px(36.0)))
                                    .background(Background::Color(scheme.primary_container.into()))
                                    .corner_radius(
                                        obtener_forma_radius(&theme.shapes, "full").px(),
                                    ),
                            )
                                as Box<AnyWidgetView<AppStateNativo>>);
                        } else {
                            let btn = view::button(
                                view::label(dia.to_string())
                                    .text_size(12.0)
                                    .color(scheme.on_surface.into()),
                                move |data: &mut AppStateNativo| {
                                    data.escribir("fecha", ValorGUI::Texto(fecha_str.clone()));
                                    if !cb.is_empty() {
                                        ejecutar_callback_con_valor(
                                            &cb,
                                            Some(ValorGUI::Texto(fecha_str.clone())),
                                            data,
                                            &p,
                                        );
                                    }
                                },
                            );
                            day_cells.push(Box::new(
                                view::sized_box(btn).dims((Length::px(36.0), Length::px(36.0))),
                            )
                                as Box<AnyWidgetView<AppStateNativo>>);
                        }
                    }
                }
                all_rows.push(Box::new(
                    view::flex(Axis::Horizontal, (day_cells,)).gap(Length::px(2.0)),
                ) as Box<AnyWidgetView<AppStateNativo>>);
            }

            Box::new(
                view::flex(Axis::Vertical, (all_rows,))
                    .cross_axis_alignment(CrossAxisAlignment::Center)
                    .gap(Length::px(4.0)),
            )
        }

        // ─── MarkdownViewer ──────────────────────────────────────────
        Layout::MarkdownViewer { texto } => {
            let scheme = &theme.scheme;
            // Renderizado simplificado: reemplaza # y ** para simular formato
            let lineas: Vec<&str> = texto.split('\n').collect();
            let mut children: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
            for line in lineas {
                let txt = line.to_string();
                if txt.starts_with("# ") {
                    let content = txt.trim_start_matches("# ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(24.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("## ") {
                    let content = txt.trim_start_matches("## ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(20.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("### ") {
                    let content = txt.trim_start_matches("### ");
                    children.push(Box::new(
                        view::label(content.to_string())
                            .text_size(16.0)
                            .weight(FontWeight::BOLD)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.starts_with("- ") || txt.starts_with("* ") {
                    let content = txt.trim_start_matches("- ").trim_start_matches("* ");
                    children.push(Box::new(
                        view::label(format!("  • {}", content))
                            .text_size(14.0)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else if txt.trim().is_empty() {
                    children.push(Box::new(
                        view::sized_box(view::label(String::new())).height(Length::px(8.0)),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                } else {
                    // Bold y cursiva simulados
                    let processed = txt.replace("**", "").replace("*", "").replace("__", "");
                    children.push(Box::new(
                        view::label(processed)
                            .text_size(14.0)
                            .color(scheme.on_surface.into()),
                    ) as Box<AnyWidgetView<AppStateNativo>>);
                }
            }
            Box::new(view::flex(Axis::Vertical, (children,)).gap(Length::px(4.0)))
        }

        // ─── QRCode (real) ───────────────────────────────────────────
        Layout::QRCode { texto, tamaño } => {
            let t = *tamaño;
            let content = if texto.len() > 30 {
                format!("{}…", &texto[..30])
            } else {
                texto.clone()
            };
            // Usar QRView/widget real con qrcode crate + Vello
            let qr = crate::qr_widget::qr_vista::<AppStateNativo>(texto, t);
            let lbl = view::label(content)
                .text_size(10.0)
                .color(theme.scheme.on_surface.into());
            Box::new(
                view::flex(
                    Axis::Vertical,
                    (
                        Box::new(qr) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(lbl) as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
            )
        }

        // ─── FilePicker (con rfd) ─────────────────────────────────────
        Layout::SelectorArchivo {
            tipos,
            multiple: _,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            // Clonar a owned: el closure del botón debe ser 'static y no puede
            // capturar la referencia 'a del layout.
            let tipos_filtro = tipos.clone();
            let btn = view::button(
                view::flex(
                    Axis::Horizontal,
                    (
                        Box::new(crate::svg_icon::<AppStateNativo>(
                            "folder",
                            18.0,
                            scheme.on_surface,
                            crate::IconStyle::Filled,
                        )) as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(
                            view::label("Seleccionar archivo")
                                .text_size(14.0)
                                .weight(FontWeight::MEDIUM),
                        ) as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
                move |data: &mut AppStateNativo| {
                    // Ruta elegida, o `None` si el usuario cancela el diálogo.
                    // Usar rfd para diálogo nativo de selección de archivos
                    #[cfg(not(target_os = "android"))]
                    let archivo_elegido: Option<String> = {
                        let mut dialog = rfd::FileDialog::new();
                        if !tipos_filtro.is_empty() {
                            dialog = dialog.add_filter(
                                "Archivos",
                                &tipos_filtro
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<&str>>(),
                            );
                        }
                        dialog
                            .set_directory(".")
                            .pick_file()
                            .map(|path| path.to_string_lossy().to_string())
                    };
                    #[cfg(target_os = "android")]
                    let archivo_elegido: Option<String> = {
                        // Sin diálogo nativo en Android: simulamos la selección.
                        // Si se configuraron filtros, usar el primero como extensión
                        // (uso legítimo de tipos_filtro en ambos cfg).
                        Some(if tipos_filtro.is_empty() {
                            "archivo_seleccionado.txt".to_string()
                        } else {
                            format!("archivo_seleccionado.{}", tipos_filtro[0].replace('.', ""))
                        })
                    };
                    if let Some(ruta) = archivo_elegido {
                        data.escribir("archivo", ValorGUI::Texto(ruta.clone()));
                        if !cb.is_empty() {
                            ejecutar_callback_con_valor(
                                &cb,
                                Some(ValorGUI::Texto(ruta)),
                                data,
                                &prog,
                            );
                        }
                    }
                },
            );
            Box::new(
                view::sized_box(btn)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "small").px())
                    .background(Background::Color(scheme.primary_container.into()))
                    .border_color(scheme.primary.into())
                    .border_width(1.0.px()),
            )
        }

        // ═══════════════════════════════════════════════════════════════
        // EXPRESSIVE (Fase 10)
        // ═══════════════════════════════════════════════════════════════

        // ─── GlassCard ───────────────────────────────────────────────
        Layout::GlassCard {
            hijo,
            blur: _,
            opacity,
        } => {
            let _scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Glassmorphism simulado con alpha blending
            // with_alpha() ya retorna xilem::Color (alias Color)
            let bg_color = RgbColor(255, 255, 255).con_alpha(*opacity);
            let border_color = RgbColor(255, 255, 255).con_alpha(0.3);
            Box::new(
                view::sized_box(inner)
                    .background(Background::Color(bg_color))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px())
                    .border_color(border_color)
                    .border_width(1.0.px()),
            )
        }

        // ─── GradientBox ─────────────────────────────────────────────
        Layout::GradientBox {
            hijo,
            colores,
            direccion,
        } => {
            let scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            // Gradiente real con Background::Gradient de Masonry
            let gradient = if colores.len() >= 2 {
                // Convertir colores a xilem::Color (AlphaColor<Srgb>)
                let color_vec: Vec<Color> = colores
                    .iter()
                    .map(|c| {
                        let rgb = parsear_color(c).unwrap_or(scheme.primary);
                        rgb.into()
                    })
                    .collect();
                // Crear color stops equiespaciados: [(0.0, c1), (1.0, c2), ...]
                let total = color_vec.len();
                let stops: Vec<(f32, Color)> = color_vec
                    .into_iter()
                    .enumerate()
                    .map(|(i, clr)| {
                        let offset = if total > 1 {
                            i as f32 / (total - 1) as f32
                        } else {
                            0.0
                        };
                        (offset, clr)
                    })
                    .collect();
                match direccion.to_lowercase().as_str() {
                    "radial" => {
                        xilem::style::Gradient::new_radial(crate::layout::UnitPoint::CENTER)
                            .with_stops(stops.as_slice())
                    }
                    "sweep" => xilem::style::Gradient::new_full_sweep(
                        crate::layout::UnitPoint::CENTER,
                        0.0,
                    )
                    .with_stops(stops.as_slice()),
                    _ => xilem::style::Gradient::new_linear(std::f64::consts::FRAC_PI_4)
                        .with_stops(stops.as_slice()),
                }
            } else if colores.len() == 1 {
                let c = parsear_color(&colores[0]).unwrap_or(scheme.primary);
                let color: Color = c.into();
                // Gradiente lineal simple con un solo color (fondo sólido)
                xilem::style::Gradient::new_linear(0.0)
                    .with_stops([(0.0, color), (1.0, color)].as_slice())
            } else {
                let c: Color = scheme.primary.into();
                xilem::style::Gradient::new_linear(0.0).with_stops([(0.0, c), (1.0, c)].as_slice())
            };
            Box::new(
                view::sized_box(inner)
                    .background(Background::Gradient(gradient))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "container").px()),
            )
        }

        // ─── MorphingButton ──────────────────────────────────────────
        Layout::MorphingButton {
            icono,
            texto_extendido,
            callback,
        } => {
            let scheme = &theme.scheme;
            let prog = _prog.to_vec();
            let cb = callback.clone();
            let icon = icono.clone();
            let txt = texto_extendido.clone();
            let btn = view::button(
                view::flex(
                    Axis::Horizontal,
                    (
                        Box::new(view::label(icon).text_size(20.0))
                            as Box<AnyWidgetView<AppStateNativo>>,
                        Box::new(view::label(txt).text_size(14.0).weight(FontWeight::MEDIUM))
                            as Box<AnyWidgetView<AppStateNativo>>,
                    ),
                )
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(Length::px(8.0)),
                move |data: &mut AppStateNativo| {
                    if !cb.is_empty() {
                        ejecutar_callback_y_actualizar(&cb, data, &prog);
                    }
                },
            );
            Box::new(
                view::sized_box(btn)
                    .corner_radius(obtener_forma_radius(&theme.shapes, "button").px())
                    .background(Background::Color(scheme.primary_container.into()))
                    .border_color(scheme.primary.into())
                    .border_width(2.0.px()),
            )
        }

        // ─── ExpressiveBackground ────────────────────────────────────
        Layout::ExpressiveBackground {
            colores,
            animado: _,
        } => {
            let scheme = &theme.scheme;
            let bg_color = if !colores.is_empty() {
                let c = parsear_color(&colores[0]).unwrap_or(scheme.primary);
                c.into()
            } else {
                scheme.primary_container.into()
            };
            // Layout de fondo expansivo con gradiente simulado (múltiples capas)
            let base = view::sized_box(view::label(String::new()))
                .dims((Length::px(300.0), Length::px(120.0)))
                .background(Background::Color(bg_color))
                .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px());
            if colores.len() > 1 {
                let c2 = parsear_color(&colores[1]).unwrap_or(scheme.secondary);
                let overlay = view::sized_box(view::label(String::new()))
                    .dims((Length::px(300.0), Length::px(60.0)))
                    .background(Background::Color(c2.into()))
                    .corner_radius(obtener_forma_radius(&theme.shapes, "extra_large").px());
                Box::new(view::zstack((
                    Box::new(base) as Box<AnyWidgetView<AppStateNativo>>,
                    Box::new(overlay) as Box<AnyWidgetView<AppStateNativo>>,
                )))
            } else {
                Box::new(base)
            }
        }

        // ─── GlowBorder ──────────────────────────────────────────────
        Layout::GlowBorder { hijo, color, ancho } => {
            let scheme = &theme.scheme;
            let inner = layout_a_vista(hijo, data, _prog, theme);
            let glow_color: Color = obtener_color_role(scheme, color).into();
            Box::new(
                view::sized_box(inner)
                    .border_color(glow_color)
                    .border_width((*ancho).px())
                    .corner_radius(obtener_forma_radius(&theme.shapes, "container").px()),
            )
        }

        // ─── Canvas / Lienzo de dibujo libre ───────────────────────────
        Layout::Canvas {
            commands_var,
            width,
            height,
        } => {
            // Leer comandos de la variable si existe
            let commands_json = data.leer(commands_var).to_string();
            let view = if commands_json != "null" && !commands_json.is_empty() {
                crate::canvas_widget::CanvasVista::<AppStateNativo>::nuevo(
                    commands_var.clone(),
                    commands_json,
                    *width,
                    *height,
                )
            } else {
                crate::canvas_widget::CanvasVista::<AppStateNativo>::empty(
                    commands_var.clone(),
                    *width,
                    *height,
                )
            };
            Box::new(view)
        }

        // ═══════════════════════════════════════════════════════════════════
        // ICONOS MATERIAL DESIGN — renderizado SVG con Vello
        // ═══════════════════════════════════════════════════════════════════

        // MaterialIconLayout: icono vectorial con tamaño, color y estilo
        // Renderiza paths SVG de Material Design directamente en el Scene de Vello,
        // usando el view wrapper SvgIconView.
        Layout::MaterialIconLayout {
            nombre,
            tamaño,
            color,
            estilo,
        } => {
            let scheme = &theme.scheme;
            // El color sale del tema activo: primero roles del esquema (primary,
            // on_surface_variant, ...), después hex o nombre de color estándar.
            // `parse_color` solo se usa como último recurso porque devuelve
            // valores fijos de Material You.
            let icon_color: RgbColor = color_de_rol(scheme, color)
                .or_else(|| parsear_color(color))
                .unwrap_or(scheme.primary);
            let icon_tamano = *tamaño;
            let icon_estilo = IconStyle::desde_str(estilo);

            // Si el nombre no está en el catálogo, mostrar el nombre en texto:
            // un icono inexistente tiene que verse, no desaparecer.
            if crate::icons::catalog::by_name(nombre).is_none() {
                return Box::new(
                    view::label(format!("[{}]", nombre))
                        .text_size((icon_tamano * 0.5) as f32)
                        .color(icon_color.into()),
                );
            }

            // Crear el view wrapper del icono SVG (renderiza paths reales de Vello)
            Box::new(svg_icon::<AppStateNativo>(
                nombre,
                icon_tamano,
                icon_color,
                icon_estilo,
            ))
        }
    }
}

