// Material Design 3 — Inputs: TextField, PasswordField, NumberField,
// SearchField, Dropdown, Select, Autocomplete, RadioGroup, Switch,
// SliderDiscrete, SliderRange, ChipGroup
// Funciones de rendering extraídas de gui_nativa.rs

use crate::gui_nativa::{AppStateNativo, TextFieldVariant};
use crate::helpers::*;
use crate::layout::AsUnit as _;
use crate::view;
use crate::Length;
use crate::{AnyWidgetView, Background, DesignStyle, FontWeight, MaterialTheme, ShapeFamily};
use forja_syntax::ast::Declaracion;
use xilem::style::Style;

// Superficie de un campo de entrada según el diseño del tema:
// - Fluent: fondo de superficie sólido con hairline de 1px (Windows 11).
// - Liquid Glass: fondo translúcido con borde claro.
// - Material You: relleno tonal sin borde.
fn superficie_campo(theme: &MaterialTheme) -> (crate::Color, crate::Color, f64) {
    let scheme = &theme.scheme;
    let borde: crate::Color = scheme.outline_variant.into();
    match theme.design {
        DesignStyle::LiquidGlass => (scheme.surface_variant.with_alpha(0.55), borde, 1.0),
        DesignStyle::Fluent => (scheme.surface.into(), borde, 1.0),
        DesignStyle::MaterialYou => (scheme.surface_variant.into(), borde, 0.0),
    }
}

// ─── MaterialTextField ───────────────────────────────────────────

pub(crate) fn render_text_field(
    variable: &str,
    label: &str,
    placeholder: &str,
    variant: &TextFieldVariant,
    error: &str,
    ctx: ContextoRender<'_>,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let ContextoRender {
        data,
        prog: _prog,
        theme,
    } = ctx;
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let label_text = label.to_string();
    let placeholder_text = placeholder.to_string();
    let err_text = error.to_string();

    // Label flotante
    let label_color: crate::Color = if !err_text.is_empty() {
        scheme.error.into()
    } else {
        scheme.on_surface_variant.into()
    };
    let label_widget = if label_text.is_empty() {
        None
    } else {
        Some(
            view::label(label_text.clone())
                .text_size(12.0)
                .color(label_color),
        )
    };

    // Campo de texto
    let mut ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .text_color(scheme.on_surface.into());
    if !placeholder_text.is_empty() {
        ti = ti.placeholder(placeholder_text.as_str());
    }

    // Aplicar colores según variante
    let input_widget = match variant {
        TextFieldVariant::Filled => {
            let (bg, borde, ancho) = superficie_campo(theme);
            Box::new(
                view::sized_box(ti)
                    .background(Background::Color(bg))
                    .border_color(borde)
                    .border_width(ancho.px())
                    .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
            ) as Box<AnyWidgetView<AppStateNativo>>
        }
        TextFieldVariant::Outlined => {
            let border: crate::Color = if !err_text.is_empty() {
                scheme.error.into()
            } else {
                scheme.outline.into()
            };
            let (bg, _, _) = superficie_campo(theme);
            if matches!(theme.design, DesignStyle::LiquidGlass) {
                Box::new(
                    view::sized_box(ti)
                        .border_color(border)
                        .border_width(1.0.px())
                        .background(Background::Color(bg))
                        .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
                ) as Box<AnyWidgetView<AppStateNativo>>
            } else {
                Box::new(
                    view::sized_box(ti)
                        .border_color(border)
                        .border_width(1.0.px())
                        .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
                ) as Box<AnyWidgetView<AppStateNativo>>
            }
        }
    };

    // Error text
    let children: Vec<Box<AnyWidgetView<AppStateNativo>>> = if !err_text.is_empty() {
        let err_color: crate::Color = scheme.error.into();
        let err_label = view::label(err_text.clone())
            .text_size(11.0)
            .color(err_color);
        match label_widget {
            Some(lw) => vec![
                Box::new(lw) as Box<AnyWidgetView<AppStateNativo>>,
                input_widget,
                Box::new(err_label) as Box<AnyWidgetView<AppStateNativo>>,
            ],
            None => vec![
                input_widget,
                Box::new(err_label) as Box<AnyWidgetView<AppStateNativo>>,
            ],
        }
    } else {
        match label_widget {
            Some(lw) => vec![
                Box::new(lw) as Box<AnyWidgetView<AppStateNativo>>,
                input_widget,
            ],
            None => vec![input_widget],
        }
    };

    Box::new(
        view::flex(crate::Axis::Vertical, (children,))
            .cross_axis_alignment(crate::CrossAxisAlignment::Start)
            .gap(Length::px(4.0)),
    )
}

// ─── MaterialPasswordField ───────────────────────────────────────

pub(crate) fn render_password_field(
    variable: &str,
    label: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = _data.leer(variable).to_string();
    let label_text = label.to_string();

    let label_color: crate::Color = scheme.on_surface_variant.into();
    let bg: crate::Color = scheme.surface_variant.into();

    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder("••••••••")
    .text_color(scheme.on_surface.into());

    let input_widget = Box::new(
        view::sized_box(ti)
            .background(Background::Color(bg))
            .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
    ) as Box<AnyWidgetView<AppStateNativo>>;

    let children: Vec<Box<AnyWidgetView<AppStateNativo>>> = if label_text.is_empty() {
        vec![input_widget]
    } else {
        vec![
            Box::new(
                view::label(label_text.clone())
                    .text_size(12.0)
                    .color(label_color),
            ) as Box<AnyWidgetView<AppStateNativo>>,
            input_widget,
        ]
    };

    Box::new(
        view::flex(crate::Axis::Vertical, (children,))
            .cross_axis_alignment(crate::CrossAxisAlignment::Start)
            .gap(Length::px(4.0)),
    )
}

// ─── MaterialNumberField ─────────────────────────────────────────

pub(crate) fn render_number_field(
    variable: &str,
    label: &str,
    min: &f64,
    max: &f64,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = _data.leer(variable).to_string();
    let label_text = label.to_string();
    let mn = *min;
    let mx = *max;

    let label_color: crate::Color = scheme.on_surface_variant.into();
    let border: crate::Color = scheme.outline.into();

    let range_text = format!("{}-{}", mn, mx);
    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        if new_val.parse::<f64>().is_ok() || new_val.is_empty() || new_val == "-" || new_val == "."
        {
            data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
        }
    })
    .placeholder(range_text.as_str());

    let input_widget: Box<AnyWidgetView<AppStateNativo>> = if label_text.is_empty() {
        Box::new(
            view::sized_box(ti)
                .border_color(border)
                .border_width(1.0.px())
                .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
        )
    } else {
        Box::new(
            view::flex(
                crate::Axis::Vertical,
                (
                    view::label(label_text.clone())
                        .text_size(12.0)
                        .color(label_color),
                    view::sized_box(ti)
                        .border_color(border)
                        .border_width(1.0.px())
                        .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
                ),
            )
            .gap(Length::px(4.0)),
        )
    };

    input_widget
}

// ─── MaterialSearchField ─────────────────────────────────────────

pub(crate) fn render_search_field(
    variable: &str,
    placeholder: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let ph = placeholder.to_string();

    let bg: crate::Color = scheme.surface_variant.into();
    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder(ph.as_str());

    Box::new(
        view::flex(
            crate::Axis::Horizontal,
            (
                crate::svg_icon::<AppStateNativo>(
                    "search",
                    16.0,
                    scheme.on_surface_variant,
                    crate::IconStyle::Filled,
                ),
                view::sized_box(ti)
                    .background(Background::Color(bg))
                    .corner_radius(theme.shapes.for_family(ShapeFamily::Button).px()),
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialDropdown ────────────────────────────────────────────

pub(crate) fn render_dropdown(
    opciones: &[String],
    seleccionada: &usize,
    placeholder: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccionada;
    let ph = placeholder.to_string();

    let display_text = opts.get(sel).cloned().unwrap_or(ph);
    let fg: crate::Color = scheme.on_surface.into();
    let border: crate::Color = scheme.outline.into();
    let bg: crate::Color = scheme.surface_variant.into();

    let cb_btn = move |_data: &mut AppStateNativo| {
        // No hacemos nada en el placeholder de dropdown (ciclo)
    };

    Box::new(
        view::button(view::label(display_text).text_size(14.0).color(fg), cb_btn)
            .background(Background::Color(bg))
            .border_color(border)
            .border_width(1.0.px())
            .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
    )
}

// ─── MaterialSelect ──────────────────────────────────────────────

pub(crate) fn render_select(
    opciones: &[String],
    seleccionada: &usize,
    label: &str,
    _data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccionada;
    let label_text = label.to_string();

    let display_text = opts
        .get(sel)
        .cloned()
        .unwrap_or_else(|| "Seleccionar...".to_string());
    let fg: crate::Color = scheme.on_surface.into();
    let border: crate::Color = scheme.outline.into();
    let label_color: crate::Color = scheme.on_surface_variant.into();

    Box::new(
        view::flex(
            crate::Axis::Vertical,
            (
                view::label(label_text.clone())
                    .text_size(12.0)
                    .color(label_color),
                view::button(
                    view::label(display_text).text_size(14.0).color(fg),
                    move |data: &mut AppStateNativo| {
                        let _ = data;
                    },
                )
                .border_color(border)
                .border_width(1.0.px())
                .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialAutocomplete ────────────────────────────────────────

pub(crate) fn render_autocomplete(
    variable: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_string();
    let border: crate::Color = scheme.outline.into();

    let ti = view::text_input(val, move |data: &mut AppStateNativo, new_val: String| {
        data.escribir(&var_name, crate::ValorGUI::Texto(new_val));
    })
    .placeholder("Escribir...");

    Box::new(
        view::sized_box(ti)
            .border_color(border)
            .border_width(1.0.px())
            .corner_radius(theme.shapes.for_family(ShapeFamily::TextField).px()),
    )
}

// ─── MaterialRadioGroup ──────────────────────────────────────────

pub(crate) fn render_radio_group(
    opciones: &[String],
    seleccion: &usize,
    callback: &str,
    direction: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let opts: Vec<String> = opciones.to_vec();
    let sel = *seleccion;

    let mut radios: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, opcion) in opts.iter().enumerate() {
        let cb_inner = cb.clone();
        let t = opcion.clone();
        let prog_inner = prog.clone();
        let is_selected = i == sel;

        let fg: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.on_surface_variant.into()
        };
        let radio_color: crate::Color = if is_selected {
            scheme.primary.into()
        } else {
            scheme.outline.into()
        };

        let radio_widget = view::flex(
            crate::Axis::Horizontal,
            (
                view::sized_box(
                    view::label(if is_selected { "◉" } else { "○" }.to_string())
                        .text_size(20.0)
                        .color(radio_color),
                )
                .dims((Length::px(24.0), Length::px(24.0))),
                view::label(t.clone()).text_size(14.0).color(fg),
            ),
        )
        .gap(Length::px(8.0));

        let btn = view::button(radio_widget, move |data: &mut AppStateNativo| {
            ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
        });

        radios.push(Box::new(btn) as Box<AnyWidgetView<AppStateNativo>>);
    }

    let ax = if direction == "horizontal" {
        crate::Axis::Horizontal
    } else {
        crate::Axis::Vertical
    };
    Box::new(view::flex(ax, (radios,)).gap(Length::px(8.0)))
}

// ─── MaterialSwitch ──────────────────────────────────────────────

pub(crate) fn render_switch(
    label: &str,
    variable: &str,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let lbl = label.to_string();
    let checked = data.leer(variable).to_bool();

    // Switch nativo de Masonry: toma sus colores del property set del tema
    // (riel, borde y thumb). Antes esto era un checkbox metido en una caja
    // pintada con el color primario, que se estiraba a todo el ancho.
    let interruptor = view::switch(checked, move |data: &mut AppStateNativo, on: bool| {
        data.escribir(&var_name, crate::ValorGUI::Booleano(on));
    });
    let etiqueta: crate::Color = scheme.on_surface.into();
    Box::new(
        view::flex(
            crate::Axis::Horizontal,
            (
                Box::new(interruptor) as Box<AnyWidgetView<AppStateNativo>>,
                Box::new(view::label(lbl).text_size(14.0).color(etiqueta))
                    as Box<AnyWidgetView<AppStateNativo>>,
            ),
        )
        .main_axis_alignment(crate::MainAxisAlignment::Start)
        .cross_axis_alignment(crate::CrossAxisAlignment::Center)
        .gap(Length::px(10.0)),
    )
}

// ─── MaterialSliderDiscrete ──────────────────────────────────────

pub(crate) fn render_slider_discrete(
    variable: &str,
    min: &f64,
    max: &f64,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var_name = variable.to_string();
    let val = data.leer(variable).to_f64();
    let mn = *min;
    let mx = *max;

    let slider = view::slider(
        mn,
        mx,
        val,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var_name, crate::ValorGUI::Decimal(new_val));
        },
    );

    let display_val = format!("{:.1}", val);
    let fg: crate::Color = scheme.on_surface.into();

    Box::new(
        view::flex(
            crate::Axis::Vertical,
            (view::label(display_val).text_size(12.0).color(fg), slider),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialSliderRange ─────────────────────────────────────────

pub(crate) fn render_slider_range(
    variable_inicio: &str,
    variable_fin: &str,
    min: &f64,
    max: &f64,
    data: &mut AppStateNativo,
    _prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let var1 = variable_inicio.to_string();
    let var2 = variable_fin.to_string();
    let val1 = data.leer(variable_inicio).to_f64();
    let val2 = data.leer(variable_fin).to_f64();
    let mn = *min;
    let mx = *max;

    let slider1 = view::slider(
        mn,
        mx,
        val1,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var1, crate::ValorGUI::Decimal(new_val));
        },
    );
    let slider2 = view::slider(
        mn,
        mx,
        val2,
        move |data: &mut AppStateNativo, new_val: f64| {
            data.escribir(&var2, crate::ValorGUI::Decimal(new_val));
        },
    );

    let fg: crate::Color = scheme.on_surface.into();

    Box::new(
        view::flex(
            crate::Axis::Vertical,
            (
                view::label(format!("Inicio: {:.1}", val1))
                    .text_size(11.0)
                    .color(fg),
                slider1,
                view::label(format!("Fin: {:.1}", val2))
                    .text_size(11.0)
                    .color(fg),
                slider2,
            ),
        )
        .gap(Length::px(4.0)),
    )
}

// ─── MaterialChipGroup ───────────────────────────────────────────

pub(crate) fn render_chip_group(
    chips: &[String],
    seleccion: &[bool],
    callback: &str,
    _data: &mut AppStateNativo,
    prog: &[Declaracion],
    theme: &MaterialTheme,
) -> Box<AnyWidgetView<AppStateNativo>> {
    let scheme = &theme.scheme;
    let cb = callback.to_string();
    let prog = prog.to_vec();
    let chip_texts: Vec<String> = chips.to_vec();
    let sels: Vec<bool> = seleccion.to_vec();

    let mut chip_widgets: Vec<Box<AnyWidgetView<AppStateNativo>>> = Vec::new();
    for (i, chip_text) in chip_texts.iter().enumerate() {
        let cb_inner = cb.clone();
        let t = chip_text.clone();
        let prog_inner = prog.clone();
        let is_selected = sels.get(i).copied().unwrap_or(false);

        if is_selected {
            let fg: crate::Color = scheme.on_secondary_container.into();
            let bg: crate::Color = scheme.secondary_container.into();
            let label = view::label(t.clone())
                .text_size(12.0)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            chip_widgets.push(Box::new(
                btn.background(Background::Color(bg))
                    .corner_radius(theme.shapes.small.px()),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        } else {
            let fg: crate::Color = scheme.on_surface.into();
            let border: crate::Color = scheme.outline.into();
            let label = view::label(t.clone())
                .text_size(12.0)
                .weight(FontWeight::MEDIUM)
                .color(fg);
            let btn = view::button(label, move |data: &mut AppStateNativo| {
                ejecutar_callback_y_actualizar(&cb_inner, data, &prog_inner);
            });
            chip_widgets.push(Box::new(
                btn.border_color(border)
                    .border_width(1.0.px())
                    .corner_radius(theme.shapes.small.px()),
            ) as Box<AnyWidgetView<AppStateNativo>>);
        }
    }

    Box::new(view::flex(crate::Axis::Horizontal, (chip_widgets,)).gap(Length::px(8.0)))
}
